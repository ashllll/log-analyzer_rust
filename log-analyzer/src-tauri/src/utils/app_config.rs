//! Shared app-config loader — avoids duplicate "resolve dir → check exists → load" patterns.

use la_core::models::config::AppConfig;
use tauri::AppHandle;
use tauri::Manager;

/// Load the application configuration from `{app_config_dir}/config.json`.
///
/// Returns `None` if the config directory cannot be resolved or the file does not
/// exist.  Errors during parsing are silently swallowed; callers that need strict
/// validation should use `AppConfigLoader` directly.
pub fn load_app_config(app: &AppHandle) -> Option<AppConfig> {
    let config_path = app.path().app_config_dir().ok()?.join("config.json");
    if !config_path.exists() {
        return None;
    }

    la_core::models::config::AppConfigLoader::load(Some(config_path))
        .ok()
        .map(|loader| loader.get_config().clone())
}
