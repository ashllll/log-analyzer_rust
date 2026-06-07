//! 导出命令实现（CSV / JSON）
//!
//! 路径安全验证 + I/O 在命令层，数据变换委托给 ExportUseCase。

use la_core::error::CommandError;
use la_core::models::LogEntry;
use tauri::{command, AppHandle, Manager};

use crate::application::{transform_csv, transform_json};

#[command]
pub async fn export_results(
    app: AppHandle,
    results: Vec<LogEntry>,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, CommandError> {
    let save_path = std::path::Path::new(&savePath);
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

    let safe = crate::utils::validation::prevent_path_traversal(&savePath)
        .map_err(|e| CommandError::new("EXPORT_PATH_UNSAFE", format!("导出路径不安全: {e}")))?;
    let download_dir = app.path().download_dir().map_err(|e| {
        CommandError::new(
            "DOWNLOAD_DIR_UNAVAILABLE",
            format!("无法获取下载目录: {e}"),
        )
    })?;
    let final_path = download_dir.join(&safe);

    if let Ok(canonical_final) = dunce::canonicalize(&final_path) {
        let cd = dunce::canonicalize(&download_dir).unwrap_or(download_dir);
        if !canonical_final.starts_with(&cd) {
            return Err(CommandError::new(
                "EXPORT_PATH_ESCAPE",
                "导出路径不允许超出下载目录范围".to_string(),
            ));
        }
    }

    let path_str = final_path.to_string_lossy().to_string();

    tokio::task::spawn_blocking(move || -> Result<String, CommandError> {
        match format.as_str() {
            "csv" => {
                let csv = transform_csv(&results);
                let mut f = std::fs::File::create(&path_str)
                    .map_err(|e| CommandError::new("IO_ERROR", e.to_string()))?;
                std::io::Write::write_all(&mut f, &[0xEFu8, 0xBB, 0xBF])
                    .map_err(|e| CommandError::new("IO_ERROR", e.to_string()))?;
                std::io::Write::write_all(&mut f, csv.as_bytes())
                    .map_err(|e| CommandError::new("IO_ERROR", e.to_string()))?;
                Ok(path_str)
            }
            "json" => {
                let json = transform_json(&results);
                std::fs::write(&path_str, json)
                    .map_err(|e| CommandError::new("IO_ERROR", e.to_string()))?;
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
