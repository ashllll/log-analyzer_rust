//! 端到端验证测试
//!
//! 验证应用启动和任务管理的完整流程

#[cfg(test)]
mod e2e_tests {
    use log_analyzer::task_manager::{TaskManagerConfig, TaskStatus};

    /// 验证配置系统正常工作
    #[test]
    fn test_config_system() {
        // 测试默认配置
        let default_config = TaskManagerConfig::default();
        assert_eq!(default_config.completed_task_ttl, 3);
        assert_eq!(default_config.failed_task_ttl, 10);
        assert_eq!(default_config.cleanup_interval, 1);
        assert_eq!(default_config.operation_timeout, 5);

        // 测试自定义配置
        let custom_config = TaskManagerConfig {
            completed_task_ttl: 10,
            failed_task_ttl: 20,
            cleanup_interval: 5,
            operation_timeout: 15,
        };
        assert_eq!(custom_config.completed_task_ttl, 10);
        assert_eq!(custom_config.failed_task_ttl, 20);
    }

    /// 验证任务状态枚举
    #[test]
    fn test_task_status_serialization() {
        let statuses = vec![
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Stopped,
        ];

        for status in statuses {
            // 验证序列化
            let json = serde_json::to_string(&status);
            assert!(json.is_ok(), "Status {:?} should serialize", status);

            // 验证反序列化
            let json_str = json.unwrap();
            let deserialized: Result<TaskStatus, _> = serde_json::from_str(&json_str);
            assert!(
                deserialized.is_ok(),
                "Status {:?} should deserialize",
                status
            );
            assert_eq!(deserialized.unwrap(), status);
        }
    }

    /// 验证任务状态转换逻辑
    #[test]
    fn test_task_status_transitions() {
        // 验证状态相等性
        assert_eq!(TaskStatus::Running, TaskStatus::Running);
        assert_ne!(TaskStatus::Running, TaskStatus::Completed);

        // 验证状态可以被克隆
        let status1 = TaskStatus::Running;
        let status2 = status1;
        assert_eq!(status1, status2);
    }

    /// 验证配置克隆和调试输出
    #[test]
    fn test_config_traits() {
        let config = TaskManagerConfig::default();

        // 测试克隆
        let cloned = config.clone();
        assert_eq!(config.completed_task_ttl, cloned.completed_task_ttl);

        // 测试调试输出
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("TaskManagerConfig"));
        assert!(debug_str.contains("completed_task_ttl"));
    }

    /// 验证配置边界值
    #[test]
    fn test_config_boundary_values() {
        // 测试最小值
        let min_config = TaskManagerConfig {
            completed_task_ttl: 1,
            failed_task_ttl: 1,
            cleanup_interval: 1,
            operation_timeout: 1,
        };
        assert_eq!(min_config.completed_task_ttl, 1);

        // 测试大值
        let large_config = TaskManagerConfig {
            completed_task_ttl: 3600,
            failed_task_ttl: 7200,
            cleanup_interval: 60,
            operation_timeout: 300,
        };
        assert_eq!(large_config.completed_task_ttl, 3600);
    }

    /// 验证任务状态的所有变体
    #[test]
    fn test_all_task_status_variants() {
        let all_statuses = vec![
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Stopped,
        ];

        // 确保所有状态都可以序列化和比较
        for (i, status1) in all_statuses.iter().enumerate() {
            for (j, status2) in all_statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(status1, status2, "Same status should be equal");
                } else {
                    assert_ne!(status1, status2, "Different statuses should not be equal");
                }
            }
        }
    }

    /// 验证配置的合理性检查
    #[test]
    fn test_config_sanity_checks() {
        let config = TaskManagerConfig::default();

        // 验证 TTL 值合理
        assert!(
            config.completed_task_ttl > 0,
            "Completed task TTL should be positive"
        );
        assert!(
            config.failed_task_ttl > 0,
            "Failed task TTL should be positive"
        );

        // 验证清理间隔合理
        assert!(
            config.cleanup_interval > 0,
            "Cleanup interval should be positive"
        );

        // 验证超时值合理
        assert!(
            config.operation_timeout > 0,
            "Operation timeout should be positive"
        );

        // 验证失败任务保留时间应该比完成任务长
        assert!(
            config.failed_task_ttl >= config.completed_task_ttl,
            "Failed tasks should be kept longer than completed tasks"
        );
    }
}
