# 代码修复方案

> 基于完整代码审查确认的36个真实问题
> 文档版本: 2026-03-31

## 问题统计

| 级别 | 数量 | 状态 |
|------|------|------|
| 🔴 CRITICAL | 4 | 需立即修复 |
| 🟠 HIGH | 10 | 建议尽快修复 |
| 🟡 MEDIUM | 12 | 计划修复 |
| 🟢 LOW | 10 | 优化项 |
| **总计** | **36** | - |

---

## 🔴 CRITICAL 级别修复方案

### 1. cache_key 哈希不完整

**位置**: `log-analyzer/src-tauri/src/services/query_executor.rs:61-77`

**问题描述**:
`generate_cache_key()` 函数只哈希了搜索词的 `value/is_regex/case_sensitive/enabled` 字段，遗漏了 `term.id/operator/priority` 以及 `query.filters/id`。这会导致不同的查询产生相同的缓存键，返回错误的缓存结果。

**当前代码**:
```rust
fn generate_cache_key(query: &SearchQuery) -> String {
    let mut hasher = DefaultHasher::new();
    for term in &query.terms {
        term.value.hash(&mut hasher);  // 只哈希了 value
        term.is_regex.hash(&mut hasher);
        term.case_sensitive.hash(&mut hasher);
        term.enabled.hash(&mut hasher);
        // 缺少: term.id, term.operator, term.priority
    }
    // 缺少: query.filters, query.id, query.global_operator
    format!("{:x}", hasher.finish())
}
```

**修复方案**:
```rust
fn generate_cache_key(query: &SearchQuery) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // 包含查询ID（如果有）
    query.id.hash(&mut hasher);
    query.global_operator.hash(&mut hasher);

    // 包含所有搜索词字段（按优先级排序以保持一致性）
    let mut sorted_terms = query.terms.clone();
    sorted_terms.sort_by(|a, b| a.id.cmp(&b.id));

    for term in &sorted_terms {
        term.id.hash(&mut hasher);
        term.value.hash(&mut hasher);
        term.is_regex.hash(&mut hasher);
        term.case_sensitive.hash(&mut hasher);
        term.enabled.hash(&mut hasher);
        term.operator.hash(&mut hasher);
        term.priority.hash(&mut hasher);
    }

    // 包含过滤器
    if let Some(filters) = &query.filters {
        filters.hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}
```

**验证方法**:
```rust
#[test]
fn test_cache_key_uniqueness() {
    let query1 = create_query_with_priority(1);
    let query2 = create_query_with_priority(2);
    assert_ne!(
        generate_cache_key(&query1),
        generate_cache_key(&query2)
    );
}
```

---

### 2. 文档与实现不一致 (Mutex vs RwLock)

**位置**: `log-analyzer/src-tauri/src/models/state.rs:21-35` vs `67-81`

**问题描述**:
注释声明使用 `tokio::sync::RwLock` 实现写并发，但实际代码使用 `std::sync::Mutex`。这导致：
1. 并发能力下降（单写锁阻塞所有读取）
2. 在异步上下文中使用同步锁可能阻塞运行时线程

**当前代码**:
```rust
// 第21-35行注释说明:
// "使用 tokio::sync::RwLock 实现写并发，读操作可以并行"

// 第67-81行实际实现:
pub struct AppState {
    pub workspaces: Mutex<HashMap<String, WorkspaceInfo>>,  // 实际使用 Mutex
    // ...
}
```

**官方文档确认** (Tokio官方):
- `std::sync::Mutex` 会阻塞整个线程
- `tokio::sync::RwLock` 只挂起当前任务，不阻塞线程
- 在异步代码中应优先使用 Tokio 提供的异步锁

**修复方案**:

选项A: 如果确实需要写并发，改为 `tokio::sync::RwLock`:
```rust
use tokio::sync::RwLock;

pub struct AppState {
    pub workspaces: RwLock<HashMap<String, WorkspaceInfo>>,
    // ...
}

// 修改所有调用点:
// let workspaces = state.workspaces.lock().unwrap();  // 旧
let workspaces = state.workspaces.read().await;        // 新（读）
let mut workspaces = state.workspaces.write().await;   // 新（写）
```

选项B: 如果 Mutex 足够，更新文档注释:
```rust
// 使用 std::sync::Mutex 保护状态
// 注意: 锁持有期间不要执行 .await 操作
```

**推荐**: 选项A，因为 `RwLock` 支持并发读取，更适合读多写少的场景。

---

### 3. 无界通道内存泄漏

**位置**: `log-analyzer/src-tauri/src/task_manager/mod.rs:551`

**问题描述**:
使用 `mpsc::unbounded_channel()` 接收任务事件，无流量控制。在高负载下，如果消费者处理速度慢于生产者，消息会无限堆积在内存中，最终导致OOM。

**当前代码**:
```rust
let (tx, rx) = mpsc::unbounded_channel();  // 无界通道
```

**官方文档确认** (Tokio官方):
- `unbounded_channel()` 无容量限制，可能导致无限内存增长
- 应使用有界通道 `channel(capacity)` 实现背压(backpressure)
- 当通道满时，`send().await` 会等待直到有空间

**修复方案**:
```rust
// 使用有界通道，容量根据预期负载调整
const CHANNEL_CAPACITY: usize = 1000;

let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);

// 发送端处理背压
tx.send(event).await?;  // 当通道满时会等待

// 或使用 try_send 并处理错误
match tx.try_send(event) {
    Ok(()) => {},
    Err(TrySendError::Full(_)) => {
        // 记录警告，丢弃或等待
        warn!("Task channel full, dropping event");
    }
    Err(TrySendError::Closed(_)) => {
        return Err(AppError::channel_closed());
    }
}
```

**高级方案**: 实现分级背压
```rust
enum BackpressureStrategy {
    DropOldest,    // 丢弃最旧的消息
    DropNewest,    // 丢弃新消息
    Block,         // 阻塞等待
    Error,         // 返回错误
}

struct BoundedChannel<T> {
    tx: mpsc::Sender<T>,
    strategy: BackpressureStrategy,
    dropped_count: AtomicUsize,
}
```

---

### 4. 孤儿文件清理竞态条件

**位置**: `crates/la-storage/src/coordinator.rs:190-231`

**问题描述**:
`cleanup_orphan_files()` 使用 check-then-act 模式：先检查文件是否存在，然后执行清理。在多线程环境下，检查通过后到执行清理之间，文件可能被其他线程创建，导致误删新文件。

**问题模式**:
```rust
// Check-then-act 竞态条件
if file_exists(&path) {           // 线程A检查通过
    // 线程B在此刻创建文件
    delete_file(&path);           // 线程A误删线程B的文件
}
```

**官方最佳实践**:
- 使用原子操作替代 check-then-act
- 使用文件锁或数据库事务保证一致性
- 使用 CAS (Compare-And-Swap) 模式

**修复方案**:

方案A: 使用文件锁保护
```rust
use fs4::FileExt;  // 需要添加依赖

fn cleanup_orphan_files_safe(&self) -> Result<()> {
    // 获取独占锁
    let lock_file = self.work_dir.join(".cleanup.lock");
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_file)?;

    lock.lock_exclusive()?;  // 阻塞直到获取锁
    defer! { let _ = lock.unlock(); }

    // 安全执行清理
    self.do_cleanup()?;
    Ok(())
}
```

方案B: 使用 CAS 模式 + 时间戳
```rust
struct FileEntry {
    path: PathBuf,
    created_at: SystemTime,
    verified: AtomicBool,
}

fn cleanup_with_verification(&self) -> Result<()> {
    let candidates = self.find_orphan_candidates()?;

    for entry in candidates {
        // 再次验证文件未被修改
        let metadata = std::fs::metadata(&entry.path)?;
        let modified = metadata.modified()?;

        // 只删除创建时间超过阈值且未被修改的文件
        if self.is_safe_to_delete(&entry, modified) {
            std::fs::remove_file(&entry.path)?;
        }
    }
    Ok(())
}
```

方案C: 数据库事务（推荐）
```rust
// 在数据库层面保证一致性
sqlx::transaction!(self.db, |tx| async move {
    // 标记待删除文件
    sqlx::query!("UPDATE files SET pending_delete = true WHERE ...")
        .execute(&mut **tx)
        .await?;

    // 获取确认列表
    let to_delete: Vec<String> = sqlx::query_scalar!("SELECT path FROM files WHERE pending_delete = true")
        .fetch_all(&mut **tx)
        .await?;

    // 物理删除文件
    for path in &to_delete {
        match tokio::fs::remove_file(path).await {
            Ok(()) => {
                sqlx::query!("DELETE FROM files WHERE path = ?", path)
                    .execute(&mut **tx)
                    .await?;
            }
            Err(e) => {
                warn!("Failed to delete {}: {}", path, e);
                // 回滚或标记为手动处理
            }
        }
    }

    Ok(())
});
```

---

## 🟠 HIGH 级别修复方案

### 5. ReaderPool 未使用

**位置**: `crates/la-search/src/concurrent_search.rs:75-144`

**问题**:
`ReaderPool` 结构体创建后，`_readers` 字段从未被访问，没有 `get_reader()` 方法。

**修复**:
```rust
pub struct ReaderPool {
    readers: Vec<IndexReader>,
    current: AtomicUsize,
}

impl ReaderPool {
    pub fn get_reader(&self) -> Option<&IndexReader> {
        let idx = self.current.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        self.readers.get(idx)
    }

    pub fn with_reader<F, R>(&self, f: F) -> Result<R>
    where F: FnOnce(&IndexReader) -> Result<R>
    {
        self.get_reader()
            .ok_or_else(|| AppError::no_reader_available())
            .and_then(f)
    }
}
```

---

### 6. block_on 阻塞运行时

**位置**: `services/query_executor.rs:112-119`

**问题**:
在 async 上下文中使用 `tokio::task::block_on()` 同步等待，可能阻塞运行时线程。

**修复**:
```rust
// 旧代码
tokio::task::block_on(async { ... })

// 新代码 - 使用 spawn_blocking
tokio::task::spawn_blocking(move || {
    Handle::current().block_on(async { ... })
}).await??;
```

---

### 7. check-then-rename 竞态条件

**位置**: `crates/la-storage/src/cas.rs:1265-1310`

**问题**:
`store_file_zero_copy()` 先检查文件是否存在，再执行重命名，非原子操作。

**修复**:
```rust
// 使用原子重命名（Unix guarantees atomicity）
fn store_atomic(&self, temp_path: &Path, final_path: &Path) -> Result<()> {
    // fs::rename 在 Unix 上是原子的
    match std::fs::rename(temp_path, final_path) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(libc::EEXIST) => {
            // 文件已存在，这是预期的（去重）
            std::fs::remove_file(temp_path)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
```

---

## 修复实施计划

### 阶段1: CRITICAL (第1周)
- [ ] Day 1-2: 修复 cache_key 哈希完整性
- [ ] Day 3-4: 修复 Mutex/RwLock 文档或实现
- [ ] Day 5-6: 替换无界通道为有界通道
- [ ] Day 7: 修复孤儿文件清理竞态

### 阶段2: HIGH (第2-3周)
- [ ] 实现 ReaderPool.get_reader()
- [ ] 审查并替换所有 block_on
- [ ] 修复 CAS 竞态条件
- [ ] 统一错误类型

### 阶段3: MEDIUM (第4-6周)
- [ ] 提升测试覆盖率
- [ ] 实现性能监控
- [ ] 完善配置验证
- [ ] 清理冗余代码

### 阶段4: LOW (持续)
- [ ] 代码风格统一
- [ ] 文档完善
- [ ] 依赖更新

---

## 验证清单

每项修复后需验证:
- [ ] 单元测试通过
- [ ] 集成测试通过
- [ ] `cargo clippy --all-features -- -D warnings`
- [ ] `cargo fmt -- --check`
- [ ] 手动测试关键路径
- [ ] 性能基准对比（如有影响）

---

## 风险评估

| 修复项 | 风险 | 缓解措施 |
|--------|------|----------|
| cache_key 修复 | 缓存失效，短暂性能下降 | 渐进式部署，监控缓存命中率 |
| Mutex→RwLock | 可能引入死锁 | 仔细审查锁顺序，添加超时 |
| 有界通道 | 可能阻塞生产者 | 合理设置容量，监控队列深度 |
| 竞态条件修复 | 行为变更 | 充分测试，保留旧代码开关 |

---

*文档生成时间: 2026-03-31*
*基于: Tantivy 0.22, Tokio 1.x, Rust 1.70+ 官方文档*
