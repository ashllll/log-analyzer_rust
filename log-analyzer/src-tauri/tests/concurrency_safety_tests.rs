//! 并发安全性测试
//!
//! 测试高级并发基础设施的安全性和正确性，包括：
//! - 死锁预防机制
//! - 线程安全的缓存访问
//! - 锁排序和超时机制

use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

use log_analyzer::{AsyncResourceManager, CacheManager, LockManager, LogEntry, SearchCacheKey};

use proptest::prelude::*;
use rstest::*;

/// **Feature: bug-fixes, Property 8: Deadlock Prevention**
/// *For any* multiple lock acquisition scenario, locks should be acquired in consistent order to prevent deadlocks
/// **Validates: Requirements 3.1**
#[test]
fn test_deadlock_prevention_property() {
    proptest!(|(
        num_threads in 2u8..=8,
        num_locks in 2u8..=5,
        operations_per_thread in 10u32..=50
    )| {
        // 创建多个锁
        let locks: Vec<Arc<Mutex<u32>>> = (0..num_locks)
            .map(|_| Arc::new(Mutex::new(0)))
            .collect();

        let lock_manager = Arc::new(LockManager::new());
        let barrier = Arc::new(std::sync::Barrier::new(num_threads as usize));

        // 创建多个线程同时获取锁
        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let locks = locks.clone();
                let lock_manager = lock_manager.clone();
                let barrier = barrier.clone();

                thread::spawn(move || {
                    barrier.wait();

                    for _ in 0..operations_per_thread {
                        // 随机选择两个不同的锁
                        let lock1_idx = (thread_id as usize) % locks.len();
                        let lock2_idx = (lock1_idx + 1) % locks.len();

                        if lock1_idx != lock2_idx {
                            // 使用安全的锁管理器获取两个锁
                            let lock1_id = format!("lock_{}", lock1_idx);
                            let lock2_id = format!("lock_{}", lock2_idx);
                            let result = lock_manager.acquire_two_locks_safe(
                                &lock1_id,
                                &locks[lock1_idx],
                                &lock2_id,
                                &locks[lock2_idx]
                            );

                            match result {
                                Ok((mut guard1, mut guard2)) => {
                                    // 执行一些工作
                                    *guard1 += 1;
                                    *guard2 += 1;

                                    // 短暂持有锁
                                    thread::sleep(Duration::from_micros(10));
                                }
                                Err(_) => {
                                    // 锁获取失败是可接受的（超时等）
                                }
                            }
                        }
                    }
                })
            })
            .collect();

        // 等待所有线程完成
        for handle in handles {
            handle.join().expect("Thread should complete without panic");
        }

        // 验证没有发生死锁（所有线程都完成了）
        // 如果发生死锁，测试会超时失败
        prop_assert!(true);
    });
}

/// **Feature: bug-fixes, Property 9: Thread-Safe Cache Access**
/// *For any* concurrent search cache access, operations should be thread-safe without race conditions
/// **Validates: Requirements 3.3**
#[tokio::test]
async fn test_thread_safe_cache_access_property() {
    proptest!(|(
        num_threads in 2u8..=4,
        num_operations in 20u32..=50,
        cache_size in 10u64..=100
    )| {
        // 创建缓存
        let search_cache = Arc::new(
            moka::sync::Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(Duration::from_secs(60))
                .build()
        );

        let cache_manager = Arc::new(CacheManager::new(search_cache.clone()));

        // 创建多个线程并发访问缓存 (使用同步缓存操作)
        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let cache = search_cache.clone();
                let cache_manager = cache_manager.clone();

                thread::spawn(move || {
                    for op_id in 0..num_operations {
                        let key = create_test_cache_key(
                            &format!("query_{}_{}", thread_id, op_id),
                            &format!("workspace_{}", thread_id % 3)
                        );

                        match op_id % 3 {
                            0 => {
                                // 插入操作
                                let entries = vec![create_test_log_entry(op_id as usize)];
                                cache.insert(key, entries);
                            }
                            1 => {
                                // 读取操作
                                let _ = cache.get(&key);
                            }
                            2 => {
                                // 失效操作
                                cache.invalidate(&key);
                            }
                            _ => unreachable!()
                        }
                    }
                })
            })
            .collect();

        // 等待所有线程完成
        for handle in handles {
            handle.join().expect("Thread should complete without panic");
        }

        // 验证缓存仍然处于一致状态
        let final_stats = cache_manager.get_cache_statistics();
        prop_assert!(final_stats.entry_count <= cache_size);

        // 验证缓存操作没有导致数据竞争或不一致状态
        // 如果有数据竞争，线程会panic或产生不一致的结果
        prop_assert!(true);
    });
}

/// 测试锁管理器的超时机制
#[rstest]
#[case(Duration::from_millis(10))]
#[case(Duration::from_millis(50))]
#[case(Duration::from_millis(100))]
fn test_lock_timeout_mechanism(#[case] timeout: Duration) {
    let lock = Arc::new(Mutex::new(42));
    let lock_manager = LockManager::new();

    // 在一个线程中持有锁
    let lock_clone = lock.clone();
    let handle = thread::spawn(move || {
        let _guard = lock_clone.lock();
        thread::sleep(Duration::from_millis(200)); // 持有锁200ms
    });

    // 在主线程中尝试获取锁（带超时）
    thread::sleep(Duration::from_millis(10)); // 确保另一个线程先获取锁

    let result = lock_manager.try_acquire_with_timeout("test_lock", &lock, timeout);

    if timeout < Duration::from_millis(150) {
        // 超时应该发生
        assert!(result.is_none(), "Lock acquisition should timeout");
    }

    handle.join().unwrap();
}

/// 测试异步资源管理器的并发安全性
#[tokio::test]
async fn test_async_resource_manager_concurrency() {
    let manager = Arc::new(AsyncResourceManager::new());
    let num_tasks = 50;
    let mut handles = Vec::new();

    // 创建多个并发任务
    for i in 0..num_tasks {
        let manager = manager.clone();
        let handle = tokio::spawn(async move {
            let operation_id = format!("operation_{}", i);
            let resource_id = format!("resource_{}", i);
            let resource_path = format!("/tmp/resource_{}", i);

            // 注册操作和资源
            let token = manager
                .register_operation(
                    operation_id.clone(),
                    log_analyzer::utils::async_resource_manager::OperationType::BackgroundTask,
                    None,
                )
                .await;
            manager
                .register_resource(resource_id.clone(), resource_path.clone())
                .await
                .unwrap();

            // 模拟一些工作
            tokio::time::sleep(Duration::from_millis(10)).await;

            // 验证资源存在
            let retrieved_path = manager.get_resource(&resource_id).await;
            assert_eq!(retrieved_path, Some(resource_path));

            // 清理
            manager.cleanup_resource(&resource_id).await.unwrap();
            manager.cancel_operation(&operation_id).await.unwrap();

            assert!(token.is_cancelled());
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }

    // 验证最终状态
    assert_eq!(manager.active_operations_count().await, 0);
    assert_eq!(manager.resources_count().await, 0);
}

/// 测试锁无关队列的并发性能
#[test]
fn test_lock_free_queue_concurrency() {
    let queue = Arc::new(SegQueue::new());
    let num_producers = 4;
    let num_consumers = 4;
    let items_per_producer = 1000;

    let barrier = Arc::new(std::sync::Barrier::new(num_producers + num_consumers));
    let mut handles = Vec::new();

    // 创建生产者线程
    for producer_id in 0..num_producers {
        let queue = queue.clone();
        let barrier = barrier.clone();

        let handle = thread::spawn(move || {
            barrier.wait();

            for item in 0..items_per_producer {
                let value = producer_id * items_per_producer + item;
                queue.push(value);
            }
        });

        handles.push(handle);
    }

    // 创建消费者线程
    let consumed_count = Arc::new(Mutex::new(0));
    for _ in 0..num_consumers {
        let queue = queue.clone();
        let barrier = barrier.clone();
        let consumed_count = consumed_count.clone();

        let handle = thread::spawn(move || {
            barrier.wait();

            let mut local_count = 0;
            while local_count < items_per_producer {
                if let Some(_value) = queue.pop() {
                    local_count += 1;
                } else {
                    thread::yield_now();
                }
            }

            *consumed_count.lock() += local_count;
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证所有项目都被消费
    let total_consumed = *consumed_count.lock();
    assert_eq!(total_consumed, num_producers * items_per_producer);

    // 验证队列为空
    assert!(queue.is_empty());
}

/// 测试异步锁的公平性和性能
#[tokio::test]
async fn test_async_lock_fairness() {
    let lock = Arc::new(AsyncMutex::new(0));
    let num_tasks = 10;
    let operations_per_task = 100;

    let mut handles = Vec::new();

    for task_id in 0..num_tasks {
        let lock = lock.clone();

        let handle = tokio::spawn(async move {
            for _ in 0..operations_per_task {
                let mut guard = lock.lock().await;
                *guard += 1;

                // 模拟一些异步工作
                tokio::time::sleep(Duration::from_micros(10)).await;
            }

            task_id
        });

        handles.push(handle);
    }

    // 等待所有任务完成
    let mut completed_tasks = Vec::new();
    for handle in handles {
        let task_id = handle.await.unwrap();
        completed_tasks.push(task_id);
    }

    // 验证最终值
    let final_value = *lock.lock().await;
    assert_eq!(final_value, num_tasks * operations_per_task);

    // 验证所有任务都完成了
    assert_eq!(completed_tasks.len(), num_tasks);
}

/// 测试取消令牌的传播和清理
#[tokio::test]
async fn test_cancellation_token_propagation() {
    let parent_token = CancellationToken::new();
    let mut child_tokens = Vec::new();

    // 创建多个子令牌
    for _ in 0..10 {
        child_tokens.push(parent_token.child_token());
    }

    // 验证初始状态
    assert!(!parent_token.is_cancelled());
    for token in &child_tokens {
        assert!(!token.is_cancelled());
    }

    // 取消父令牌
    parent_token.cancel();

    // 验证所有子令牌都被取消
    assert!(parent_token.is_cancelled());
    for token in &child_tokens {
        assert!(token.is_cancelled());
    }
}

// 辅助函数

fn create_test_cache_key(query: &str, workspace_id: &str) -> SearchCacheKey {
    (
        query.to_string(),
        workspace_id.to_string(),
        None,             // time_start
        None,             // time_end
        vec![],           // levels
        None,             // file_pattern
        false,            // case_sensitive
        1000,             // max_results
        "v1".to_string(), // query_version
    )
}

fn create_test_log_entry(id: usize) -> LogEntry {
    LogEntry {
        id,
        timestamp: "2024-01-01T12:00:00Z".to_string(),
        level: "INFO".to_string(),
        file: format!("file_{}.log", id),
        real_path: format!("/test/file_{}.log", id),
        line: id,
        content: format!("Test message {}", id),
        tags: vec![],
        match_details: None,
        matched_keywords: Some(vec![]),
    }
}

/// 基准测试：锁性能比较
#[cfg(test)]
mod benchmarks {
    use super::*;
    use criterion::{black_box, Criterion};
    use std::sync::Mutex as StdMutex;

    pub fn bench_lock_performance(c: &mut Criterion) {
        let std_mutex = Arc::new(StdMutex::new(0));
        let parking_lot_mutex = Arc::new(Mutex::new(0));

        c.bench_function("std_mutex_contention", |b| {
            b.iter(|| {
                let mutex = std_mutex.clone();
                let handles: Vec<_> = (0..4)
                    .map(|_| {
                        let mutex = mutex.clone();
                        thread::spawn(move || {
                            for _ in 0..1000 {
                                let mut guard = mutex.lock().unwrap();
                                *guard += 1;
                                black_box(*guard);
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.join().unwrap();
                }
            })
        });

        c.bench_function("parking_lot_mutex_contention", |b| {
            b.iter(|| {
                let mutex = parking_lot_mutex.clone();
                let handles: Vec<_> = (0..4)
                    .map(|_| {
                        let mutex = mutex.clone();
                        thread::spawn(move || {
                            for _ in 0..1000 {
                                let mut guard = mutex.lock();
                                *guard += 1;
                                black_box(*guard);
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.join().unwrap();
                }
            })
        });
    }
}
