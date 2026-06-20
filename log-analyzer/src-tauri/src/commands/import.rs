//! 导入相关命令实现
//!
//! # P4 / Repair Slice 2 重构
//!
//! `import_folder` 现在是薄层 Tauri 命令：导入生命周期已下沉到
//! `infrastructure::import_pipeline::run_import`。
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use serde::Serialize;
use tauri::{command, AppHandle, State};

use crate::infrastructure::import_pipeline::run_import;
use crate::infrastructure::{TauriEventPublisher, TauriWorkspacePaths};
use crate::models::AppState;
use std::sync::Arc;

// ============================================================================
// 共享工具函数
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct RarSupportInfo {
    compiled: bool,
    available: bool,
    reason: Option<String>,
}

fn get_rar_support_info() -> RarSupportInfo {
    #[cfg(feature = "rar-support")]
    {
        RarSupportInfo {
            compiled: true,
            available: true,
            reason: None,
        }
    }

    #[cfg(not(feature = "rar-support"))]
    {
        RarSupportInfo {
            compiled: false,
            available: false,
            reason: Some("RAR support is not compiled into this build".to_string()),
        }
    }
}

// ============================================================================
// 简化版 import_folder 命令
// ============================================================================

#[tauri::command]
pub async fn import_folder(
    app: AppHandle,
    path: String,
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let event_publisher = Arc::new(TauriEventPublisher {
        app_handle: app.clone(),
    });
    let workspace_paths = TauriWorkspacePaths::new(&app)?;
    let config_provider = crate::adapters::tauri_config::TauriAppConfigProvider(app.clone());
    run_import(
        event_publisher,
        &workspace_paths,
        &config_provider,
        &app,
        &state,
        &workspace_id,
        &path,
    )
    .await
}

/// 检查 RAR 支持状态（无 sidecar 依赖）
#[command]
pub async fn check_rar_support() -> Result<serde_json::Value, String> {
    serde_json::to_value(get_rar_support_info())
        .map_err(|error| format!("Failed to serialize RAR support info: {error}"))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::get_rar_support_info;

    #[test]
    fn rar_support_reports_compiled_feature_state() {
        let support = get_rar_support_info();

        #[cfg(feature = "rar-support")]
        {
            assert!(support.compiled);
            assert!(support.available);
            assert!(support.reason.is_none());
        }

        #[cfg(not(feature = "rar-support"))]
        {
            assert!(!support.compiled);
            assert!(!support.available);
            assert!(support
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("not compiled")));
        }
    }
}
