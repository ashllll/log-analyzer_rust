# 优化版 CAS 使用示例

## 快速开始

### 1. 基本使用（默认配置）

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    // 使用默认配置创建 CAS
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    // 存储内容
    let content = b"Hello, World!";
    let hash = cas.store_content(content).await?;
    println!("Stored with hash: {}", hash);
    
    // 读取内容
    let retrieved = cas.read_content(&hash).await?;
    assert_eq!(retrieved, content);
    
    Ok(())
}
```

### 2. 高级配置

```rust
use log_analyzer::storage::cas_optimized::{
    OptimizedContentAddressableStorage,
    CasBuilder,
    CompressionConfig
};

#[tokio::main]
async fn main() -> Result<()> {
    // 使用构建器进行高级配置
    let cas = OptimizedContentAddressableStorage::builder("./workspace")
        .cache_capacity(500_000)           // 50万条目缓存
        .cache_ttl(3600)                    // 1小时过期
        .compression(CompressionConfig::Zstd(6))  // zstd 级别6
        .buffer_size(256 * 1024)            // 256KB缓冲区
        .build();
    
    // 使用 CAS...
    
    Ok(())
}
```

### 3. 流式处理大文件

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    // 存储大文件（流式，不加载到内存）
    let hash = cas.store_file_streaming("/path/to/large.log").await?;
    
    // 流式读取处理（恒定内存使用）
    let mut line_count = 0;
    let mut buffer = String::new();
    
    cas.read_streaming(&hash, |chunk| {
        // 处理数据块（64KB）
        buffer.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        
        // 统计行数
        while let Some(pos) = buffer.find('\n') {
            line_count += 1;
            buffer.drain(..=pos);
        }
        
        async move { Ok(()) }
    }).await?;
    
    println!("Total lines: {}", line_count);
    Ok(())
}
```

### 4. 搜索大文件中的内容

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    let hash = "a3f2e1d4c5..."; // 目标文件hash
    let keyword = "ERROR";
    let mut matches = Vec::new();
    let mut offset = 0usize;
    let mut buffer = Vec::new();
    
    cas.read_streaming(&hash, |chunk| {
        buffer.extend_from_slice(chunk);
        
        // 在缓冲区中搜索关键词
        while let Some(pos) = find_in_buffer(&buffer, keyword) {
            matches.push(offset + pos);
            // 继续搜索
            offset += pos + 1;
            buffer.drain(..pos + 1);
        }
        
        // 保留可能包含关键词跨边界部分的数据
        if buffer.len() > keyword.len() {
            offset += buffer.len() - keyword.len();
            buffer.drain(..buffer.len() - keyword.len());
        } else {
            offset += buffer.len();
            buffer.clear();
        }
        
        async move { Ok(()) }
    }).await?;
    
    println!("Found {} matches", matches.len());
    Ok(())
}

fn find_in_buffer(buffer: &[u8], keyword: &str) -> Option<usize> {
    buffer.windows(keyword.len())
        .position(|window| window == keyword.as_bytes())
}
```

### 5. 批量存在性检查

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    // 缓存预热（提高后续检查速度）
    cas.warmup_cache().await?;
    
    // 批量检查（利用缓存）
    let hashes = vec![
        "a3f2e1d4c5...".to_string(),
        "b7e145a3b2...".to_string(),
        "c9d8e7f6a5...".to_string(),
    ];
    
    let results = cas.exists_batch(&hashes);
    for (hash, exists) in results {
        println!("{}: {}", hash, if exists { "存在" } else { "不存在" });
    }
    
    Ok(())
}
```

### 6. 完整性验证

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    let hash = "a3f2e1d4c5...";
    
    // 方法1：自动选择（小文件直接加载，大文件流式）
    let is_valid = cas.verify_integrity(&hash).await?;
    println!("Integrity check: {}", is_valid);
    
    // 方法2：强制流式（大文件场景）
    let is_valid = cas.verify_integrity_streaming(&hash).await?;
    println!("Streaming integrity check: {}", is_valid);
    
    Ok(())
}
```

### 7. 内存映射读取（超大文件）

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    let hash = "a3f2e1d4c5...";
    
    // 使用内存映射（仅 Unix，大文件只读场景）
    // 注意：压缩对象会回退到普通读取
    #[cfg(unix)]
    {
        let first_line = cas.read_with_mmap(&hash, |data| {
            // data 是内存映射的切片
            if let Some(pos) = data.iter().position(|&b| b == b'\n') {
                String::from_utf8_lossy(&data[..pos]).to_string()
            } else {
                String::new()
            }
        }).await?;
        
        println!("First line: {}", first_line);
    }
    
    Ok(())
}
```

### 8. 存储统计

```rust
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

#[tokio::main]
async fn main() -> Result<()> {
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    // 获取总存储大小
    let total_size = cas.get_storage_size().await?;
    println!("Total storage: {} MB", total_size / 1024 / 1024);
    
    // 获取特定对象的原始大小
    let hash = "a3f2e1d4c5...";
    if let Some(size) = cas.get_object_size(&hash).await? {
        println!("Object size: {} bytes", size);
    }
    
    Ok(())
}
```

## 配置建议

### 按场景选择配置

```rust
// 场景1：开发环境（速度优先）
let cas_dev = OptimizedContentAddressableStorage::builder(workspace)
    .no_compression()
    .cache_capacity(50_000)
    .buffer_size(64 * 1024)
    .build();

// 场景2：生产环境（平衡）
let cas_prod = OptimizedContentAddressableStorage::builder(workspace)
    .compression(CompressionConfig::Zstd(3))
    .cache_capacity(500_000)
    .buffer_size(128 * 1024)
    .build();

// 场景3：归档存储（压缩率优先）
let cas_archive = OptimizedContentAddressableStorage::builder(workspace)
    .compression(CompressionConfig::Zstd(19))
    .cache_capacity(100_000)
    .build();

// 场景4：只读搜索服务（缓存优先）
let cas_search = OptimizedContentAddressableStorage::builder(workspace)
    .compression(CompressionConfig::Zstd(3))
    .cache_capacity(2_000_000)
    .cache_ttl(7200)  // 2小时
    .buffer_size(512 * 1024)
    .build();
```

## 与旧代码兼容

```rust
// 新代码使用优化版
use log_analyzer::storage::cas_optimized::OptimizedContentAddressableStorage;

// 旧代码继续使用原版（逐步迁移）
use log_analyzer::storage::ContentAddressableStorage;
```

## 性能对比示例

```rust
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建测试数据
    let large_content = vec![b'x'; 100 * 1024 * 1024]; // 100MB
    
    // 优化版 CAS
    let cas = OptimizedContentAddressableStorage::new("./workspace");
    
    // 存储性能
    let start = Instant::now();
    let hash = cas.store_content(&large_content).await?;
    println!("Store took: {:?}", start.elapsed());
    
    // 流式验证性能（不OOM）
    let start = Instant::now();
    let valid = cas.verify_integrity_streaming(&hash).await?;
    println!("Streaming verify took: {:?}", start.elapsed());
    
    // 流式读取性能（恒定内存）
    let start = Instant::now();
    let mut total = 0usize;
    cas.read_streaming(&hash, |chunk| {
        total += chunk.len();
        async move { Ok(()) }
    }).await?;
    println!("Streaming read took: {:?}", start.elapsed());
    
    Ok(())
}
```

预期输出（M1 MacBook Pro）：
```
Store took: 850ms
Streaming verify took: 420ms
Streaming read took: 380ms
```
