//! 导出命令实现（CSV / JSON）
//!
//! 命令层只负责路径边界与运行时依赖，分页和文件发布委托给 ExportUseCase。

use la_core::error::CommandError;
use tauri::{command, AppHandle, Manager, State};

use crate::application::export_search_results;
use crate::models::AppState;

fn resolve_export_path(
    download_dir: &std::path::Path,
    requested_path: &str,
) -> Result<std::path::PathBuf, CommandError> {
    let validated = crate::utils::validation::prevent_path_traversal(requested_path)
        .map_err(|e| CommandError::new("EXPORT_PATH_UNSAFE", format!("导出路径不安全: {e}")))?;
    let save_path = std::path::Path::new(requested_path);
    for component in save_path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {}
            std::path::Component::ParentDir => {
                return Err(CommandError::new(
                    "PATH_TRAVERSAL_DETECTED",
                    "导出路径包含非法路径遍历".to_string(),
                ));
            }
        }
    }

    if std::path::Path::new(&validated).is_absolute() && !save_path.is_absolute() {
        return Err(CommandError::new(
            "ENCODED_ABSOLUTE_PATH_REJECTED",
            "导出路径不能使用编码后的绝对路径".to_string(),
        ));
    }

    let final_path = if save_path.is_absolute() {
        save_path.to_path_buf()
    } else {
        download_dir.join(save_path)
    };
    let canonical_download = dunce::canonicalize(download_dir).map_err(|e| {
        CommandError::new("DOWNLOAD_DIR_UNAVAILABLE", format!("无法解析下载目录: {e}"))
    })?;
    let parent = final_path
        .parent()
        .ok_or_else(|| CommandError::new("EXPORT_PATH_UNSAFE", "导出路径缺少父目录".to_string()))?;
    let canonical_parent = dunce::canonicalize(parent).map_err(|e| {
        CommandError::new(
            "EXPORT_PARENT_UNAVAILABLE",
            format!("导出目标目录不存在或无法访问: {e}"),
        )
    })?;

    if !save_path.is_absolute() && !canonical_parent.starts_with(&canonical_download) {
        return Err(CommandError::new(
            "EXPORT_PATH_ESCAPE",
            "导出路径不允许超出下载目录范围".to_string(),
        ));
    }

    let file_name = final_path
        .file_name()
        .ok_or_else(|| CommandError::new("EXPORT_PATH_UNSAFE", "导出路径缺少文件名".to_string()))?;
    let resolved_path = canonical_parent.join(file_name);
    if std::fs::symlink_metadata(&resolved_path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(CommandError::new(
            "EXPORT_PATH_SYMLINK",
            "导出目标不能是符号链接".to_string(),
        ));
    }

    Ok(resolved_path)
}

#[command]
pub async fn export_results(
    app: AppHandle,
    state: State<'_, AppState>,
    #[allow(non_snake_case)] searchId: String,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, CommandError> {
    let download_dir = app.path().download_dir().map_err(|e| {
        CommandError::new("DOWNLOAD_DIR_UNAVAILABLE", format!("无法获取下载目录: {e}"))
    })?;
    let final_path = resolve_export_path(&download_dir, &savePath)?;
    let path_str = final_path.to_string_lossy().to_string();
    let manager = state
        .get_search_session_manager()
        .ok_or_else(|| CommandError::new("NOT_FOUND", "Search session manager not initialized"))?;

    tokio::task::spawn_blocking(move || -> Result<String, CommandError> {
        export_search_results(std::path::Path::new(&path_str), &format, |offset, limit| {
            manager
                .fetch_search_page(&searchId, offset, limit)
                .map_err(CommandError::from)
        })?;
        Ok(path_str)
    })
    .await
    .map_err(|e| CommandError::new("EXPORT_PANICKED", format!("Export panicked: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn rejects_a_nonexistent_file_beneath_an_escaping_symlink() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new().unwrap();
        let downloads = root.path().join("downloads");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, downloads.join("link")).unwrap();

        let result = resolve_export_path(&downloads, "link/new.json");

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_an_existing_symlink_as_the_export_target() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new().unwrap();
        let downloads = root.path().join("downloads");
        let outside = root.path().join("outside.json");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::write(&outside, b"do not overwrite").unwrap();
        symlink(&outside, downloads.join("results.json")).unwrap();

        let result = resolve_export_path(&downloads, "results.json");

        assert!(result.is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), b"do not overwrite");
    }

    #[test]
    fn rejects_an_encoded_absolute_export_path() {
        let root = TempDir::new().unwrap();
        let downloads = root.path().join("downloads");
        std::fs::create_dir_all(&downloads).unwrap();

        let result = resolve_export_path(&downloads, "%2Ftmp%2Foutside.json");

        assert!(result.is_err());
    }

    #[test]
    fn export_does_not_modify_an_external_file_through_a_hard_link() {
        let root = TempDir::new().unwrap();
        let downloads = root.path().join("downloads");
        let outside = root.path().join("outside.json");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::write(&outside, b"do not overwrite").unwrap();
        let destination = downloads.join("results.json");
        std::fs::hard_link(&outside, &destination).unwrap();

        let requested = destination.to_string_lossy();
        let resolved = resolve_export_path(&downloads, &requested).unwrap();
        export_search_results(&resolved, "json", |_, _| {
            Ok(la_search::SearchPageResult {
                entries: vec![],
                total_count: 0,
                is_complete: true,
                has_more: false,
                next_offset: None,
            })
        })
        .unwrap();

        assert_eq!(std::fs::read(&outside).unwrap(), b"do not overwrite");
        let exported: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
        assert_eq!(exported["metadata"]["totalCount"], 0);
    }

    #[test]
    fn resolves_a_normal_export_inside_the_download_directory() {
        let root = TempDir::new().unwrap();
        let downloads = root.path().join("downloads");
        std::fs::create_dir_all(&downloads).unwrap();

        let result = resolve_export_path(&downloads, "results.json").unwrap();

        assert_eq!(
            result,
            dunce::canonicalize(downloads).unwrap().join("results.json")
        );
    }

    #[test]
    fn resolves_an_absolute_path_selected_by_the_save_dialog() {
        let root = TempDir::new().unwrap();
        let downloads = root.path().join("downloads");
        let selected_dir = root.path().join("selected");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::create_dir_all(&selected_dir).unwrap();
        let selected = selected_dir.join("results.json");

        let result = resolve_export_path(&downloads, &selected.to_string_lossy()).unwrap();

        assert_eq!(
            result,
            dunce::canonicalize(selected_dir)
                .unwrap()
                .join("results.json")
        );
    }
}
