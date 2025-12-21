//! 资源管理属性测试
//!
//! 测试资源管理的正确性属性，包括：
//! - Property 17: 临时目录清理
//! - Property 19: 搜索取消

use log_analyzer::utils::async_resource_manager::OperationType;
use log_analyzer::utils::{AsyncResourceManager, ResourceManager, ResourceType};
use proptest::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Property 17: Temporary Directory Cleanup
///
/// **Feature: bug-fixes, Property 17: Temporary Directory Cleanup**
///
/// *For any* temporary directory creation, cleanup should occur on application exit
/// **Validates: Requirements 5.1**
#[cfg(test)]
mod property_17_temp_directory_cleanup {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn temp_directory_cleanup_on_guard_drop(
            prefixes in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..10)
        ) {
            let manager = ResourceManager::new();
            let mut created_paths = Vec::new();
            let mut resource_ids = Vec::new();

            // 创建多个临时目录守卫
            {
                let mut guards = Vec::new();
                for prefix in &prefixes {
                    let guard = manager.create_temp_dir_guard(prefix).unwrap();
                    created_paths.push(guard.path().to_path_buf());
                    resource_ids.push(guard.resource_id().to_string());
                    guards.push(guard);
                }

                // 验证所有目录都存在
                for path in &created_paths {
                    prop_assert!(path.exists(), "Temporary directory should exist while guard is alive");
                }

                // 验证资源已注册
                let stats = manager.get_resource_stats();
                prop_assert_eq!(stats.active_resources, prefixes.len());

            } // 所有守卫在这里被丢弃，应该触发清理

            // 验证资源已被清理
            let stats = manager.get_resource_stats();
            prop_assert_eq!(stats.active_resources, 0, "All resources should be cleaned up after guards are dropped");

            // 验证所有资源信息都标记为已清理
            for resource_id in &resource_ids {
                if let Some(resource_info) = manager.get_resource_info(resource_id) {
                    prop_assert!(resource_info.cleaned, "Resource should be marked as cleaned");
                }
            }
        }

        #[test]
        fn temp_directory_manual_cleanup(
            prefixes in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
        ) {
            let manager = ResourceManager::new();
            let mut resource_ids = Vec::new();

            // 创建临时目录
            for prefix in &prefixes {
                let (resource_id, path) = manager.create_temp_dir(prefix).unwrap();
                prop_assert!(path.exists(), "Temporary directory should exist after creation");
                resource_ids.push(resource_id);
            }

            let initial_stats = manager.get_resource_stats();
            prop_assert_eq!(initial_stats.active_resources, prefixes.len());

            // 手动清理所有资源
            for resource_id in &resource_ids {
                manager.cleanup_resource(resource_id).unwrap();
            }

            // 验证清理后的状态
            let final_stats = manager.get_resource_stats();
            prop_assert_eq!(final_stats.active_resources, 0, "All resources should be cleaned up");

            // 验证所有资源都标记为已清理
            for resource_id in &resource_ids {
                if let Some(resource_info) = manager.get_resource_info(resource_id) {
                    prop_assert!(resource_info.cleaned, "Resource should be marked as cleaned");
                }
            }
        }

        #[test]
        fn cleanup_all_resources(
            prefixes in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..10)
        ) {
            let manager = ResourceManager::new();

            // 创建多个临时目录
            for prefix in &prefixes {
                let (_resource_id, path) = manager.create_temp_dir(prefix).unwrap();
                prop_assert!(path.exists(), "Temporary directory should exist after creation");
            }

            let initial_stats = manager.get_resource_stats();
            prop_assert_eq!(initial_stats.active_resources, prefixes.len());

            // 清理所有资源
            manager.cleanup_all().unwrap();

            // 验证所有资源都被清理
            let final_stats = manager.get_resource_stats();
            prop_assert_eq!(final_stats.active_resources, 0, "All resources should be cleaned up");
            prop_assert_eq!(final_stats.temp_dir_count, 0, "All temp directories should be cleaned up");
        }
    }
}

/// Property 19: Search Cancellation
///
/// **Feature: bug-fixes, Property 19: Search Cancellation**
///
/// *For any* search operation cancellation, ongoing file processing should be aborted properly
/// **Validates: Requirements 5.3**
#[cfg(test)]
mod property_19_search_cancellation {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn search_operation_cancellation(
            workspace_ids in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5),
            queries in prop::collection::vec("[a-zA-Z0-9 ]{1,50}", 1..5)
        ) {
            tokio_test::block_on(async {
                let manager = AsyncResourceManager::new();
                let mut operation_ids = Vec::new();
                let mut tokens = Vec::new();

                // 注册多个搜索操作 - 为每个工作区创建一个操作
                for (i, workspace_id) in workspace_ids.iter().enumerate() {
                    let query = queries.get(i % queries.len()).unwrap(); // 循环使用查询
                    let (operation_id, token) = manager.register_search_operation(
                        workspace_id.clone(),
                        query.clone(),
                    ).await;

                    prop_assert!(!token.is_cancelled(), "Token should not be cancelled initially");
                    operation_ids.push(operation_id);
                    tokens.push(token);
                }

                let initial_count = manager.active_operations_count().await;
                prop_assert_eq!(initial_count, workspace_ids.len(), "All operations should be registered");

                // 取消所有操作
                for operation_id in &operation_ids {
                    manager.cancel_operation(operation_id).await.unwrap();
                }

                // 验证所有令牌都被取消
                for token in &tokens {
                    prop_assert!(token.is_cancelled(), "All tokens should be cancelled");
                }

                // 验证操作计数为0
                let final_count = manager.active_operations_count().await;
                prop_assert_eq!(final_count, 0, "All operations should be removed after cancellation");

                Ok(())
            })?;
        }

        #[test]
        fn workspace_operations_cancellation(
            workspace_ids in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 2..5),
            operations_per_workspace in 1..5usize
        ) {
            tokio_test::block_on(async {
                let manager = AsyncResourceManager::new();
                let mut all_tokens = Vec::new();
                let target_workspace = &workspace_ids[0];

                // 为每个工作区创建多个操作
                for workspace_id in &workspace_ids {
                    for i in 0..operations_per_workspace {
                        let token = manager.register_operation(
                            format!("op_{}_{}", workspace_id, i),
                            OperationType::Search,
                            Some(workspace_id.clone())
                        ).await;
                        all_tokens.push((workspace_id.clone(), token));
                    }
                }

                let initial_count = manager.active_operations_count().await;
                let expected_total = workspace_ids.len() * operations_per_workspace;
                prop_assert_eq!(initial_count, expected_total, "All operations should be registered");

                // 取消目标工作区的所有操作
                let cancelled_count = manager.cancel_workspace_operations(target_workspace).await.unwrap();
                prop_assert_eq!(cancelled_count, operations_per_workspace, "Should cancel all operations for target workspace");

                // 验证目标工作区的令牌都被取消
                for (workspace_id, token) in &all_tokens {
                    if workspace_id == target_workspace {
                        prop_assert!(token.is_cancelled(), "Target workspace tokens should be cancelled");
                    } else {
                        prop_assert!(!token.is_cancelled(), "Other workspace tokens should not be cancelled");
                    }
                }

                // 验证剩余操作数量
                let remaining_count = manager.active_operations_count().await;
                let expected_remaining = expected_total - operations_per_workspace;
                prop_assert_eq!(remaining_count, expected_remaining, "Should have correct number of remaining operations");

                Ok(())
            })?;
        }

        #[test]
        fn global_cancellation(
            operation_count in 1..20usize
        ) {
            tokio_test::block_on(async {
                let manager = AsyncResourceManager::new();
                let mut tokens = Vec::new();

                // 创建多个不同类型的操作
                for i in 0..operation_count {
                    let operation_type = match i % 4 {
                        0 => OperationType::Search,
                        1 => OperationType::FileWatch,
                        2 => OperationType::ArchiveExtraction,
                        _ => OperationType::BackgroundTask,
                    };

                    let token = manager.register_operation(
                        format!("operation_{}", i),
                        operation_type,
                        Some(format!("workspace_{}", i % 3))
                    ).await;

                    tokens.push(token);
                }

                let initial_count = manager.active_operations_count().await;
                prop_assert_eq!(initial_count, operation_count, "All operations should be registered");

                // 全局取消
                manager.cancel_all_operations().await.unwrap();

                // 验证所有令牌都被取消
                for token in &tokens {
                    prop_assert!(token.is_cancelled(), "All tokens should be cancelled after global cancellation");
                }

                // 验证操作计数为0
                let final_count = manager.active_operations_count().await;
                prop_assert_eq!(final_count, 0, "All operations should be removed after global cancellation");

                Ok(())
            })?;
        }

        #[test]
        fn graceful_shutdown_with_timeout(
            operation_count in 1..10usize,
            timeout_ms in 100..1000u64
        ) {
            tokio_test::block_on(async {
                let manager = AsyncResourceManager::new();
                let mut tokens = Vec::new();

                // 创建一些操作
                for i in 0..operation_count {
                    let token = manager.register_operation(
                        format!("operation_{}", i),
                        OperationType::BackgroundTask,
                        None
                    ).await;
                    tokens.push(token);
                }

                let initial_count = manager.active_operations_count().await;
                prop_assert_eq!(initial_count, operation_count, "All operations should be registered");

                // 执行优雅关闭
                let timeout = Duration::from_millis(timeout_ms);
                manager.graceful_shutdown(timeout).await.unwrap();

                // 验证所有操作都被取消
                for token in &tokens {
                    prop_assert!(token.is_cancelled(), "All tokens should be cancelled after graceful shutdown");
                }

                let final_count = manager.active_operations_count().await;
                prop_assert_eq!(final_count, 0, "All operations should be removed after graceful shutdown");

                Ok(())
            })?;
        }
    }
}

/// 集成测试：资源管理器和异步资源管理器协同工作
#[cfg(test)]
mod integration_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn integrated_resource_management(
            temp_dir_count in 1..5usize,
            async_operation_count in 1..5usize
        ) {
            tokio_test::block_on(async {
                let sync_manager = Arc::new(ResourceManager::new());
                let async_manager = AsyncResourceManager::with_sync_manager(Arc::clone(&sync_manager));

                let mut temp_guards = Vec::new();
                let mut async_tokens = Vec::new();

                // 创建临时目录
                for i in 0..temp_dir_count {
                    let guard = sync_manager.create_temp_dir_guard(&format!("test_{}", i)).unwrap();
                    temp_guards.push(guard);
                }

                // 创建异步操作
                for i in 0..async_operation_count {
                    let token = async_manager.register_operation(
                        format!("async_op_{}", i),
                        OperationType::Search,
                        Some(format!("workspace_{}", i))
                    ).await;
                    async_tokens.push(token);
                }

                // 验证初始状态
                let sync_stats = sync_manager.get_resource_stats();
                prop_assert_eq!(sync_stats.active_resources, temp_dir_count);

                let async_count = async_manager.active_operations_count().await;
                prop_assert_eq!(async_count, async_operation_count);

                // 执行优雅关闭
                async_manager.graceful_shutdown(Duration::from_secs(1)).await.unwrap();

                // 验证异步操作被取消
                for token in &async_tokens {
                    prop_assert!(token.is_cancelled(), "Async tokens should be cancelled");
                }

                let final_async_count = async_manager.active_operations_count().await;
                prop_assert_eq!(final_async_count, 0, "All async operations should be cleaned up");

                // 同步资源也应该被清理（通过集成）
                let final_sync_stats = sync_manager.get_resource_stats();
                prop_assert_eq!(final_sync_stats.active_resources, 0, "Sync resources should be cleaned up through integration");

                // 手动丢弃守卫以确保清理
                drop(temp_guards);

                Ok(())
            })?;
        }
    }
}

/// 单元测试：边界情况和错误处理
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_empty_resource_manager() {
        let manager = ResourceManager::new();
        let stats = manager.get_resource_stats();

        assert_eq!(stats.total_resources, 0);
        assert_eq!(stats.active_resources, 0);
        assert_eq!(stats.temp_dir_count, 0);
        assert!(stats.by_type.is_empty());
    }

    #[test]
    fn test_cleanup_nonexistent_resource() {
        let manager = ResourceManager::new();
        let result = manager.cleanup_resource("nonexistent");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_operation() {
        let manager = AsyncResourceManager::new();
        let result = manager.cancel_operation("nonexistent").await;

        // 应该成功，但不会有实际效果
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_workspace_cancellation() {
        let manager = AsyncResourceManager::new();
        let cancelled_count = manager
            .cancel_workspace_operations("nonexistent")
            .await
            .unwrap();

        assert_eq!(cancelled_count, 0);
    }

    #[test]
    fn test_resource_guard_manual_cleanup() {
        let manager = ResourceManager::new();
        let cleanup_called = Arc::new(std::sync::Mutex::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        let guard = manager
            .register_resource(
                ResourceType::FileHandle,
                Some(std::path::PathBuf::from("/test/path")),
                move || {
                    *cleanup_called_clone.lock().unwrap() = true;
                },
            )
            .unwrap();

        // 手动清理
        guard.cleanup().unwrap();

        // 验证清理函数被调用
        assert!(*cleanup_called.lock().unwrap());
    }
}
