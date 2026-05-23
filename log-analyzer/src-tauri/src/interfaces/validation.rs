//! Validation command interface adapters.

use la_core::models::validated::{
    ValidatedArchiveConfig, ValidatedSearchQuery, ValidatedWorkspaceConfig, ValidationErrorReport,
};

#[tauri::command]
pub async fn validate_workspace_config_cmd(
    config: ValidatedWorkspaceConfig,
) -> Result<ValidationErrorReport, String> {
    crate::commands::validation::validate_workspace_config_cmd(config).await
}

#[tauri::command]
pub async fn validate_search_query_cmd(
    query: ValidatedSearchQuery,
) -> Result<ValidationErrorReport, String> {
    crate::commands::validation::validate_search_query_cmd(query).await
}

#[tauri::command]
pub async fn validate_archive_config_cmd(
    config: ValidatedArchiveConfig,
) -> Result<ValidationErrorReport, String> {
    crate::commands::validation::validate_archive_config_cmd(config).await
}

#[tauri::command]
pub async fn batch_validate_workspace_configs(
    configs: Vec<ValidatedWorkspaceConfig>,
) -> Result<Vec<ValidationErrorReport>, String> {
    crate::commands::validation::batch_validate_workspace_configs(configs).await
}

#[tauri::command]
pub async fn validate_workspace_id_format(workspace_id: String) -> Result<bool, String> {
    crate::commands::validation::validate_workspace_id_format(workspace_id).await
}

#[tauri::command]
pub async fn validate_path_security(path: String) -> Result<ValidationErrorReport, String> {
    crate::commands::validation::validate_path_security(path).await
}
