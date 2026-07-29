//! 导出命令实现（CSV / JSON）
//!
//! 路径安全验证 + I/O 在命令层，数据变换委托给 ExportUseCase。

use la_core::error::CommandError;
use la_core::models::LogEntry;
use tauri::{command, AppHandle, Manager};

use crate::application::{transform_csv, transform_json};

fn resolve_export_path(
    download_dir: &std::path::Path,
    requested_path: &str,
) -> Result<std::path::PathBuf, CommandError> {
    let safe = crate::utils::validation::prevent_path_traversal(requested_path)
        .map_err(|e| CommandError::new("EXPORT_PATH_UNSAFE", format!("导出路径不安全: {e}")))?;
    let save_path = std::path::Path::new(&safe);
    for component in save_path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(CommandError::new(
                    "ABSOLUTE_PATH_REJECTED",
                    "导出路径不允许使用绝对路径".to_string(),
                ));
            }
            std::path::Component::ParentDir => {
                return Err(CommandError::new(
                    "PATH_TRAVERSAL_DETECTED",
                    "导出路径包含非法路径遍历".to_string(),
                ));
            }
        }
    }

    let final_path = download_dir.join(&safe);
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

    if !canonical_parent.starts_with(&canonical_download) {
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

fn write_export_bytes(destination: &std::path::Path, contents: &[u8]) -> Result<(), CommandError> {
    use std::io::Write;

    let parent = destination
        .parent()
        .ok_or_else(|| CommandError::new("EXPORT_PATH_UNSAFE", "导出路径缺少父目录".to_string()))?;
    let file_name = destination
        .file_name()
        .ok_or_else(|| CommandError::new("EXPORT_PATH_UNSAFE", "导出路径缺少文件名".to_string()))?;
    let temporary_path = parent.join(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));

    let write_result = (|| -> std::io::Result<()> {
        let mut temporary = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)?;
        temporary.write_all(contents)?;
        temporary.sync_all()?;
        drop(temporary);

        if let Err(first_error) = std::fs::rename(&temporary_path, destination) {
            if std::fs::symlink_metadata(destination).is_err() {
                return Err(first_error);
            }

            std::fs::remove_file(destination)?;
            std::fs::rename(&temporary_path, destination)?;
        }

        Ok(())
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }

    write_result.map_err(|e| CommandError::new("IO_ERROR", e.to_string()))
}

#[command]
pub async fn export_results(
    app: AppHandle,
    results: Vec<LogEntry>,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, CommandError> {
    let download_dir = app.path().download_dir().map_err(|e| {
        CommandError::new("DOWNLOAD_DIR_UNAVAILABLE", format!("无法获取下载目录: {e}"))
    })?;
    let final_path = resolve_export_path(&download_dir, &savePath)?;
    let path_str = final_path.to_string_lossy().to_string();

    tokio::task::spawn_blocking(move || -> Result<String, CommandError> {
        match format.as_str() {
            "csv" => {
                let csv = transform_csv(&results);
                let mut bytes = Vec::with_capacity(3 + csv.len());
                bytes.extend_from_slice(&[0xEFu8, 0xBB, 0xBF]);
                bytes.extend_from_slice(csv.as_bytes());
                write_export_bytes(std::path::Path::new(&path_str), &bytes)?;
                Ok(path_str)
            }
            "json" => {
                let json = transform_json(&results);
                write_export_bytes(std::path::Path::new(&path_str), json.as_bytes())?;
                Ok(path_str)
            }
            _ => Err(CommandError::new(
                "UNSUPPORTED_EXPORT_FORMAT",
                format!("Unsupported format: {format}"),
            )),
        }
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

        write_export_bytes(&destination, b"new export").unwrap();

        assert_eq!(std::fs::read(&outside).unwrap(), b"do not overwrite");
        assert_eq!(std::fs::read(&destination).unwrap(), b"new export");
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
}
