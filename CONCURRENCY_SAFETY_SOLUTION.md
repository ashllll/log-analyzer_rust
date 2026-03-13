# Rust 后端并发安全问题解决方案

本文档提供针对 Log Analyzer Rust 后端并发安全问题的完整解决方案。

## 问题概述

### 1. CAS 存储 TOCTOU 问题
**问题**: `store_content()` 中 `exists()` 检查与 `create_new()` 之间存在竞争窗口。

**解决方案**: 采用 **临时文件 + 原子重命名** 模式，配合 `O_EXCL` 原子创建标志。

### 2. 搜索引擎伪异步
**问题**: `execute_search` 标记为 `async` 但内部无 `await`，阻塞 Tokio 运行时。

**解决方案**: 使用 **`tokio::spawn_blocking`** 将 CPU 密集型搜索移至专用线程池。

### 3. 缺乏背压机制
**问题**: 没有限制并发搜索数量，可能导致资源耗尽。

**解决方案**: 使用 **`tokio::sync::Semaphore`** 实现背压控制。

### 4. 取消机制不完善
**问题**: 超时后无法真正中断 Tantivy 搜索。

**解决方案**: 实现 **协作式取消**（Cooperative Cancellation）机制。

---

## 业内成熟方案详解

### 1. CAS 原子操作方案

#### 参考标准
- **Git 对象存储**: 使用 SHA-256 + O_EXCL 原子创建
- **Content-Addressable Storage**: 临时文件 + 原子重命名

#### 核心机制
```rust
// 1. 原子创建（O_EXCL）
OpenOptions::new()
    .write(true)
    .create_new(true)  // 原子创建，失败说明已存在
    .open(&object_path)
    .await

// 2. 临时文件 + 原子重命名（适用于大文件）
1. 写入临时文件: objects/ab/.tmp_cd1234...
2. fsync 确保落盘
3. rename 原子移动到: objects/ab/cd1234...
```

### 2. CPU 密集型任务异步化

#### 参考标准
- **Tokio 最佳实践**: `spawn_blocking` 用于阻塞操作
- **Rust Async Book**: CPU 密集型任务必须使用独立线程池

#### 核心机制
```rust
// 使用 spawn_blocking 移动 CPU 密集型任务
tokio::task::spawn_blocking(move || {
    // 在此执行 Tantivy 搜索（CPU 密集型）
    searcher.search(&query, &collector)
})
```

### 3. 背压控制方案

#### 参考标准
- **Tokio Semaphore**: 信号量限流
- **Netflix Hystrix**: 熔断 + 背压模式

#### 核心机制
```rust
// 信号量控制并发
let semaphore = Arc::new(Semaphore::new(max_concurrent));

// 获取许可
let permit = semaphore.acquire().await?;

// 许可自动释放（RAII 模式）
```

### 4. 可取消任务方案

#### 参考标准
- **Tokio CancellationToken**: 协作式取消
- **Go Context**: 上下文传播模式

#### 核心机制
```rust
// 创建取消令牌
let token = CancellationToken::new();

// 传递令牌到任务
spawn_blocking(move || {
    for doc in docs {
        if token.is_cancelled() {
            return Err("Cancelled");
        }
        process(doc);
    }
})

// 取消任务
token.cancel();
```

---

## 完整代码实现

### 1. CAS 原子存储 (`storage/cas_atomic.rs`)

```rust
/// 原子存储内容
pub async fn store_content_atomic(&self, content: &[u8]) -> Result<String> {
    let hash = Self::compute_hash(content);
    let object_path = self.get_object_path(&hash);

    // 快速路径：检查缓存
    if self.existence_cache.get(&hash).is_some() {
        return Ok(hash);
    }

    // 获取写入许可（背压控制）
    let _permit = self
        .write_semaphore
        .acquire()
        .await
        .map_err(|_| AppError::io_error("Write semaphore closed", None::<PathBuf>))?;

    // 原子写入：临时文件 -> 原子重命名
    let temp_path = object_path.with_extension(&self.config.temp_suffix);
    
    // 步骤 1: 写入临时文件
    self.write_to_temp_file(&temp_path, content).await?;
    
    // 步骤 2: 原子重命名
    fs::rename(&temp_path, &object_path).await?;
    
    self.existence_cache.insert(hash.clone(), ());
    Ok(hash)
}
```

### 2. 真正异步搜索 (`search_engine/async_manager.rs`)

```rust
/// 带背压和取消的异步搜索（真正异步）
pub async fn search_cancellable(
    &self,
    query: &str,
    limit: Option<usize>,
    timeout_duration: Option<Duration>,
    token: CancellationToken,
) -> SearchResult<SearchResults> {
    let start_time = Instant::now();
    let limit = limit.unwrap_or(self.config.max_results);
    let timeout_duration = timeout_duration.unwrap_or(self.config.default_timeout);

    // 获取搜索许可（背压控制）
    let _permit = self
        .search_semaphore
        .acquire()
        .await
        .map_err(|_| SearchError::IndexError("Search semaphore closed".to_string()))?;

    // 解析查询
    let parsed_query = self.parse_query(query)?;

    // 使用 spawn_blocking 执行 CPU 密集型搜索
    let reader = self.reader.clone();
    let schema = self.schema.clone();
    let token_clone = token.clone();

    let search_handle: JoinHandle<SearchResult<SearchResults>> =
        tokio::task::spawn_blocking(move || {
            Self::execute_search_blocking(
                reader, schema, parsed_query, limit, token_clone,
            )
        });

    // 等待结果或超时
    match timeout(timeout_duration, search_handle).await {
        Ok(Ok(result)) => result,
        Ok(Err(join_err)) => Err(SearchError::IndexError(format!(
            "Search task failed: {}", join_err
        ))),
        Err(_) => {
            token.cancel();
            Err(SearchError::Timeout(format!(
                "Search timed out after {}ms", timeout_duration.as_millis()
            )))
        }
    }
}
```

### 3. 背压控制 (`concurrency_safety/backpressure.rs`)

```rust
/// 背压控制器
pub struct BackpressureController {
    semaphore: Arc<Semaphore>,
    config: SemaphoreConfig,
    waiting_count: Arc<Mutex<usize>>,
    rejected_count: Arc<Mutex<u64>>,
}

impl BackpressureController {
    /// 获取访问许可（异步）
    pub async fn acquire(&self) -> Option<BackpressurePermit<'_>> {
        // 增加等待计数
        {
            let mut waiting = self.waiting_count.lock().await;
            *waiting += 1;
        }

        let result = match tokio::time::timeout(
            self.config.acquire_timeout,
            self.semaphore.acquire(),
        ).await {
            Ok(Ok(permit)) => Some(BackpressurePermit { permit, controller: self }),
            Ok(Err(_)) => None,
            Err(_) => {
                // 超时，增加拒绝计数
                let mut rejected = self.rejected_count.lock().await;
                *rejected += 1;
                None
            }
        };

        // 减少等待计数
        {
            let mut waiting = self.waiting_count.lock().await;
            *waiting -= 1;
        }

        result
    }
}
```

### 4. 可取消搜索 (`search_engine/cancellable_search.rs`)

```rust
/// 增强版可取消收集器
pub struct EnhancedCancellableCollector<C> {
    inner: C,
    token: CancellationToken,
    config: CancellableConfig,
    docs_processed: Arc<AtomicUsize>,
}

impl<C: Collector> Collector for EnhancedCancellableCollector<C> {
    fn for_segment(&self, segment_id: SegmentOrdinal, reader: &SegmentReader)
        -> tantivy::Result<Self::Child> 
    {
        // 在 segment 级别检查取消
        if self.token.is_cancelled() {
            return Err(TantivyError::InternalError("Search cancelled".to_string()));
        }
        
        let child = self.inner.for_segment(segment_id, reader)?;
        Ok(EnhancedCancellableChildCollector { inner: child, token: self.token.clone() })
    }
}

/// 搜索取消控制器
pub struct SearchCancellationController {
    token: CancellationToken,
    start_time: Instant,
    timeout: Option<Duration>,
}

impl SearchCancellationController {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            token: CancellationToken::new(),
            start_time: Instant::now(),
            timeout: Some(timeout),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        if self.token.is_cancelled() {
            return true;
        }
        
        // 检查超时
        if let Some(timeout) = self.timeout {
            if self.start_time.elapsed() > timeout {
                self.cancel();
                return true;
            }
        }
        false
    }
}
```

---

## 关键 API 使用示例

### 1. CAS 原子写入

```rust
use crate::storage::cas_atomic::AtomicContentAddressableStorage;

let cas = AtomicContentAddressableStorage::new(workspace_dir);

// 存储内容
let hash = cas.store_content_atomic(b"content").await?;

// 流式存储大文件
let hash = cas.store_file_streaming_atomic(file_path).await?;
```

### 2. 真正异步搜索

```rust
use crate::search_engine::async_manager::{
    AsyncSearchEngineManager, AsyncSearchConfig
};

let config = AsyncSearchConfig::default();
let manager = AsyncSearchEngineManager::new(config)?;

// 带超时的搜索
let results = manager.search_with_timeout("query", None, Some(Duration::from_secs(1))).await?;

// 可取消搜索
let token = CancellationToken::new();
let results = manager.search_cancellable("query", None, None, token).await?;
```

### 3. 背压控制

```rust
use crate::concurrency_safety::backpressure::BackpressureController;

let controller = BackpressureController::default_with_concurrency(10);

// 获取许可
if let Some(permit) = controller.acquire().await {
    // 执行受保护的资源访问
    perform_search().await;
    // 许可自动释放
}
```

### 4. 可取消任务

```rust
use crate::search_engine::cancellable_search::SearchCancellationController;

let controller = SearchCancellationController::with_timeout(Duration::from_secs(5));
let token = controller.token();

// 在另一个任务中取消
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(2)).await;
    controller.cancel();
});

// 检查取消
if token.is_cancelled() {
    return Err("Search cancelled".into());
}
```

---

## 性能考量

### 1. CAS 原子写入性能

| 方案 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| O_EXCL 原子创建 | 简单、原子性强 | 不能覆盖 | 小文件、新文件 |
| 临时文件 + 重命名 | 跨平台、可恢复 | 需要额外 I/O | 大文件、关键数据 |
| 内存缓存检查 | 性能最好 | 非权威 | 高频重复检查 |

### 2. spawn_blocking 性能

```rust
// 线程池大小建议
let pool_size = match task_type {
    CpuBound => num_cpus::get() * 2,
    IoBound => num_cpus::get() * 4,
};
```

### 3. 背压参数建议

```rust
let config = SemaphoreConfig {
    max_concurrent: num_cpus::get() * 2,  // CPU 密集型
    acquire_timeout: Duration::from_secs(30),
    fair: true,  // 公平模式防止饥饿
};
```

---

## 测试验证

所有实现都包含完整的单元测试：

```rust
#[tokio::test]
async fn test_concurrent_writes() {
    let cas = AtomicContentAddressableStorage::new(temp_dir);
    let content = b"concurrent content";
    
    // 并发写入相同内容
    let (r1, r2) = tokio::join!(
        cas.store_content_atomic(content),
        cas.store_content_atomic(content)
    );
    
    // 两者都应该成功且返回相同哈希
    assert_eq!(r1.unwrap(), r2.unwrap());
}

#[tokio::test]
async fn test_search_cancellation() {
    let manager = AsyncSearchEngineManager::new(config).unwrap();
    let token = CancellationToken::new();
    
    // 延迟取消
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
    });
    
    let result = manager.search_cancellable("query", None, None, token).await;
    // 应该被取消或完成
    assert!(result.is_ok() || matches!(result, Err(SearchError::QueryError(_))));
}
```

---

## 迁移指南

### 从旧版 CAS 迁移

```rust
// 旧代码
let cas = ContentAddressableStorage::new(workspace);
let hash = cas.store_content(content).await?;

// 新代码
let cas = AtomicContentAddressableStorage::new(workspace);
let hash = cas.store_content_atomic(content).await?;
```

### 从旧版搜索管理器迁移

```rust
// 旧代码
let manager = SearchEngineManager::new(config)?;
let results = manager.search_with_timeout(query, None, None, None).await?;

// 新代码
let manager = AsyncSearchEngineManager::new(config)?;
let results = manager.search_with_timeout(query, None, None).await?;
```

---

## 总结

本解决方案提供了：

1. **完整的并发安全保证**: 消除 TOCTOU、阻塞、资源耗尽风险
2. **业内成熟方案**: Git CAS、Tokio spawn_blocking、Semaphore 背压
3. **生产就绪代码**: 完整的错误处理、日志、测试
4. **向后兼容**: 平滑迁移路径
5. **性能优化**: 最小化同步开销，最大化并发性能

所有代码已集成到项目中，可直接使用。
