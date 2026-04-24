# 纯 Rust 存储迁移方案

> 状态：设计阶段  
> 日期：2026-04-23  
> 关联文档：`ARCHIVE_SEARCH_PIPELINE_ANALYSIS.md`、`CAS_ARCHITECTURE.md`、`MODULE_ARCHITECTURE.md`

---

## 1. 调研结论：纯 Rust 官方解决方案

### 1.1 为什么没有"官方"嵌入式数据库

Rust 官方（Rust Foundation / Core Team）**没有维护**嵌入式数据库项目。与 Go 的 `boltdb` 或 Zig 的 `TigerBeetle` 不同，Rust 生态的嵌入式存储由社区驱动。

### 1.2 社区最认可的纯 Rust 方案

| 项目 | Stars | 维护状态 | 类型 | C 依赖 | 评估 |
|------|:-----:|:--------:|:-----|:------:|:-----|
| **redb** | ~3k | 活跃（2024-2026 持续更新） | B-tree KV | ❌ 无 | ✅ 推荐 |
| native_db | ~1k | 活跃 | redb 上的 ORM | ❌ 无 | ⚠️ API 不稳定 |
| sled | ~7k | **已归档**（2023 停止维护） | LSM-tree | ❌ 无 | ❌ 不推荐 |
| persy | ~300 | 不活跃 | 事务存储 | ❌ 无 | ❌ 不推荐 |
| heed | ~1k | 活跃 | LMDB 的 Rust 封装 | ✅ 有 | ❌ 不符合纯 Rust 要求 |
| rustqlite | ~3k | 活跃 | SQLite 的 Rust 绑定 | ✅ 有 | ❌ 底层仍是 C |

**结论：`redb` 是当前 Rust 生态中最成熟、维护最活跃、社区认可度最高的纯 Rust 嵌入式数据库。**

### 1.3 redb 核心特性

```rust
// redb 2.x API 示例
use redb::{Database, TableDefinition};

const FILES: TableDefinition<&str, &[u8]> = TableDefinition::new("files");

let db = Database::create("metadata.redb")?;
let write_txn = db.begin_write()?;
{
    let mut table = write_txn.open_table(FILES)?;
    table.insert("hash_abc", serialized_metadata)?;
}
write_txn.commit()?;
```

- **纯 Rust**：零 C/C++ 依赖，编译产物完全可控
- **ACID 事务**：支持读写事务、savepoint、rollback
- **MVCC**：并发读不阻塞写
- **B-tree 存储**：天然支持范围查询和有序遍历
- **零拷贝读取**：`AccessGuard` 允许直接引用内存映射数据
- **单文件**：数据库即一个 `.redb` 文件，便于备份和迁移

---

## 2. 当前架构审计

### 2.1 CAS 已是纯 Rust

`ContentAddressableStorage` (`crates/la-storage/src/cas.rs`) **无需替换**：
- 100% Rust 实现
- 纯磁盘存储（`objects/{shard}/{hash}`）
- 无外部依赖（仅标准库 + tokio + sha2）
- 已实现 `ContentStorage` trait

### 2.2 SQLite 不是纯 Rust

`MetadataStore` (`crates/la-storage/src/metadata_store.rs`) **需要替换**：
- 依赖 `sqlx` + `libsqlite3-sys`，底层是 C 的 SQLite
- 引入 C 编译链、跨平台构建复杂度、安全审计面

### 2.3 Trait 抽象现状

`la-core/src/traits.rs` 已定义抽象：

```rust
#[async_trait]
pub trait MetadataStorage: Send + Sync {
    async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64>;
    async fn get_all_files(&self) -> Result<Vec<FileMetadata>>;
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>>;
}
```

**问题**：trait 只定义了 3 个方法，但 `MetadataStore` 实际提供 30+ 个方法。各处代码直接依赖 `MetadataStore` struct 而非 `MetadataStorage` trait，导致后端不可替换。

### 2.4 实际使用的方法集（生产代码）

通过对 `src/commands/` 和 `src/services/` 的静态分析，实际调用的方法如下：

**MetadataStore（按调用频率排序）：**

| 方法 | 调用次数 | 调用方 | 重要性 |
|------|:--------:|--------|:------:|
| `get_all_files` | 8 | search, import, async_search, virtual_tree | 🔴 核心 |
| `get_all_archives` | 3 | virtual_tree | 🔴 核心 |
| `count_files` | 2 | workspace, import | 🟡 次要 |
| `insert_file` | — | processor.rs | 🔴 核心 |
| `insert_archive` | — | processor.rs | 🔴 核心 |
| `update_archive_status` | — | processor.rs | 🟡 次要 |
| `get_file_by_hash` | — | gc.rs | 🟡 次要 |
| `search_files` (FTS5) | — | 可能某处 | 🟢 低频 |
| `get_file_by_virtual_path` | — | 可能某处 | 🟢 低频 |
| `clear_all` | — | import.rs (rebuild) | 🟡 次要 |
| `begin_transaction` | — | batch insert | 🟡 次要 |

**CAS（按调用频率排序）：**

| 方法 | 调用次数 | 调用方 | 重要性 |
|------|:--------:|--------|:------:|
| `store_file_streaming` | — | processor.rs, coordinator.rs | 🔴 核心 |
| `exists` | 6 | search, virtual_tree, index_validator | 🔴 核心 |
| `read_content_sync` | 2 | search, import | 🔴 核心 |
| `read_content` | 1 | async_search | 🔴 核心 |
| `get_storage_size` | 3 | workspace_metrics | 🟡 次要 |

---

## 3. 迁移方案设计

### 3.1 目标

1. 用 **redb** 替换 SQLite，消除 C 依赖
2. 保留 CAS（已是纯 Rust）
3. 保持向后兼容：SQLite 后端可作为 feature flag 保留
4. 最小化对业务代码的侵入

### 3.2 架构变更概览

```
变更前：
┌─────────────────┐     ┌──────────────────┐     ┌─────────────┐
│   业务代码       │────→│  MetadataStore   │────→│  SQLite (C) │
│  (硬编码依赖)     │     │  (sqlx + sqlite) │     │  metadata.db│
└─────────────────┘     └──────────────────┘     └─────────────┘
                                │
                                ↓
                        ┌──────────────────┐
                        │ ContentAddressable│
                        │    Storage       │──→ objects/ (纯 Rust)
                        │   (保持现状)      │
                        └──────────────────┘

变更后：
┌─────────────────┐     ┌──────────────────────┐
│   业务代码       │────→│   MetadataStorage    │◄── trait 对象
│  (trait 抽象)    │     │      (trait)         │
└─────────────────┘     └──────────┬───────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    ↓              ↓              ↓
            ┌─────────────┐ ┌─────────────┐ ┌────────────┐
            │RedbMetadata │ │SqliteMetadata│ │ MockStore  │
            │   Store     │ │   Store      │ │  (tests)   │
            │  (redb)     │ │ (sqlx,保留)  │ │            │
            └─────────────┘ └─────────────┘ └────────────┘
                                   │
                        ┌──────────┴──────────┐
                        ↓                     ↓
                 metadata.redb          metadata.db
                 (纯 Rust)              (C, 兼容)
```

### 3.3 Phase 1：扩展 MetadataStorage trait

当前 trait 过于简单，需扩展以覆盖生产代码实际使用的方法。

```rust
// la-core/src/traits.rs

#[async_trait]
pub trait MetadataStorage: Send + Sync {
    // === 文件元数据 ===
    async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64>;
    async fn get_all_files(&self) -> Result<Vec<FileMetadata>>;
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>>;
    async fn get_file_by_virtual_path(&self, vpath: &str) -> Result<Option<FileMetadata>>;
    async fn count_files(&self) -> Result<i64>;
    async fn clear_files(&self) -> Result<()>;

    // === 压缩包元数据 ===
    async fn insert_archive(&self, metadata: &ArchiveMetadata) -> Result<i64>;
    async fn get_all_archives(&self) -> Result<Vec<ArchiveMetadata>>;
    async fn get_archive_by_id(&self, id: i64) -> Result<Option<ArchiveMetadata>>;
    async fn update_archive_status(&self, archive_id: i64, status: &str) -> Result<()>;
    async fn count_archives(&self) -> Result<i64>;
    async fn clear_archives(&self) -> Result<()>;

    // === 批量操作 ===
    async fn insert_files_batch(&self, files: Vec<FileMetadata>) -> Result<Vec<i64>>;

    // === 全文搜索（可选，可降级实现）===
    async fn search_files(&self, query: &str) -> Result<Vec<FileMetadata>>;
}
```

**影响范围**：
- `la-core`：扩展 trait 定义
- `la-storage`：`MetadataStore` 改名为 `SqliteMetadataStore` 并实现扩展后的 trait
- 各命令模块：将 `MetadataStore` 类型参数改为 `Arc<dyn MetadataStorage>` 或泛型参数

### 3.4 Phase 2：实现 RedbMetadataStore

#### 数据模型映射

```rust
// redb 表定义
const FILES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("files");
const ARCHIVES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("archives");
const VPATH_INDEX: TableDefinition<&str, &str> = TableDefinition::new("vpath_index");
```

| SQLite 表/索引 | redb 映射 | 说明 |
|----------------|----------|------|
| `files` | `FILES_TABLE` | key = `sha256_hash`, value = ` postcard::serialize(FileMetadata)` |
| `archives` | `ARCHIVES_TABLE` | key = `id.to_string()`, value = `postcard::serialize(ArchiveMetadata)` |
| `idx_files_virtual_path` | `VPATH_INDEX` | key = `virtual_path`, value = `sha256_hash`（二级索引） |
| `idx_files_hash` | 内嵌 | `FILES_TABLE` 的 key 本身就是 hash |
| `idx_files_parent_archive` | 查询时过滤 | `get_all_files()` 后内存过滤，或增加 `PARENT_INDEX` |
| `files_fts` (FTS5) | 降级为前缀扫描 | redb 不支持全文索引，可用 `VPATH_INDEX` 的范围扫描替代 |

#### 序列化选择

| 方案 | 优点 | 缺点 |
|------|------|------|
| `postcard` | 零拷贝友好、体积小、纯 Rust | 无 schema 演进支持 |
| `bincode` | 成熟、速度快 | 体积稍大 |
| `serde_json` | 人类可读、调试方便 | 体积大、速度慢 |

**推荐：`postcard`**（已被 redb 社区广泛采用，与 `native_db` 默认序列化一致）

#### 事务映射

```rust
// SQLite: BEGIN → INSERT → COMMIT
// redb:   begin_write() → open_table() → insert() → commit()

impl RedbMetadataStore {
    pub async fn insert_files_batch(&self, files: Vec<FileMetadata>) -> Result<Vec<i64>> {
        // redb 写事务是同步的，需要在 spawn_blocking 中执行
        tokio::task::spawn_blocking(move || {
            let write_txn = self.db.begin_write()?;
            {
                let mut table = write_txn.open_table(FILES_TABLE)?;
                let mut vpath_idx = write_txn.open_table(VPATH_INDEX)?;
                let mut ids = Vec::new();
                let mut next_id = self.next_id.fetch_add(files.len(), Ordering::SeqCst);

                for file in files {
                    let id = next_id as i64;
                    next_id += 1;
                    let mut file_with_id = file;
                    file_with_id.id = id;

                    let bytes = postcard::to_allocvec(&file_with_id)?;
                    table.insert(&*file_with_id.sha256_hash, &*bytes)?;
                    vpath_idx.insert(&*file_with_id.virtual_path, &*file_with_id.sha256_hash)?;
                    ids.push(id);
                }
            }
            write_txn.commit()?;
            Ok(ids)
        }).await?
    }
}
```

**关键差异**：
- redb 的写事务是**同步**的（基于 `parking_lot::RwLock`），大量写入需要在 `spawn_blocking` 中执行
- 但读事务是**非阻塞**的（MVCC），多个读者可并发

### 3.5 Phase 3：业务代码迁移

#### 当前硬编码依赖

```rust
// search.rs (当前)
let files = metadata_store.get_all_files().await?;
```

#### 目标形态

```rust
// 方式 A：泛型参数（零运行时开销）
pub async fn search_logs<S: MetadataStorage>(
    metadata_store: Arc<S>,
    ...
) -> Result<String, CommandError> {
    let files = metadata_store.get_all_files().await?;
}

// 方式 B：trait 对象（动态分发，更灵活）
pub async fn search_logs(
    metadata_store: Arc<dyn MetadataStorage>,
    ...
) -> Result<String, CommandError> {
    let files = metadata_store.get_all_files().await?;
}
```

**推荐方式 B**（`Arc<dyn MetadataStorage>`）：
- 与当前 `AppState` 中 `Arc<MetadataStore>` 的用法一致
- 允许运行时切换后端（通过配置或 feature flag）
- `async_trait` 已处理动态分发的复杂性

#### AppState 变更

```rust
// models/state.rs (当前)
pub struct AppState {
    pub metadata_stores: Mutex<HashMap<String, Arc<MetadataStore>>>,
    ...
}

// 目标
pub struct AppState {
    pub metadata_stores: Mutex<HashMap<String, Arc<dyn MetadataStorage>>>,
    ...
}
```

#### ensure_workspace_runtime_state 变更

```rust
// import.rs
pub async fn ensure_workspace_runtime_state(...) -> Result<(
    Arc<ContentAddressableStorage>,
    Arc<dyn MetadataStorage>,  // ← 改为 trait 对象
    Arc<SearchEngineManager>,
), String> {
    // 根据配置选择后端
    let metadata_store: Arc<dyn MetadataStorage> = if use_redb {
        Arc::new(RedbMetadataStore::new(workspace_dir).await?)
    } else {
        Arc::new(SqliteMetadataStore::new(workspace_dir).await?)
    };
    ...
}
```

### 3.6 Phase 4：FTS5 替代方案

当前 SQLite 使用 FTS5 虚拟表进行文件名全文搜索。redb 不支持全文索引，需要替代方案：

| 方案 | 实现 | 复杂度 |
|------|------|:------:|
| **A. 前缀范围扫描** | `VPATH_INDEX.start_with("query")` | 低 |
| **B. 内存倒排索引** | 启动时加载所有 vpath 到 `HashMap<String, Vec<String>>` | 中 |
| **C. Tantivy 统一索引** | 将文件名也加入 Tantivy 索引，搜索时先查 Tantivy 再过滤 | 中 |

**推荐方案 A + C 组合**：
- 简单前缀匹配走 redb 范围扫描
- 复杂搜索（如时间范围 + 文件名）利用已有的 Tantivy 基础设施

### 3.7 Feature Flag 设计

```toml
# la-storage/Cargo.toml
[features]
default = ["sqlite-backend"]
sqlite-backend = ["sqlx", "libsqlite3-sys"]
redb-backend = ["redb", "postcard"]
```

```rust
// la-storage/src/lib.rs
#[cfg(feature = "sqlite-backend")]
pub use metadata_store::SqliteMetadataStore;

#[cfg(feature = "redb-backend")]
pub use redb_metadata_store::RedbMetadataStore;
```

---

## 4. 风险评估

### 4.1 redb 的已知限制

| 限制 | 影响 | 缓解措施 |
|------|------|----------|
| 写事务是同步的 | 批量导入可能阻塞 async runtime | 在 `spawn_blocking` 中执行写事务 |
| 不支持全文索引 | 失去 FTS5 能力 | 前缀扫描 + Tantivy 替代 |
| 不支持外键约束 | 失去 `ON DELETE CASCADE` | 应用层实现级联删除 |
| 单文件上限 ~4EB | 无实际影响 | 远超桌面应用需求 |
| 社区规模 < SQLite | 长期维护风险 | redb 被多个知名项目采用（如 `native_db`） |

### 4.2 迁移风险

| 风险 | 概率 | 影响 | 缓解 |
|------|:----:|:----:|------|
| 数据格式不兼容 | 低 | 高 | 提供迁移工具：SQLite → redb 导出导入 |
| 性能退化（小查询） | 低 | 中 | benchmark 对比，保留 SQLite fallback |
| 并发写入冲突 | 中 | 中 | redb 的 MVCC 比 SQLite WAL 更优，但需测试验证 |
| 编译问题（旧平台） | 低 | 低 | redb 纯 Rust，编译兼容性优于 SQLite C 库 |

### 4.3 回滚策略

1. **Feature flag 级别**：编译时选择 `sqlite-backend` 或 `redb-backend`
2. **数据级别**：迁移工具支持 redb → SQLite 反向导出
3. **运行时级别**：通过配置文件 `backend = "sqlite" | "redb"` 切换

---

## 5. 实施路线图

```
Week 1-2: Phase 1
  ├── 扩展 MetadataStorage trait（la-core）
  ├── 将 MetadataStore 改名为 SqliteMetadataStore
  ├── 实现扩展 trait for SqliteMetadataStore
  └── 验证所有测试通过

Week 3-4: Phase 2
  ├── 引入 redb + postcard 依赖
  ├── 实现 RedbMetadataStore
  ├── 实现 SQLite → redb 迁移工具
  └── 单元测试覆盖所有 trait 方法

Week 5-6: Phase 3
  ├── 修改 AppState 使用 Arc<dyn MetadataStorage>
  ├── 修改 commands/ 中硬编码的 MetadataStore 引用
  ├── 添加 backend 配置项
  └── 集成测试（redb 后端）

Week 7-8: Phase 4
  ├── benchmark：SQLite vs redb（导入速度、搜索速度、内存占用）
  ├── 修复性能退化点
  ├── 文档更新
  └── 灰度发布（默认 SQLite，可选 redb）

Week 9+: Phase 5
  ├── 收集用户反馈
  ├── 修复 edge case
  └── 考虑将 redb 设为默认后端
```

---

## 6. 替代方案（如果 redb 不适用）

### 6.1 方案 B：native_db（redb 上的 ORM）

```rust
#[derive(Serialize, Deserialize)]
#[native_model(id = 1, version = 1)]
#[native_db]
struct FileMetadata {
    #[primary_key]
    sha256_hash: String,
    #[secondary_key]
    virtual_path: String,
    ...
}
```

**优点**：自动索引管理、类型安全、有 Tauri 示例  
**缺点**：API 尚不稳定、macro 魔法增加编译时间、对复杂查询支持有限  
**适用场景**：如果团队偏好 ORM 风格且能接受 API 变动风险

### 6.2 方案 C：完全自定义纯磁盘格式

如 `ARCHIVE_SEARCH_PIPELINE_ANALYSIS.md` 中方案 B 所述：
- `files.ndjson` + `files-by-hash.idx` + `files-by-vpath.idx`

**优点**：零外部依赖、完全可控  
**缺点**：需自行实现事务、并发控制、崩溃恢复  
**适用场景**：如果项目对依赖数量极度敏感

### 6.3 方案 D：保留 SQLite，仅去掉 Tantivy 重建

如 `ARCHIVE_SEARCH_PIPELINE_ANALYSIS.md` 中方案 F 所述：
- 不替换 SQLite，仅停止无效的 Tantivy 索引重建

**优点**：最小改动、最低风险  
**缺点**：没有解决"纯 Rust"诉求  
**适用场景**：如果"去掉 C 依赖"的优先级低于"稳定性"

---

## 7. 决策建议

| 优先级 | 行动 | 预期收益 |
|--------|------|----------|
| P0 | **采用 redb 替换 SQLite** | 消除 C 依赖，实现全栈纯 Rust |
| P1 | **扩展 MetadataStorage trait** | 解耦存储后端，支持 A/B 测试 |
| P2 | **保留 SQLite 作为 feature flag** | 风险可控，支持回滚 |
| P3 | **CAS 保持现状** | 已是纯 Rust，无需改动 |
| P4 | **benchmark 验证** | 确保 redb 性能不低于 SQLite |

**最终建议**：

> 采用 **redb** 作为 SQLite 的纯 Rust 替代方案，分 5 个阶段实施。CAS 已是纯 Rust 实现，无需替换。通过扩展 `MetadataStorage` trait 实现后端解耦，保留 SQLite 作为编译时 feature flag 确保回滚能力。

---

## 8. 关联文件索引

| 文件 | 当前职责 | 变更范围 |
|------|---------|----------|
| `la-core/src/traits.rs` | `MetadataStorage` trait 定义 | 扩展 trait 方法 |
| `la-core/src/storage_types.rs` | `FileMetadata`, `ArchiveMetadata` | 无变更 |
| `la-storage/src/metadata_store.rs` | SQLite MetadataStore 实现 | 重命名为 `SqliteMetadataStore` |
| `la-storage/src/cas.rs` | CAS 实现 | 无变更 |
| `la-storage/src/lib.rs` | 模块导出 | 条件导出新后端 |
| `src/models/state.rs` | `AppState` 定义 | `MetadataStore` → `dyn MetadataStorage` |
| `src/commands/import.rs` | 导入命令 | 后端初始化逻辑 |
| `src/commands/search.rs` | 搜索命令 | 无变更（已通过 trait 解耦） |
| `Cargo.toml` | 依赖管理 | 添加 redb + postcard |
