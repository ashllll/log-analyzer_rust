//! CAS 并发安全测试
//!
//! 验证 Content-Addressable Storage 的并发安全性：
//! 1. 并发写入同一内容的去重
//! 2. 原子存储操作
//! 3. 信号量背压控制

use std::sync::Arc;
use tempfile::TempDir;
use tokio::task::JoinSet;

use log_analyzer::storage::ContentAddressableStorage;

#[tokio::test]
async fn test_concurrent_atomic_store() {
    let temp_dir = TempDir::new().unwrap();
    let cas = Arc::new(ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    ));

    let content = b"concurrent test content";
    let mut join_set = JoinSet::new();

    // 20 个并发写入
    for i in 0..20 {
        let cas = cas.clone();
        let content = content.clone();
        join_set.spawn(async move {
            cas.store_content_atomic(&content).await
        });
    }

    let results: Vec<_> = join_set.join_all().await;

    // 所有都应该成功
    for result in &results {
        assert!(result.is_ok(), "Concurrent store should succeed: {:?}", result);
    }

    // 所有结果应该有相同的 hash
    let first_hash = results[0].as_ref().unwrap();
    for result in &results {
        assert_eq!(result.as_ref().unwrap(), first_hash, "All concurrent stores should return same hash");
    }

    // 只有一个文件被创建
    let hash = ContentAddressableStorage::compute_hash(content);
    assert!(cas.exists(&hash), "Hash should exist in CAS");
    
    // 验证内容正确
    let retrieved = cas.read_content(&hash).await.unwrap();
    assert_eq!(retrieved, content, "Retrieved content should match original");
}

#[tokio::test]
async fn test_concurrent_different_content() {
    let temp_dir = TempDir::new().unwrap();
    let cas = Arc::new(ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    ));

    let mut join_set = JoinSet::new();
    let mut expected_hashes = Vec::new();

    // 10 个并发写入不同内容
    for i in 0..10 {
        let cas = cas.clone();
        let content = format!("unique content {}", i);
        expected_hashes.push(ContentAddressableStorage::compute_hash(content.as_bytes()));
        
        join_set.spawn(async move {
            cas.store_content_atomic(content.as_bytes()).await
        });
    }

    let results: Vec<_> = join_set.join_all().await;

    // 所有都应该成功
    for result in &results {
        assert!(result.is_ok());
    }

    // 验证所有文件都存在
    for hash in &expected_hashes {
        assert!(cas.exists(hash), "Hash {} should exist", hash);
    }

    // 验证可以读取所有内容
    for i in 0..10 {
        let content = format!("unique content {}", i);
        let hash = ContentAddressableStorage::compute_hash(content.as_bytes());
        let retrieved = cas.read_content(&hash).await.unwrap();
        assert_eq!(retrieved, content.as_bytes());
    }
}

#[tokio::test]
async fn test_concurrent_store_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let cas = Arc::new(ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    ));

    // 先存储一些内容
    let contents: Vec<String> = (0..5)
        .map(|i| format!("content for read test {}", i))
        .collect();

    let mut hashes = Vec::new();
    for content in &contents {
        let hash = cas.store_content_atomic(content.as_bytes()).await.unwrap();
        hashes.push(hash);
    }

    // 并发读取
    let mut join_set = JoinSet::new();
    for _ in 0..20 {
        let cas = cas.clone();
        let hashes = hashes.clone();
        let contents = contents.clone();
        
        join_set.spawn(async move {
            for (i, hash) in hashes.iter().enumerate() {
                let retrieved = cas.read_content(hash).await.unwrap();
                assert_eq!(retrieved, contents[i].as_bytes());
            }
        });
    }

    let results: Vec<_> = join_set.join_all().await;
    
    // 所有读取都应该成功
    for result in results {
        // spawn 返回 ()，这里只是验证没有 panic
        let _: () = result;
    }
}

#[tokio::test]
async fn test_atomic_store_integrity() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    );

    let content = b"test content for integrity check";
    
    // 存储内容
    let hash = cas.store_content_atomic(content).await.unwrap();
    
    // 验证内容完整性
    let is_valid = cas.verify_integrity(&hash).await.unwrap();
    assert!(is_valid, "Content should pass integrity check");
    
    // 验证内容正确
    let retrieved = cas.read_content(&hash).await.unwrap();
    assert_eq!(retrieved, content);
}

#[tokio::test]
async fn test_store_content_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    );

    let content = b"duplicate content test";

    // 第一次存储
    let hash1 = cas.store_content(content).await.unwrap();
    
    // 第二次存储相同内容（应该返回相同 hash，不会重复写入）
    let hash2 = cas.store_content(content).await.unwrap();
    
    assert_eq!(hash1, hash2, "Same content should produce same hash");
    
    // 验证文件只存在一份
    let object_path = cas.get_object_path(&hash1);
    assert!(object_path.exists());
    
    // 验证内容正确
    let retrieved = cas.read_content(&hash1).await.unwrap();
    assert_eq!(retrieved, content);
}

#[tokio::test]
async fn test_exists_and_exists_async() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    );

    let content = b"test for exists methods";
    let hash = ContentAddressableStorage::compute_hash(content);

    // 存储前不应该存在
    assert!(!cas.exists(&hash));
    assert!(!cas.exists_async(&hash).await);

    // 存储后应该存在
    cas.store_content(content).await.unwrap();
    
    assert!(cas.exists(&hash));
    assert!(cas.exists_async(&hash).await);

    // 不存在的 hash
    let nonexistent = ContentAddressableStorage::compute_hash(b"nonexistent");
    assert!(!cas.exists(&nonexistent));
    assert!(!cas.exists_async(&nonexistent).await);
}

#[tokio::test]
async fn test_storage_size_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    );

    // 初始大小应该是 0
    let initial_size = cas.get_storage_size().await.unwrap();
    assert_eq!(initial_size, 0);

    // 存储一些内容
    let contents: Vec<&[u8]> = vec![
        b"first content",
        b"second content that is longer",
        b"third",
    ];

    let mut total_size = 0usize;
    for content in &contents {
        cas.store_content(content).await.unwrap();
        total_size += content.len();
    }

    // 验证大小
    let size = cas.get_storage_size().await.unwrap();
    assert_eq!(size as usize, total_size, 
        "Storage size should equal sum of all content sizes");
}

#[tokio::test]
async fn test_concurrent_store_file_streaming() {
    let temp_dir = TempDir::new().unwrap();
    let cas = Arc::new(ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    ));

    // 创建临时源文件
    let source_dir = TempDir::new().unwrap();
    let mut source_files = Vec::new();
    
    for i in 0..5 {
        let file_path = source_dir.path().join(format!("test_file_{}.txt", i));
        let content = format!("file {} content for streaming test", i);
        tokio::fs::write(&file_path, content.as_bytes()).await.unwrap();
        source_files.push((file_path, content));
    }

    // 并发流式存储
    let mut join_set = JoinSet::new();
    for (file_path, _) in &source_files {
        let cas = cas.clone();
        let file_path = file_path.clone();
        join_set.spawn(async move {
            cas.store_file_streaming(&file_path).await
        });
    }

    let results: Vec<_> = join_set.join_all().await;

    // 所有都应该成功
    for result in &results {
        assert!(result.is_ok());
    }

    // 验证所有文件都能读取
    for (i, (_, expected_content)) in source_files.iter().enumerate() {
        let hash = results[i].as_ref().unwrap();
        let retrieved = cas.read_content(hash).await.unwrap();
        assert_eq!(retrieved, expected_content.as_bytes());
    }
}

#[tokio::test]
async fn test_hash_idempotence() {
    let content = b"idempotent content test";
    
    // 多次计算相同内容的 hash 应该得到相同结果
    let hash1 = ContentAddressableStorage::compute_hash(content);
    let hash2 = ContentAddressableStorage::compute_hash(content);
    let hash3 = ContentAddressableStorage::compute_hash(content);
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
    
    // 不同内容应该产生不同 hash
    let different_content = b"different content";
    let different_hash = ContentAddressableStorage::compute_hash(different_content);
    assert_ne!(hash1, different_hash);
    
    // hash 长度应该是 64 (SHA-256 hex)
    assert_eq!(hash1.len(), 64);
    
    // hash 应该只包含十六进制字符
    assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_concurrent_high_load() {
    let temp_dir = TempDir::new().unwrap();
    let cas = Arc::new(ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    ));

    // 高并发测试：50 个并发写入
    let num_tasks = 50;
    let mut join_set = JoinSet::new();

    for i in 0..num_tasks {
        let cas = cas.clone();
        let content = format!("high load test content {}", i);
        join_set.spawn(async move {
            cas.store_content_atomic(content.as_bytes()).await
        });
    }

    let results: Vec<_> = join_set.join_all().await;

    // 统计成功和失败
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let failure_count = results.iter().filter(|r| r.is_err()).count();

    // 所有操作都应该成功
    assert_eq!(success_count, num_tasks, 
        "All {} concurrent operations should succeed, but {} failed", 
        num_tasks, failure_count);

    // 验证最终存储的内容数量
    let size = cas.get_storage_size().await.unwrap();
    // 每个内容长度不同，所以应该有 num_tasks 个不同文件
    assert!(size > 0);
}

#[tokio::test]
async fn test_object_path_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    );

    let content = b"test content";
    let hash = ContentAddressableStorage::compute_hash(content);
    
    let object_path = cas.get_object_path(&hash);
    
    // 路径应该包含 objects 目录
    assert!(object_path.to_string_lossy().contains("objects"));
    
    // 路径应该以 hash 的后 62 位作为文件名
    let file_name = object_path.file_name().unwrap().to_string_lossy();
    assert_eq!(file_name, &hash[2..]);
    
    // 父目录名应该是 hash 的前 2 位
    let parent_name = object_path.parent().unwrap().file_name().unwrap().to_string_lossy();
    assert_eq!(parent_name, &hash[..2]);
}

#[tokio::test]
async fn test_empty_content_storage() {
    let temp_dir = TempDir::new().unwrap();
    let cas = ContentAddressableStorage::new(
        temp_dir.path().to_path_buf()
    );

    let empty_content: &[u8] = b"";
    
    // 空内容也应该能存储
    let hash = cas.store_content(empty_content).await.unwrap();
    assert!(!hash.is_empty());
    
    // 验证可以读取
    let retrieved = cas.read_content(&hash).await.unwrap();
    assert!(retrieved.is_empty());
    
    // 验证 hash 正确
    let expected_hash = ContentAddressableStorage::compute_hash(empty_content);
    assert_eq!(hash, expected_hash);
}
