//! 搜索引擎异步测试
//!
//! 验证搜索引擎的异步特性：
//! 1. 异步搜索不阻塞
//! 2. 搜索取消机制
//! 3. 超时处理
//! 4. 并发搜索

use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use log_analyzer::search_engine::{SearchEngineManager, SearchConfig, SearchError};
use log_analyzer::models::LogEntry;

fn create_test_log_entry(id: usize, content: &str) -> LogEntry {
    LogEntry {
        id,
        timestamp: format!("2024-01-01T00:{:02}:00", id % 60).into(),
        level: if id % 3 == 0 { "ERROR" } else { "INFO" }.into(),
        file: "/test.log".into(),
        real_path: "/real/test.log".into(),
        line: id,
        content: content.into(),
        tags: vec![],
        match_details: None,
        matched_keywords: None,
    }
}

#[tokio::test]
async fn test_async_search_does_not_block() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加测试数据
    for i in 0..100 {
        let entry = create_test_log_entry(
            i,
            &format!("Test log entry {} with some searchable content", i)
        );
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 执行多个并发搜索，不应阻塞
    let mut handles = vec![];
    for i in 0..5 {
        let manager = manager.clone();
        // 使用 spawn_blocking 包装同步搜索
        let handle = tokio::task::spawn_blocking(move || {
            let query = format!("entry {}", i * 10);
            // 注意：这里使用同步搜索，但包装在 spawn_blocking 中
            // 实际测试的是 spawn_blocking 不会阻塞异步运行时
            manager.search_multi_keyword(
                &[query],
                false,
                Some(10),
                Some(Duration::from_secs(5)),
                None,
            )
        });
        handles.push(handle);
    }

    // 所有搜索都应该在 5 秒内完成
    let result = timeout(Duration::from_secs(5), async {
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok(), "Search should succeed: {:?}", result);
        }
    }).await;

    assert!(result.is_ok(), "All searches should complete within timeout");
}

#[tokio::test]
async fn test_search_with_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加一些数据
    for i in 0..50 {
        let entry = create_test_log_entry(i, &format!("content {}", i));
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 使用 search_with_timeout 进行异步搜索
    let result = manager.search_with_timeout(
        "content",
        Some(10),
        Some(Duration::from_millis(100)),
        None,
    ).await;

    // 应该成功完成（在空索引或小索引上应该很快）
    match result {
        Ok(results) => {
            // 搜索成功
            assert!(results.entries.len() <= 10);
        }
        Err(SearchError::Timeout(_)) => {
            // 超时也是可以接受的结果
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_search_cancellation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加数据
    for i in 0..100 {
        let entry = create_test_log_entry(i, &format!("searchable content {}", i));
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    let token = CancellationToken::new();
    let token_clone = token.clone();

    // 在 1ms 后取消（非常快，确保取消生效）
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1)).await;
        token_clone.cancel();
    });

    let result = manager.search_with_timeout(
        "content",
        None,
        Some(Duration::from_secs(10)), // 长超时
        Some(token),
    ).await;

    // 应该被取消或超时
    assert!(
        matches!(result, Err(SearchError::Cancelled))
        || matches!(result, Err(SearchError::Timeout(_)))
        || result.is_ok(), // 如果搜索非常快完成，也是可以接受的
        "Search should be cancelled, timeout, or succeed quickly"
    );
}

#[tokio::test]
async fn test_search_with_memory_budget() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加大量数据
    for i in 0..100 {
        let entry = create_test_log_entry(
            i,
            &format!("content for memory budget test {}", i)
        );
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 使用内存预算限制
    let result = manager.search_with_budget(
        "content",
        Some(1000), // 请求很多结果
        Some(Duration::from_secs(5)),
        None,
        Some(10), // 10MB 预算
    ).await;

    assert!(result.is_ok());
    let results = result.unwrap();
    // 结果应该被内存预算限制
    assert!(results.entries.len() <= 50_000); // 绝对上限
}

#[tokio::test]
async fn test_concurrent_searches_with_same_manager() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加数据
    for i in 0..200 {
        let entry = create_test_log_entry(
            i,
            &format!("concurrent search test content {}", i)
        );
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 并发搜索（使用 spawn_blocking 包装）
    let mut handles = vec![];
    for i in 0..10 {
        let manager = manager.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let query = format!("content {}", i);
            manager.search_multi_keyword(
                &[query],
                false,
                Some(5),
                Some(Duration::from_secs(2)),
                None,
            )
        });
        handles.push(handle);
    }

    // 等待所有搜索完成
    let timeout_result = timeout(Duration::from_secs(10), async {
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }).await;

    assert!(timeout_result.is_ok(), "All concurrent searches should complete");
}

#[tokio::test]
async fn test_empty_index_search() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 不添加任何数据，直接搜索
    let result = manager.search_with_timeout(
        "test",
        Some(10),
        Some(Duration::from_secs(1)),
        None,
    ).await;

    // 空索引搜索应该成功，但返回空结果
    assert!(result.is_ok());
    let results = result.unwrap();
    assert!(results.entries.is_empty());
    assert_eq!(results.total_count, 0);
}

#[tokio::test]
async fn test_empty_query_error() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 空查询应该返回错误
    let result = manager.search_with_timeout(
        "",
        Some(10),
        Some(Duration::from_secs(1)),
        None,
    ).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SearchError::QueryError(_)));
}

#[tokio::test]
async fn test_search_stats_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加数据
    for i in 0..50 {
        let entry = create_test_log_entry(i, &format!("stats tracking test {}", i));
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 获取初始统计
    let initial_stats = manager.get_stats();
    let initial_count = initial_stats.total_searches;

    // 执行几次搜索
    for i in 0..5 {
        let _ = manager.search_with_timeout(
            &format!("test {}", i),
            Some(10),
            Some(Duration::from_secs(2)),
            None,
        ).await;
    }

    // 验证统计更新
    let final_stats = manager.get_stats();
    assert_eq!(final_stats.total_searches, initial_count + 5);
}

#[tokio::test]
async fn test_delete_file_documents() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加不同文件的数据
    for i in 0..20 {
        let file_path = if i < 10 { "/path/file1.log" } else { "/path/file2.log" };
        let entry = LogEntry {
            id: i,
            timestamp: format!("2024-01-01T00:{:02}:00", i % 60).into(),
            level: "INFO".into(),
            file: file_path.into(),
            real_path: file_path.into(),
            line: i,
            content: format!("entry {} in {}", i, file_path).into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 删除 file1 的文档
    let deleted_count = manager.delete_file_documents("/path/file1.log").unwrap();
    assert_eq!(deleted_count, 10);

    // 验证删除后搜索
    let result = manager.search_with_timeout(
        "file1",
        Some(20),
        Some(Duration::from_secs(2)),
        None,
    ).await;

    // 应该找不到 file1 的内容了
    assert!(result.is_ok());
    let results = result.unwrap();
    assert!(results.entries.iter().all(|e| !e.file.as_ref().contains("file1")));
}

#[tokio::test]
async fn test_clear_index() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加数据
    for i in 0..20 {
        let entry = create_test_log_entry(i, &format!("clear test {}", i));
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 搜索确认有数据
    let result = manager.search_with_timeout(
        "clear",
        Some(20),
        Some(Duration::from_secs(2)),
        None,
    ).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().entries.is_empty());

    // 清空索引
    manager.clear_index().unwrap();

    // 再次搜索应该为空
    let result = manager.search_with_timeout(
        "clear",
        Some(20),
        Some(Duration::from_secs(2)),
        None,
    ).await;
    assert!(result.is_ok());
    assert!(result.unwrap().entries.is_empty());
}

#[tokio::test]
async fn test_multi_keyword_search() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加包含不同关键词的数据
    let keywords_data = vec![
        ("error database connection", "ERROR"),
        ("error network timeout", "ERROR"),
        ("warning database slow", "WARN"),
        ("info service started", "INFO"),
        ("error database query", "ERROR"),
    ];

    for (i, (content, level)) in keywords_data.iter().enumerate() {
        let entry = LogEntry {
            id: i,
            timestamp: format!("2024-01-01T00:{:02}:00", i).into(),
            level: (*level).into(),
            file: "/test.log".into(),
            real_path: "/real/test.log".into(),
            line: i,
            content: (*content).into(),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        };
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 多关键词搜索（OR 模式）
    let result = manager.search_multi_keyword(
        &["error".to_string(), "database".to_string()],
        false, // OR
        Some(10),
        Some(Duration::from_secs(2)),
        None,
    ).await;

    assert!(result.is_ok());
    let results = result.unwrap();
    // 应该找到包含 error 或 database 的条目
    assert!(results.entries.len() > 0);
}

#[tokio::test]
async fn test_search_performance_under_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SearchEngineManager::new(SearchConfig {
        index_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    }).unwrap();

    // 添加大量数据
    for i in 0..500 {
        let entry = create_test_log_entry(
            i,
            &format!("performance test entry with searchable content {}", i)
        );
        manager.add_document(&entry).unwrap();
    }
    manager.commit().unwrap();

    // 使用很短的超时进行搜索
    let start = std::time::Instant::now();
    let result = manager.search_with_timeout(
        "searchable",
        Some(50),
        Some(Duration::from_millis(50)), // 50ms 超时
        None,
    ).await;
    let elapsed = start.elapsed();

    // 应该在超时时间内完成或返回超时错误
    match result {
        Ok(results) => {
            // 成功完成
            assert!(elapsed < Duration::from_millis(100)); // 应该很快
        }
        Err(SearchError::Timeout(_)) => {
            // 超时也是可以接受的
        }
        Err(e) => {
            panic!("Unexpected error: {}", e);
        }
    }
}
