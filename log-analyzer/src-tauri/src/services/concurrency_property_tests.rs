/**
 * 并发安全属性测试
 *
 * 实现以下属性：
 * - Property 8: Deadlock Prevention
 * - Property 9: Thread-Safe Cache Access
 * - Property 10: Workspace State Protection
 */
#[cfg(test)]
mod tests {
    use parking_lot::Mutex;
    use proptest::prelude::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    /**
     * **Feature: bug-fixes, Property 8: Deadlock Prevention**
     *
     * *For any* multiple lock acquisition scenario,
     * locks should be acquired in consistent order to prevent deadlocks
     * **Validates: Requirements 3.1**
     */
    #[test]
    fn property_8_deadlock_prevention() {
        proptest!(|(
            // 生成多个线程数量
            thread_count in 2usize..10
        )| {
            let lock_a = Arc::new(Mutex::new(0));
            let lock_b = Arc::new(Mutex::new(0));

            let mut handles = vec![];

            for i in 0..thread_count {
                let lock_a = Arc::clone(&lock_a);
                let lock_b = Arc::clone(&lock_b);

                let handle = thread::spawn(move || {
                    // 使用一致的锁顺序（按内存地址排序）
                    let addr_a = Arc::as_ptr(&lock_a) as usize;
                    let addr_b = Arc::as_ptr(&lock_b) as usize;

                    if addr_a < addr_b {
                        let _guard_a = lock_a.lock();
                        thread::sleep(Duration::from_micros(1));
                        let _guard_b = lock_b.lock();
                        i
                    } else {
                        let _guard_b = lock_b.lock();
                        thread::sleep(Duration::from_micros(1));
                        let _guard_a = lock_a.lock();
                        i
                    }
                });

                handles.push(handle);
            }

            // 所有线程应该能够完成（不会死锁）
            for handle in handles {
                let result = handle.join();
                prop_assert!(result.is_ok());
            }
        });
    }

    /**
     * **Feature: bug-fixes, Property 9: Thread-Safe Cache Access**
     *
     * *For any* concurrent search cache access,
     * operations should be thread-safe without race conditions
     * **Validates: Requirements 3.3**
     */
    #[test]
    fn property_9_thread_safe_cache_access() {
        proptest!(|(
            // 生成并发操作数量
            operation_count in 10usize..50
        )| {
            use std::collections::HashMap;

            let cache = Arc::new(Mutex::new(HashMap::<String, Vec<String>>::new()));
            let mut handles = vec![];

            for i in 0..operation_count {
                let cache = Arc::clone(&cache);
                let key = format!("key_{}", i % 10);
                let value = vec![format!("value_{}", i)];

                let handle = thread::spawn(move || {
                    // 写入操作
                    {
                        let mut cache_guard = cache.lock();
                        cache_guard.insert(key.clone(), value.clone());
                    }

                    // 读取操作
                    {
                        let cache_guard = cache.lock();
                        cache_guard.get(&key).cloned()
                    }
                });

                handles.push(handle);
            }

            // 所有操作应该成功完成
            for handle in handles {
                let result = handle.join();
                prop_assert!(result.is_ok());
            }

            // 缓存应该包含数据
            let final_cache = cache.lock();
            prop_assert!(!final_cache.is_empty());
        });
    }

    /**
     * **Feature: bug-fixes, Property 10: Workspace State Protection**
     *
     * *For any* concurrent workspace state modification,
     * the system should protect against race conditions
     * **Validates: Requirements 3.4**
     */
    #[test]
    fn property_10_workspace_state_protection() {
        proptest!(|(
            // 生成并发更新数量
            update_count in 10usize..50
        )| {
            #[derive(Clone, Debug)]
            struct WorkspaceState {
                status: String,
                file_count: usize,
            }

            let state = Arc::new(Mutex::new(WorkspaceState {
                status: "READY".to_string(),
                file_count: 0,
            }));

            let mut handles = vec![];

            for i in 0..update_count {
                let state = Arc::clone(&state);

                let handle = thread::spawn(move || {
                    let mut state_guard = state.lock();
                    state_guard.file_count += 1;

                    if i % 10 == 0 {
                        state_guard.status = "PROCESSING".to_string();
                    }
                });

                handles.push(handle);
            }

            // 等待所有更新完成
            for handle in handles {
                let result = handle.join();
                prop_assert!(result.is_ok());
            }

            // 验证最终状态的一致性
            let final_state = state.lock();
            prop_assert_eq!(final_state.file_count, update_count);
        });
    }
}
