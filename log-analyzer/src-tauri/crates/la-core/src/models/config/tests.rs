#[cfg(test)]
mod tests {
    use super::*;

    // ============ ConfigValidator Trait 测试 ============

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());

        result.add_error("field1", "error message", "code1");
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "field1");
        assert_eq!(result.errors[0].message, "error message");
        assert_eq!(result.errors[0].code, "code1");
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.add_error("field1", "error1", "code1");

        let mut result2 = ValidationResult::new();
        result2.add_error("field2", "error2", "code2");

        result1.merge(result2);

        assert!(!result1.is_valid);
        assert_eq!(result1.errors.len(), 2);
    }

    #[test]
    fn test_validation_result_to_config_error() {
        let result = ValidationResult::new();
        assert!(result.to_config_error().is_none());

        let mut result = ValidationResult::new();
        result.add_error("field", "message", "code");
        let err = result.to_config_error();
        assert!(err.is_some());
        match err {
            Some(ConfigError::ValidationError { field, message }) => {
                assert_eq!(field, "field");
                assert_eq!(message, "message");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    // ============ ServerConfig 验证测试 ============

    #[test]
    fn test_server_config_valid() {
        let config = ServerConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_server_config_invalid_port() {
        let config = ServerConfig {
            port: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.field == "port"));
    }

    #[test]
    fn test_server_config_empty_host() {
        let config = ServerConfig {
            host: "".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.field == "host"));
    }

    #[test]
    fn test_server_config_invalid_max_connections() {
        let config = ServerConfig {
            max_connections: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    // ============ SearchConfig 验证测试 ============

    #[test]
    fn test_search_config_valid() {
        let config = SearchConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_search_config_invalid_max_results() {
        let config = SearchConfig {
            max_results: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);

        let config = SearchConfig {
            max_results: 2_000_000,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    // ============ MonitoringConfig 验证测试 ============

    #[test]
    fn test_monitoring_config_valid() {
        let config = MonitoringConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_monitoring_config_invalid_log_level() {
        let config = MonitoringConfig {
            log_level: "invalid".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.field == "log_level"));
    }

    // ============ SecurityConfig 验证测试 ============

    #[test]
    fn test_security_config_valid() {
        let config = SecurityConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_security_config_short_api_key() {
        let config = SecurityConfig {
            api_key: Some("short".to_string()),
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.field == "api_key"));
    }

    #[test]
    fn test_security_config_invalid_origin() {
        let config = SecurityConfig {
            allowed_origins: vec!["ftp://example.com".to_string()],
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    // ============ ArchiveConfig 验证测试 ============

    #[test]
    fn test_archive_config_valid() {
        let config = ArchiveConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_archive_config_invalid_depth() {
        let config = ArchiveConfig {
            max_extraction_depth: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_archive_config_invalid_compression_ratio() {
        let config = ArchiveConfig {
            max_compression_ratio: 0.5,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    // ============ FrontendConfig 验证测试 ============

    #[test]
    fn test_frontend_config_valid() {
        let config = FrontendConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_frontend_config_invalid_websocket_url() {
        let config = FrontendConfig {
            websocket_url: "http://localhost:8080".to_string(),
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.field == "websocket_url"));
    }

    #[test]
    fn test_frontend_config_invalid_port() {
        let config = FrontendConfig {
            vite_dev_server_port: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    // ============ FileFilterConfig 验证测试 ============

    #[test]
    fn test_file_filter_config_valid() {
        let config = FileFilterConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_file_filter_config_invalid_extension() {
        let config = FileFilterConfig {
            allowed_extensions: vec!["log@invalid".to_string()],
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
    }

    // ============ AppConfig 整体验证测试 ============

    #[test]
    fn test_app_config_valid() {
        let config = AppConfig::default();
        let result = config.validate();
        assert!(result.is_valid);
    }

    #[test]
    fn test_app_config_multiple_errors() {
        let config = AppConfig {
            server: ServerConfig {
                port: 0,
                host: "".to_string(),
                ..Default::default()
            },
            search: SearchConfig {
                max_results: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = config.validate();
        assert!(!result.is_valid);
        assert!(result.errors.len() >= 3); // port, host, max_results
    }

    // ============ validate_with_defaults 测试 ============

    #[test]
    fn test_validate_with_defaults_all_valid() {
        let config = ServerConfig::default();
        let (result, is_valid) = config.validate_with_defaults();
        assert!(result.is_valid);
        assert!(is_valid);
    }

    #[test]
    fn test_validate_with_defaults_invalid() {
        let config = ServerConfig {
            port: 0,
            max_connections: 50000,
            ..Default::default()
        };
        let (result, is_valid) = config.validate_with_defaults();
        assert!(!result.is_valid);
        assert!(!is_valid);
    }

    // ============ 序列化测试 ============

    #[test]
    fn test_config_error_serialization() {
        let error = ConfigError::ValidationError {
            field: "test_field".to_string(),
            message: "test message".to_string(),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("test_field"));
        assert!(json.contains("test message"));

        let deserialized: ConfigError = serde_json::from_str(&json).unwrap();
        match deserialized {
            ConfigError::ValidationError { field, message } => {
                assert_eq!(field, "test_field");
                assert_eq!(message, "test message");
            }
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_validation_result_serialization() {
        let mut result = ValidationResult::new();
        result.add_error("field1", "message1", "code1");
        result.add_error("field2", "message2", "code2");

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.is_valid);
        assert_eq!(deserialized.errors.len(), 2);
    }

    // ============ 辅助函数测试 ============

    #[test]
    fn test_validate_port() {
        assert!(validate_port(3000).is_none());
        assert!(validate_port(0).is_some());
    }

    #[test]
    fn test_validate_host() {
        assert!(validate_host("localhost").is_none());
        assert!(validate_host("").is_some());
        assert!(validate_host("host\0null").is_some());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range("test", 50, 0, 100).is_none());
        assert!(validate_range("test", -5, 0, 100).is_some());
        assert!(validate_range("test", 150, 0, 100).is_some());
    }

    #[test]
    fn test_validate_log_level() {
        assert!(validate_log_level("info").is_none());
        assert!(validate_log_level("INFO").is_none());
        assert!(validate_log_level("invalid").is_some());
    }

    #[test]
    fn test_validate_extension() {
        assert!(validate_extension("log").is_none());
        assert!(validate_extension("").is_some());
        assert!(validate_extension("log@invalid").is_some());
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("test", "/valid/path").is_none());
        assert!(validate_path("test", "").is_some());
        assert!(validate_path("test", "../traversal").is_some());
        assert!(validate_path("test", "path\0null").is_some());
    }

    #[test]
    fn test_validate_regex_pattern() {
        assert!(validate_regex_pattern("valid.*pattern").is_none());
        assert!(validate_regex_pattern("").is_none()); // 空模式允许
        assert!(validate_regex_pattern("[invalid").is_some());
    }
}
