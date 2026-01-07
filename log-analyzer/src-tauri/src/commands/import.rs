//! 导入相关命令实现
//! 包含工作区导入与 RAR 支持检查

use tauri::command;
use tracing::warn;

use crate::archive::rar_handler::{get_unrar_path, validate_unrar_binary};

/**
 * 检查 RAR 支持状态
 *
 * 实际检查 unrar 二进制文件是否存在并验证完整性（运行时验证）
 * 返回详细的诊断信息
 */
#[command]
pub async fn check_rar_support() -> Result<serde_json::Value, String> {
    let unrar_path = get_unrar_path();
    let validation = validate_unrar_binary(&unrar_path);
    
    if !validation.exists {
        warn!(
            "unrar binary not found at: {}",
            unrar_path.display()
        );
    }
    
    if !validation.is_valid {
        for error in &validation.errors {
            warn!("unrar binary validation error: {}", error);
        }
    }
    
    Ok(serde_json::json!({
        "available": validation.is_valid,
        "path": unrar_path.display().to_string(),
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "file_exists": validation.exists,
        "is_executable": validation.is_executable,
        "version_info": validation.version_info,
        "validation_errors": validation.errors,
        "bundled": true,
        "install_guide": if !validation.is_valid {
            Some("unrar 二进制文件似乎缺失或已损坏。请从官方源重新安装应用程序。")
        } else {
            None
        }
    }))
}
