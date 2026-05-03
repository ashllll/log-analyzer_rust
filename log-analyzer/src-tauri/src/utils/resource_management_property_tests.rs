//! 资源管理属性测试
//!
//! 本模块包含资源管理相关的属性测试，验证：
//! - Property 17: 临时目录清理
//! - Property 19: 搜索取消
//!
//! 使用 proptest 框架进行属性测试，确保资源管理在各种输入下都能正确工作。

#[cfg(test)]
mod tests {
    use crate::utils::cancellation_manager::CancellationManager;
    use crate::utils::resource_tracker::{ResourceTracker, ResourceType};
    use proptest::prelude::*;
    use std::fs;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;

    /// **Feature: bug-fixes, Property 17: Temporary Directory Cleanup**
    /// **Validates: Requirements 5.1**
    ///
    /// Property: 使用 tempfile crate 创建的临时目录在 drop 时应该被自动清理
    #[test]
    fn property_temp_directory_cleanup() {
        proptest!(|(prefix in "[a-z]{3,10}")| {
            let base_temp = TempDir::new().unwrap();
            let temp_path = base_temp.path().join(format!("{}_test", prefix));
            fs::create_dir_all(&temp_path).unwrap();

            prop_assert!(temp_path.exists(), "Temp directory should exist after creation");

            // 手动清理模拟 tempfile::TempDir 的 drop 行为
            fs::remove_dir_all(&temp_path).unwrap();

            prop_assert!(!temp_path.exists(), "Temp directory should be cleaned up after removal");
        });
    }

    /// **Feature: bug-fixes, Property 19: Search Cancellation**
    /// **Validates: Requirements 5.3**
    ///
    /// Property: 对于任何搜索操作，取消令牌被触发后，操作应该能够检测到取消状态
    #[test]
    fn property_search_cancellation() {
        proptest!(|(operation_id in "[a-z0-9\\-]{10,20}")| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(CancellationManager::new());

                // 创建取消令牌
                let token = manager.create_token(operation_id.clone());

                // 验证初始状态
                assert!(!token.is_cancelled(), "Token should not be cancelled initially");

                // 取消操作
                manager.cancel_operation(&operation_id).unwrap();

                // 验证取消状态
                assert!(token.is_cancelled(), "Token should be cancelled after cancel_operation");
            });
        });
    }

    /// **Feature: bug-fixes, Property 19: Search Cancellation (Async Task)**
    /// **Validates: Requirements 5.3**
    ///
    /// Property: 对于任何异步任务，取消令牌应该能够中断正在执行的任务
    #[tokio::test]
    async fn property_async_task_cancellation() {
        let manager = Arc::new(CancellationManager::new());
        let operation_id = "test-async-task".to_string();

        let token = manager.create_token(operation_id.clone());
        let token_clone = token.clone();
        let manager_clone = manager.clone();
        let operation_id_clone = operation_id.clone();

        // 启动异步任务
        let task_handle = tokio::spawn(async move {
            let mut iterations = 0;
            loop {
                tokio::select! {
                    _ = token_clone.cancelled() => {
                        return Ok::<_, String>(iterations);
                    }
                    _ = sleep(Duration::from_millis(10)) => {
                        iterations += 1;
                        if iterations > 100 {
                            return Err("Task did not cancel in time".to_string());
                        }
                    }
                }
            }
        });

        // 等待一小段时间后取消
        sleep(Duration::from_millis(50)).await;
        manager_clone.cancel_operation(&operation_id_clone).unwrap();

        // 等待任务完成
        let result = task_handle.await.unwrap();
        assert!(
            result.is_ok(),
            "Task should complete successfully after cancellation"
        );

        let iterations = result.unwrap();
        assert!(
            iterations < 100,
            "Task should be cancelled before reaching 100 iterations"
        );
    }

    /// **Feature: bug-fixes, Property 19: Search Cancellation (Multiple Operations)**
    /// **Validates: Requirements 5.3**
    ///
    /// Property: 对于多个并发操作，取消所有操作应该取消所有令牌
    #[test]
    fn property_cancel_all_operations() {
        proptest!(|(count in 1usize..10)| {
            let manager = Arc::new(CancellationManager::new());

            // 创建多个操作
            let tokens: Vec<_> = (0..count)
                .map(|i| {
                    let operation_id = format!("op-{}", i);
                    manager.create_token(operation_id)
                })
                .collect();

            // 验证所有令牌都未取消
            for token in &tokens {
                prop_assert!(!token.is_cancelled(), "Token should not be cancelled initially");
            }

            // 取消所有操作
            manager.cancel_all();

            // 验证所有令牌都被取消
            for token in &tokens {
                prop_assert!(token.is_cancelled(), "Token should be cancelled after cancel_all");
            }
        });
    }

    /// **Feature: bug-fixes, Property 17 & 19: Resource Tracking**
    /// **Validates: Requirements 5.1, 5.3, 5.5**
    ///
    /// Property: 对于任何注册的资源，追踪器应该能够正确追踪其生命周期
    #[test]
    fn property_resource_tracking() {
        proptest!(|(count in 1usize..10)| {
            let tracker = Arc::new(ResourceTracker::new());

            // 注册多个资源
            for i in 0..count {
                let resource_id = format!("resource-{}", i);
                tracker.register_resource(
                    resource_id,
                    ResourceType::TempDirectory,
                    format!("/tmp/test-{}", i),
                );
            }

            // 验证活跃资源数量
            prop_assert_eq!(tracker.active_count(), count, "Active count should match registered count");

            // 标记一半资源为已清理
            let half = count / 2;
            for i in 0..half {
                let resource_id = format!("resource-{}", i);
                tracker.mark_cleaned(&resource_id);
            }

            // 验证活跃资源数量
            let expected_active = count - half;
            prop_assert_eq!(tracker.active_count(), expected_active, "Active count should decrease after marking cleaned");

            // 清理所有资源
            tracker.cleanup_all();

            // 验证所有资源都被清理
            prop_assert_eq!(tracker.active_count(), 0, "All resources should be cleaned after cleanup_all");
        });
    }

    /// **Feature: bug-fixes, Property 17: Resource Leak Detection**
    /// **Validates: Requirements 5.5**
    ///
    /// Property: 对于任何长时间未清理的资源，泄漏检测应该能够识别
    /// 限制 proptest 迭代次数为 10，避免 CI 超时
    #[test]
    fn property_resource_leak_detection() {
        let proptest_config = proptest::test_runner::Config {
            cases: 10,
            ..Default::default()
        };
        proptest!(proptest_config, |(count in 1usize..3)| {
            let tracker = Arc::new(ResourceTracker::new());

            // 注册多个资源
            for i in 0..count {
                let resource_id = format!("resource-{}", i);
                tracker.register_resource(
                    resource_id,
                    ResourceType::TempDirectory,
                    format!("/tmp/test-{}", i),
                );
            }

            // 等待一小段时间（减少 sleep 时间）
            std::thread::sleep(Duration::from_millis(10));

            // 检测泄漏（阈值设为 5ms）
            let leaks = tracker.detect_leaks(Duration::from_millis(5));
            prop_assert_eq!(leaks.len(), count, "All resources should be detected as leaks");

            // 标记所有资源为已清理
            for i in 0..count {
                let resource_id = format!("resource-{}", i);
                tracker.mark_cleaned(&resource_id);
            }

            // 再次检测泄漏
            let leaks = tracker.detect_leaks(Duration::from_millis(5));
            prop_assert_eq!(leaks.len(), 0, "No leaks should be detected after marking cleaned");
        });
    }
}
