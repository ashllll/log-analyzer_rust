//! ConfigUseCase — application-layer configuration orchestration.

use std::fs;
use std::io::Write;
use std::sync::Arc;

use la_core::error::{AppError, Result};
use la_core::models::config::{AppConfig, AppConfigLoader, ConfigValidator};
use la_core::traits::AppConfigProvider;

/// Application use case for loading and saving app configuration.
pub struct ConfigUseCase<P>
where
    P: AppConfigProvider + 'static,
{
    provider: Arc<P>,
}

impl<P> ConfigUseCase<P>
where
    P: AppConfigProvider,
{
    pub fn new(provider: Arc<P>) -> Self {
        Self { provider }
    }

    pub fn load(&self) -> Result<AppConfig> {
        let config_path = self.config_path()?;
        if !config_path.exists() {
            return Ok(AppConfig::default());
        }

        AppConfigLoader::load(Some(config_path))
            .map(|loader| loader.get_config().clone())
            .or_else(|error| {
                tracing::warn!(%error, "Failed to load config, falling back to default");
                Ok(AppConfig::default())
            })
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        let validation = config.validate();
        if !validation.is_valid {
            let errors = validation
                .errors
                .iter()
                .map(|error| format!("{}: {}", error.field, error.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(AppError::config_error(format!("配置验证失败: {errors}")));
        }

        let config_dir = self
            .provider
            .config_dir()
            .map_err(|e| AppError::config_error(e.to_string()))?;
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| AppError::io_error(e.to_string(), Some(config_dir.clone())))?;
        }

        let config_path = config_dir.join("config.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| AppError::config_error(format!("Failed to serialize config: {e}")))?;

        let mut temporary = tempfile::Builder::new()
            .prefix(".config.json.")
            .suffix(".tmp")
            .tempfile_in(&config_dir)
            .map_err(|e| AppError::io_error(e.to_string(), Some(config_dir.clone())))?;
        temporary
            .write_all(json.as_bytes())
            .map_err(|e| AppError::io_error(e.to_string(), Some(temporary.path().to_path_buf())))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|e| AppError::io_error(e.to_string(), Some(temporary.path().to_path_buf())))?;
        temporary
            .persist(&config_path)
            .map_err(|e| AppError::io_error(e.error.to_string(), Some(config_path)))?;
        Ok(())
    }

    fn config_path(&self) -> Result<std::path::PathBuf> {
        self.provider
            .config_dir()
            .map(|dir| dir.join("config.json"))
            .map_err(|e| AppError::config_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TempConfigProvider {
        dir: std::path::PathBuf,
    }

    impl AppConfigProvider for TempConfigProvider {
        fn config_dir(&self) -> std::result::Result<std::path::PathBuf, String> {
            Ok(self.dir.clone())
        }
    }

    #[test]
    fn config_use_case_loads_default_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let use_case = ConfigUseCase::new(Arc::new(TempConfigProvider {
            dir: temp_dir.path().to_path_buf(),
        }));

        let config = use_case.load().unwrap();
        assert_eq!(
            config.search.max_results,
            AppConfig::default().search.max_results
        );
    }

    #[test]
    fn config_use_case_saves_and_loads_config() {
        let temp_dir = TempDir::new().unwrap();
        let use_case = ConfigUseCase::new(Arc::new(TempConfigProvider {
            dir: temp_dir.path().to_path_buf(),
        }));
        let mut config = AppConfig::default();
        config.search.max_results = 1234;

        use_case.save(&config).unwrap();
        let loaded = use_case.load().unwrap();

        assert_eq!(loaded.search.max_results, 1234);
    }

    #[test]
    fn concurrent_config_saves_succeed_and_leave_a_loadable_config() {
        const SAVE_COUNT: usize = 32;

        let temp_dir = TempDir::new().unwrap();
        let use_case = Arc::new(ConfigUseCase::new(Arc::new(TempConfigProvider {
            dir: temp_dir.path().to_path_buf(),
        })));
        let start = Arc::new(std::sync::Barrier::new(SAVE_COUNT));

        let handles = (0..SAVE_COUNT)
            .map(|offset| {
                let use_case = Arc::clone(&use_case);
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
                    let mut config = AppConfig::default();
                    config.search.max_results = 1000 + offset;
                    start.wait();
                    use_case.save(&config)
                })
            })
            .collect::<Vec<_>>();

        let failures = handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap().err())
            .collect::<Vec<_>>();
        assert!(
            failures.is_empty(),
            "concurrent saves must not fail: {failures:?}"
        );

        let loaded = use_case.load().unwrap();
        assert!((1000..1000 + SAVE_COUNT).contains(&loaded.search.max_results));
    }
}
