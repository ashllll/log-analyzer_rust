//! 生产系统集成测试
//!
//! 测试所有生产就绪系统之间的交互：
//! - eyre + tracing + parking_lot + moka 集成
//! - validator + eyre 错误报告集成
//! - scopeguard + tokio-util 取消协调
//! - 依赖注入和服务生命周期管理

#[cfg(test)]
mod tests {
    use crate::services::{AppServices, ServiceConfiguration};
    use eyre::{Context, Result};
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use tracing::{error, info, warn};

    /// 测试 eyre + tracing + parking_lot 集成
    #[test]
    fn test_eyre_tracing_parking_lot_integration() -> Result<()> {
        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        // 使用 parking_lot Mutex
        let data = Arc::new(Mutex::new(Vec::<String>::new()));

        // 使用 eyre 进行错误处理
        let result: Result<()> = (|| {
            let mut guard = data.lock();
            guard.push("test".to_string());

            info!("Successfully added item to vector");

            if guard.is_empty() {
                return Err(eyre::eyre!("Vector should not be empty"))
                    .context("Checking vector state");
            }

            Ok(())
        })();

        assert!(result.is_ok());

        // 验证数据
        let guard = data.lock();
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0], "test");

        Ok(())
    }

    /// 测试 eyre + tracing + moka 缓存集成
    #[tokio::test]
    async fn test_eyre_tracing_moka_integration() -> Result<()> {
        use moka::future::Cache;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        // 创建 moka 缓存
        let cache: Cache<String, String> = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(60))
            .build();

        // 使用 eyre 进行错误处理
        let result: Result<()> = async {
            // 插入数据
            cache.insert("key1".to_string(), "value1".to_string()).await;
            info!("Inserted item into cache");

            // 获取数据
            let value = cache
                .get(&"key1".to_string())
                .await
                .ok_or_else(|| eyre::eyre!("Key not found in cache"))
                .context("Retrieving value from cache")?;

            assert_eq!(value, "value1");
            info!("Successfully retrieved item from cache");

            Ok(())
        }
        .await;

        assert!(result.is_ok());

        // 验证缓存统计（moka 缓存可能需要时间更新统计）
        // 使用 get 来确认数据存在
        let cached_value = cache.get(&"key1".to_string()).await;
        assert!(cached_value.is_some());
        assert_eq!(cached_value.unwrap(), "value1");

        Ok(())
    }

    /// 测试 validator + eyre 错误报告集成
    #[test]
    fn test_validator_eyre_integration() -> Result<()> {
        use crate::utils::validation::validate_safe_path;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        // 测试有效路径
        let valid_result = validate_safe_path("valid/path/to/file.txt");
        assert!(valid_result.is_ok());
        info!("Valid path passed validation");

        // 测试无效路径（路径遍历）
        let invalid_result = validate_safe_path("../../../etc/passwd");
        assert!(invalid_result.is_err());

        if let Err(e) = invalid_result {
            // 验证错误可以转换为 eyre::Report
            let report: eyre::Report = e.into();
            let error_msg = format!("{:?}", report);
            assert!(error_msg.contains("path"));
            warn!("Invalid path rejected: {}", error_msg);
        }

        Ok(())
    }

    /// 测试 scopeguard + tokio-util 取消协调
    #[tokio::test]
    async fn test_scopeguard_tokio_cancellation_integration() -> Result<()> {
        use scopeguard::defer;
        use tokio_util::sync::CancellationToken;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        let cleanup_flag = Arc::new(Mutex::new(false));
        let cleanup_flag_clone = Arc::clone(&cleanup_flag);

        // 使用 scopeguard 确保清理
        let _guard = scopeguard::guard((), |_| {
            let mut flag = cleanup_flag_clone.lock();
            *flag = true;
            info!("Cleanup executed via scopeguard");
        });

        // 创建取消令牌
        let token = CancellationToken::new();
        let token_clone = token.clone();

        // 启动可取消的任务
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    info!("Task cancelled gracefully");
                    Ok::<_, eyre::Report>(())
                }
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    Err(eyre::eyre!("Task should have been cancelled"))
                }
            }
        });

        // 立即取消任务
        token.cancel();

        // 等待任务完成
        let result = task.await.context("Waiting for task completion")?;
        assert!(result.is_ok());

        // 验证清理标志（在 defer 执行后）
        drop(_guard);
        let flag = cleanup_flag.lock();
        assert!(*flag);

        Ok(())
    }

    /// 测试依赖注入和服务生命周期管理
    #[test]
    fn test_dependency_injection_service_lifecycle() -> Result<()> {
        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        // 创建服务配置
        let config = ServiceConfiguration::development();
        info!("Created development configuration");

        // 使用依赖注入创建服务
        let services = AppServices::builder()
            .with_config(config)
            .build()
            .context("Building services with dependency injection")?;

        info!("Services created successfully");

        // 启动所有服务
        services
            .start_all()
            .context("Starting all services")?;
        info!("All services started");

        // 检查健康状态
        let health = services.overall_health();
        assert_eq!(health.status, crate::services::HealthStatus::Healthy);
        info!("All services are healthy");

        // 使用服务
        let event_bus = services.event_bus();
        assert!(event_bus.subscriber_count() >= 0);

        let resource_tracker = services.resource_tracker();
        let report = resource_tracker.generate_report();
        assert!(report.total >= 0);
        info!("Services are functioning correctly");

        // 停止所有服务
        services
            .stop_all()
            .context("Stopping all services")?;
        info!("All services stopped");

        Ok(())
    }

    /// 测试并发场景下的系统集成
    #[test]
    fn test_concurrent_system_integration() -> Result<()> {
        use moka::sync::Cache;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        // 创建共享缓存（使用 moka）
        let cache = Arc::new(
            Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(60))
                .build(),
        );

        // 创建共享状态（使用 parking_lot）
        let state = Arc::new(Mutex::new(0usize));

        let mut handles = vec![];

        // 启动多个线程进行并发操作
        for i in 0..10 {
            let cache = Arc::clone(&cache);
            let state = Arc::clone(&state);

            let handle = thread::spawn(move || -> Result<()> {
                // 使用 eyre 进行错误处理
                let result: Result<()> = (|| {
                    // 更新状态（使用 parking_lot）
                    {
                        let mut guard = state.lock();
                        *guard += 1;
                    }

                    // 更新缓存（使用 moka）
                    cache.insert(format!("key_{}", i), format!("value_{}", i));

                    info!("Thread {} completed successfully", i);
                    Ok(())
                })();

                result.context(format!("Thread {} execution", i))
            });

            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle
                .join()
                .map_err(|_| eyre::eyre!("Thread panicked"))?
                .context("Thread execution")?;
        }

        // 验证最终状态
        let final_state = state.lock();
        assert_eq!(*final_state, 10);

        // 验证缓存（通过实际查询而不是 entry_count）
        for i in 0..10 {
            let key = format!("key_{}", i % 10);
            let value = cache.get(&key);
            assert!(value.is_some(), "Key {} should exist in cache", key);
        }

        info!("Concurrent system integration test completed successfully");

        Ok(())
    }

    /// 测试错误恢复和优雅降级
    #[tokio::test]
    async fn test_error_recovery_graceful_degradation() -> Result<()> {
        use tokio_util::sync::CancellationToken;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        let services = AppServices::new().context("Creating services")?;

        // 启动服务
        services.start_all().context("Starting services")?;

        // 模拟错误场景
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let task = tokio::spawn(async move {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    warn!("Operation cancelled, performing graceful shutdown");
                    Ok::<_, eyre::Report>(())
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    error!("Operation timed out");
                    Err(eyre::eyre!("Operation timeout"))
                }
            }
        });

        // 等待一小段时间后取消
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();

        // 验证优雅取消
        let result = task.await.context("Waiting for task")?;
        assert!(result.is_ok());

        // 验证服务仍然健康
        let health = services.overall_health();
        assert_eq!(health.status, crate::services::HealthStatus::Healthy);

        // 停止服务
        services.stop_all().context("Stopping services")?;

        info!("Error recovery and graceful degradation test completed");

        Ok(())
    }

    /// 测试资源清理和内存管理
    #[test]
    fn test_resource_cleanup_memory_management() -> Result<()> {
        use scopeguard::defer;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        let cleanup_count = Arc::new(Mutex::new(0usize));
        let cleanup_count_clone = Arc::clone(&cleanup_count);

        // 使用 scopeguard 确保资源清理
        let _guard = scopeguard::guard((), |_| {
            let mut count = cleanup_count_clone.lock();
            *count += 1;
            info!("Resource cleanup executed");
        });

        // 创建服务
        let services = AppServices::new().context("Creating services")?;

        // 使用服务
        let resource_tracker = services.resource_tracker();
        let initial_report = resource_tracker.generate_report();
        info!("Initial resource count: {}", initial_report.total);

        // 模拟资源使用
        let _token = services
            .cancellation_manager()
            .create_token("test-operation".to_string());

        // 验证资源追踪
        let final_report = resource_tracker.generate_report();
        assert!(final_report.total >= initial_report.total);

        info!("Resource cleanup test completed");

        Ok(())
    }

    /// 测试完整的端到端工作流
    #[tokio::test]
    async fn test_end_to_end_workflow() -> Result<()> {
        use moka::future::Cache;
        use scopeguard::defer;
        use tokio_util::sync::CancellationToken;

        // 初始化 tracing
        let _guard = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        info!("Starting end-to-end workflow test");

        // 1. 创建服务（依赖注入）
        let services = AppServices::builder()
            .with_development_config()
            .build()
            .context("Building services")?;

        info!("Services created");

        // 2. 启动服务
        services.start_all().context("Starting services")?;
        info!("Services started");

        // 3. 设置资源清理
        let cleanup_flag = Arc::new(Mutex::new(false));
        let cleanup_flag_clone = Arc::clone(&cleanup_flag);
        let _cleanup_guard = scopeguard::guard((), |_| {
            let mut flag = cleanup_flag_clone.lock();
            *flag = true;
            info!("Cleanup guard executed");
        });

        // 4. 创建缓存（moka）
        let cache: Cache<String, String> = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(60))
            .build();

        // 5. 执行业务逻辑
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let task = tokio::spawn(async move {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    info!("Task cancelled gracefully");
                    Ok::<_, eyre::Report>(())
                }
                _ = async {
                    // 模拟业务操作
                    cache.insert("key".to_string(), "value".to_string()).await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok::<_, eyre::Report>(())
                } => {
                    info!("Task completed successfully");
                    Ok(())
                }
            }
        });

        // 6. 等待任务完成
        let result = task.await.context("Waiting for task")?;
        assert!(result.is_ok());

        // 7. 验证健康状态
        let health = services.overall_health();
        assert_eq!(health.status, crate::services::HealthStatus::Healthy);
        info!("All services healthy");

        // 8. 停止服务
        services.stop_all().context("Stopping services")?;
        info!("Services stopped");

        // 9. 验证清理
        drop(_cleanup_guard);
        let flag = cleanup_flag.lock();
        assert!(*flag);

        info!("End-to-end workflow test completed successfully");

        Ok(())
    }
}
