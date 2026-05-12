use la_core::traits::AppConfigProvider;
use tauri::Manager;

/// Tauri AppHandle 的 AppConfigProvider 适配器
///
/// 由于 Rust 孤儿规则，无法直接为 `tauri::AppHandle` 实现外部 trait。
/// 使用 newtype 包装器绕过此限制，将框架特定的路径获取逻辑隔离在 adapter 层。
pub struct TauriAppConfigProvider(pub tauri::AppHandle);

impl AppConfigProvider for TauriAppConfigProvider {
    fn config_dir(&self) -> std::result::Result<std::path::PathBuf, String> {
        self.0
            .path()
            .app_config_dir()
            .map_err(|e: tauri::Error| e.to_string())
    }
}
