# 模块架构详解

本文档按分层结构逐模块说明职责边界、核心设计与调用关系。

## 总体分层

```text
┌─────────────────────────────────────────────────────────┐
│                    React 前端层                          │
│   pages / components / hooks / stores / services        │
└───────────────────────┬─────────────────────────────────┘
                        │ Tauri invoke / emit / listen
┌───────────────────────▼─────────────────────────────────┐
│                  Tauri 命令层 (commands/)                │
│   search / workspace / import / watch / performance      │
└───┬───────────────────┬─────────────────────────────────┘
    │                   │
┌───▼──────────┐  ┌─────▼──────────────────────────────────┐
│ 业务服务层    │  │          基础设施层                      │
│ services/    │  │  storage / search_engine / archive /    │
│ monitoring/  │  │  task_manager / state_sync              │
└───┬──────────┘  └─────┬──────────────────────────────────┘
    │                   │
┌───▼───────────────────▼─────────────────────────────────┐
│              Workspace Crates                            │
│   la-core / la-storage / la-search / la-archive         │
└─────────────────────────────────────────────────────────┘
```

---

## 一、Workspace Crates

### 1. la-core — 公共基础层

**路径：** `src-tauri/crates/la-core/src/`

**职责：** 整个项目的基础层，提供所有模块共用的错误类型、领域模型、抽象 Trait 和工具函数。其他 crate 均依赖 la-core，la-core 本身不依赖其他内部 crate。

#### 1.1 错误类型 (`error.rs`)

```rust
pub enum AppError {
    Io(std::io::Error),
    Database(sqlx::Error),
    Search(String),
    Archive(String),
    Serialization(String),
    InvalidInput(String),
    NotFound(String),
    // ...
}

pub type CommandResult<T> = Result<T, CommandError>;
pub struct CommandError { pub message: String, pub code: String }
```

- 使用 `thiserror` 实现 `From` 转换，统一错误冒泡路径
- 使用 `miette` 提供带诊断信息的错误报告，便于调试

#### 1.2 领域模型 (`models/`)

**`models/log_entry.rs`** — 搜索结果核心数据结构

```rust
pub struct LogEntry {
    pub id: usize,                              // 结果集内唯一顺序 ID
    pub timestamp: Arc<str>,                    // 原始时间戳字符串
    pub level: Arc<str>,                        // 日志级别（INFO/WARN/ERROR 等）
    pub file: Arc<str>,                         // 虚拟文件路径
    pub real_path: Arc<str>,                    // CAS 对象路径或原始路径
    pub line: usize,                            // 行号（1-based）
    pub content: Arc<str>,                      // 原始行内容
    pub match_details: Option<Vec<MatchDetail>>,// 命中关键词位置信息
    pub matched_keywords: Option<Vec<String>>,  // 命中的关键词列表
}

pub struct MatchDetail {
    pub keyword: String,
    pub start: usize,    // 字节偏移
    pub end: usize,
}
```

使用 `Arc<str>` 而非 `String`，在大量结果克隆时共享字符串数据，降低内存开销。

**`models/search.rs`** — 查询模型

```rust
pub struct SearchQuery {
    pub terms: Vec<SearchTerm>,
    pub global_operator: QueryOperator,  // AND / OR / NOT
    pub filters: Option<SearchFilters>,
}

pub struct SearchTerm {
    pub value: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub enabled: bool,
}

pub enum QueryOperator { And, Or, Not }
```

**`models/filters.rs`** — 过滤条件

```rust
pub struct SearchFilters {
    pub log_levels: Option<Vec<String>>,  // 日志级别白名单
    pub time_range: Option<TimeRange>,    // 时间范围
    pub file_pattern: Option<String>,     // 文件路径模式（支持通配符）
}

pub struct TimeRange {
    pub start: Option<String>,  // ISO8601 或 datetime-local 格式
    pub end: Option<String>,
}
```

**`models/config.rs`** — 应用配置结构

```rust
pub struct AppConfig {
    pub cache: CacheConfig,
    pub search: SearchConfig,
    pub task_manager: TaskManagerConfig,
}

pub struct CacheConfig {
    pub max_capacity: u64,       // 最大条目数
    pub ttl_seconds: u64,        // 过期时间（秒）
    pub tti_seconds: u64,        // 空闲过期时间（秒）
    pub compression_threshold: usize,  // 启用压缩的大小阈值（字节）
}
```

#### 1.3 抽象 Trait (`traits.rs`)

```rust
pub trait QueryValidation: Send + Sync {
    fn validate(&self, query: &SearchQuery) -> ValidationResult;
}

pub trait ContentStorage: Send + Sync {
    async fn store(&self, content: &[u8]) -> Result<String, AppError>;
    async fn retrieve(&self, hash: &str) -> Result<Vec<u8>, AppError>;
    async fn exists(&self, hash: &str) -> bool;
    async fn delete(&self, hash: &str) -> Result<(), AppError>;
}

pub trait MetadataStorage: Send + Sync {
    async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64, AppError>;
    async fn get_all_files(&self) -> Result<Vec<FileMetadata>, AppError>;
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>, AppError>;
    async fn delete_file(&self, id: i64) -> Result<(), AppError>;
}

pub trait AppConfigProvider: Send + Sync {
    fn config_dir(&self) -> PathBuf;
}
```

Trait 均标注 `Send + Sync`，支持跨线程共享；使用 `async_trait` 宏支持异步方法。

#### 1.4 存储类型 (`storage_types.rs`)

```rust
pub struct FileMetadata {
    pub id: Option<i64>,
    pub sha256_hash: String,        // CAS 内容哈希
    pub virtual_path: String,       // 工作区内虚拟路径
    pub original_name: String,      // 原始文件名
    pub size: i64,                  // 文件大小（字节）
    pub modified_time: i64,         // 修改时间（Unix 时间戳）
    pub parent_archive_id: Option<i64>,  // 所属归档 ID
    pub depth_level: i32,           // 嵌套深度（0 = 顶层）
}

pub struct ArchiveMetadata {
    pub id: Option<i64>,
    pub virtual_path: String,
    pub original_name: String,
    pub archive_type: String,       // zip/tar/gz/rar/7z
    pub file_count: i64,
    pub total_size: i64,
}
```

---

### 2. la-storage — 存储实现层

**路径：** `src-tauri/crates/la-storage/src/`

**职责：** 实现 la-core 中定义的 `ContentStorage` 和 `MetadataStorage` Trait，提供 CAS 对象存储、SQLite 元数据持久化、垃圾回收和缓存监控等完整存储能力。

#### 2.1 CAS 对象存储 (`cas.rs`)

**设计思路：** 仿 Git 对象库，以 SHA-256 哈希前 2 位作为子目录，后续位作为文件名，避免单目录文件过多导致 ext4/NTFS 性能下降。

```text
workspace/
└── objects/
    ├── ab/
    │   └── cdef1234...（完整文件内容）
    ├── 12/
    │   └── 3456abcd...
    └── ...
```

核心实现：

```rust
pub struct ContentAddressableStorage {
    base_dir: PathBuf,
    // 读写共用 Arc，支持并发读取
}

impl ContentStorage for ContentAddressableStorage {
    async fn store(&self, content: &[u8]) -> Result<String> {
        let hash = sha256_hex(content);       // SHA-256 计算
        let (prefix, suffix) = hash.split_at(2);
        let path = self.base_dir.join("objects").join(prefix).join(suffix);
        if !path.exists() {
            // 原子写入：先写临时文件，再重命名，避免并发写入污染
            tokio::fs::write(&tmp_path, content).await?;
            tokio::fs::rename(tmp_path, path).await?;
        }
        Ok(hash)
    }
}
```

**并发安全：** 原子重命名保证写入幂等，并发存储同一内容不会产生冲突。

#### 2.2 SQLite 元数据 (`metadata_store.rs`)

**设计：** 使用 `sqlx` 异步驱动，WAL 模式开启，支持多读单写并发。

核心表结构：

```sql
CREATE TABLE files (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash     TEXT NOT NULL,
    virtual_path    TEXT NOT NULL UNIQUE,
    original_name   TEXT NOT NULL,
    size            INTEGER NOT NULL,
    modified_time   INTEGER NOT NULL,
    parent_archive_id INTEGER REFERENCES archives(id),
    depth_level     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE archives (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    virtual_path    TEXT NOT NULL UNIQUE,
    original_name   TEXT NOT NULL,
    archive_type    TEXT NOT NULL,
    file_count      INTEGER NOT NULL,
    total_size      INTEGER NOT NULL
);

CREATE TABLE search_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    query       TEXT NOT NULL,
    result_count INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE INDEX idx_files_hash ON files(sha256_hash);
CREATE INDEX idx_files_parent ON files(parent_archive_id);
```

`MetadataStore` 主要 API：

```rust
pub struct MetadataStore {
    pool: SqlitePool,  // 连接池，WAL 模式，max_connections=5
}

impl MetadataStore {
    pub async fn get_all_files(&self) -> Result<Vec<FileMetadata>>;
    pub async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>>;
    pub async fn get_files_by_pattern(&self, pattern: &str) -> Result<Vec<FileMetadata>>;
    pub async fn insert_archive(&self, meta: &ArchiveMetadata) -> Result<i64>;
    pub async fn checkpoint(&self) -> Result<()>;  // 应用退出前 WAL checkpoint
}
```

#### 2.3 存储协调器 (`coordinator.rs`)

**职责：** 在同一事务语义下协调 CAS 写入和元数据写入，确保两者一致性。

```rust
pub struct StorageCoordinator {
    cas: Arc<ContentAddressableStorage>,
    metadata: Arc<MetadataStore>,
}

impl StorageCoordinator {
    // 原子化存储一个文件
    pub async fn store_file(
        &self,
        content: &[u8],
        virtual_path: &str,
        parent_archive_id: Option<i64>,
        depth_level: i32,
    ) -> Result<FileMetadata>;
}
```

设计要点：先写 CAS（磁盘文件），再写 SQLite（元数据）；若元数据写入失败，CAS 对象仍保留（由 GC 定期清理孤儿对象）。

#### 2.4 垃圾回收 (`gc.rs`)

**职责：** 定期清理 CAS 中不被任何元数据记录引用的孤儿对象，防止磁盘空间泄漏。

```rust
pub struct GarbageCollector {
    cas_dir: PathBuf,
    metadata: Arc<MetadataStore>,
}

impl GarbageCollector {
    // 扫描 objects/ 目录，与 SQLite 中 sha256_hash 集合取差集
    pub async fn collect(&self) -> Result<GCStats>;
}

pub struct GCStats {
    pub scanned: usize,
    pub deleted: usize,
    pub freed_bytes: u64,
}
```

**GCManager** 封装定时调度逻辑，通过 Tokio interval 按配置周期触发 GC。

#### 2.5 缓存监控 (`cache_monitor.rs`)

**职责：** 追踪 L1（moka 内存缓存）的实时健康指标。

```rust
pub struct CacheMonitor {
    pub hit_count: AtomicU64,
    pub miss_count: AtomicU64,
    pub eviction_count: AtomicU64,
    pub total_load_time_ns: AtomicU64,
}

impl CacheMonitor {
    pub fn hit_rate(&self) -> f64;           // 命中率
    pub fn avg_load_time_ns(&self) -> f64;   // 平均加载时间
    pub fn snapshot(&self) -> CacheSnapshot; // 当前状态快照
}
```

#### 2.6 完整性校验 (`integrity.rs`)

**职责：** 验证工作区存储一致性（CAS 内容与元数据互相匹配）。

```rust
pub async fn verify_workspace(
    cas: &ContentAddressableStorage,
    metadata: &MetadataStore,
) -> Result<IntegrityReport>;

pub struct IntegrityReport {
    pub ok: bool,
    pub missing_objects: Vec<String>,   // 元数据有记录但 CAS 对象不存在
    pub orphan_objects: Vec<String>,    // CAS 对象存在但无元数据
    pub hash_mismatches: Vec<String>,   // 内容哈希与记录不符
}
```

---

### 3. la-search — 搜索基础设施层

**路径：** `src-tauri/crates/la-search/src/`

**职责：** 提供搜索结果的磁盘持久化、虚拟搜索会话管理、Tantivy 全文索引基础设施和高级搜索特性（时间分区索引、自动补全等）。

> **重要说明：** 当前 UI 主搜索链路不经过 Tantivy 索引，而是走 `commands/search.rs` 的文件扫描 + 逐行匹配路径。la-search 中的 Tantivy 相关模块作为预留能力，已在导入时初始化并建索引，可供后续切换。

#### 3.1 磁盘结果存储 (`disk_result_store.rs`)

**设计动机：** 单次搜索可能命中数十万行，全部放内存会导致 OOM。DiskResultStore 将结果序列化为 bincode 写入临时文件，前端按需分页读取。

```rust
pub struct DiskResultStore {
    base_dir: PathBuf,
    // sessions: session_id → 磁盘文件路径
    sessions: DashMap<String, PathBuf>,
}

impl DiskResultStore {
    // 写入一批结果（追加）
    pub async fn write_results(
        &self,
        session_id: &str,
        entries: &[LogEntry],
    ) -> Result<()>;

    // 分页读取
    pub async fn get_page(
        &self,
        session_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<LogEntry>>;

    // 获取结果总数（读取文件头部元数据）
    pub async fn get_total_count(&self, session_id: &str) -> Result<usize>;

    // 清理会话（应用退出时调用）
    pub async fn cleanup_session(&self, session_id: &str) -> Result<()>;
}
```

**文件格式：**

```text
[u64: total_count][bincode(LogEntry)][bincode(LogEntry)]...
```

使用 `DashMap` 避免会话查找时锁竞争。

#### 3.2 虚拟搜索会话管理 (`virtual_search_manager.rs`)

**职责：** 管理搜索会话的生命周期和统计信息，作为 DiskResultStore 的上层封装。

```rust
pub struct VirtualSearchManager {
    sessions: DashMap<String, SearchSession>,
}

pub struct SearchSession {
    pub session_id: String,
    pub query: String,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub created_at: Instant,
    pub expires_at: Instant,
    pub status: SessionStatus,
}

pub enum SessionStatus { Running, Completed, Cancelled, Failed }
```

**会话过期：** 默认 30 分钟过期，通过 Tokio 后台任务定期清理。提供 `cleanup_expired_search_sessions` 命令供前端主动触发。

#### 3.3 流式索引构建 (`streaming_builder.rs`)

**职责：** 支持以流式方式向 Tantivy 索引追加文档，避免一次性加载全部内容。

```rust
pub struct StreamingIndexBuilder {
    writer: IndexWriter,
    schema: Schema,
    buffer_size: usize,
    pending_docs: usize,
}

impl StreamingIndexBuilder {
    pub fn add_document(&mut self, entry: &LogEntry) -> Result<()>;
    // 每 N 个文档自动提交一次，避免 writer buffer 溢出
    pub async fn flush_if_needed(&mut self) -> Result<()>;
    pub async fn commit(self) -> Result<()>;
}
```

#### 3.4 高级搜索特性 (`advanced_features.rs`)

**时间分区索引（TimePartitionedIndex）：**

```rust
pub struct TimePartitionedIndex {
    // 按日期分片，每个分片是独立的 Tantivy 索引
    partitions: BTreeMap<NaiveDate, Index>,
    schema: LogSchema,
}
```

适用于日志按日期分布稀疏、需要快速定位特定日期范围的场景。

**过滤引擎（FilterEngine）：**

```rust
pub struct FilterEngine {
    level_filter: Option<HashSet<String>>,
    time_filter: Option<TimeRange>,
    pattern_filter: Option<Regex>,
}

impl FilterEngine {
    pub fn matches(&self, entry: &LogEntry) -> bool;
}
```

**正则搜索引擎（RegexSearchEngine）：**

```rust
pub struct RegexSearchEngine {
    pattern: Regex,
    max_results: usize,
}
```

**自动补全引擎（AutocompleteEngine）：**

```rust
pub struct AutocompleteEngine {
    // 基于 BTreeSet 的前缀索引，存储历史搜索词
    prefixes: BTreeSet<String>,
}

impl AutocompleteEngine {
    pub fn suggest(&self, prefix: &str, limit: usize) -> Vec<String>;
}
```

#### 3.5 查询优化器 (`query_optimizer.rs`)

**职责：** 分析查询特征，生成最优执行计划。

```rust
pub struct QueryOptimizer;

impl QueryOptimizer {
    pub fn optimize(&self, query: &SearchQuery) -> QueryPlan;
}

pub struct QueryPlan {
    pub steps: Vec<PlanStep>,
    pub estimated_cost: f64,
    pub use_index: bool,
    pub use_aho_corasick: bool,
}
```

#### 3.6 高亮引擎 (`highlighting_engine.rs`)

**职责：** 根据 `MatchDetail` 中的字节偏移信息，在原始行内容中标注命中位置。

```rust
pub struct HighlightingEngine {
    max_context_chars: usize,  // 命中词前后保留的字符数
}

impl HighlightingEngine {
    pub fn highlight(&self, content: &str, matches: &[MatchDetail]) -> HighlightedLine;
}

pub struct HighlightedLine {
    pub segments: Vec<Segment>,
}

pub enum Segment {
    Normal(String),
    Highlighted { text: String, keyword: String },
}
```

#### 3.7 索引 Schema (`schema.rs`)

```rust
pub struct LogSchema {
    pub content: Field,     // 日志行内容（全文索引）
    pub level: Field,       // 日志级别（FAST 字段，用于过滤）
    pub timestamp: Field,   // Unix 时间戳（FAST 字段，用于范围查询）
    pub file_path: Field,   // 文件路径（存储字段）
    pub line_num: Field,    // 行号（存储字段）
}
```

---

### 4. la-archive — 归档处理层

**路径：** `src-tauri/crates/la-archive/src/`

**职责：** 统一处理 ZIP / TAR / GZ / RAR / 7Z 等压缩格式，支持嵌套归档递归解压，提供路径安全校验、断点续传和进度追踪。

#### 4.1 解压编排器 (`extraction_orchestrator.rs`)

**核心入口，设计为递归解压控制器。**

```rust
pub struct ExtractionOrchestrator {
    security: SecurityDetector,
    path_manager: PathManager,
    checkpoint_manager: CheckpointManager,
    max_depth: u32,
}

impl ExtractionOrchestrator {
    pub async fn extract(
        &self,
        archive_path: &Path,
        output_dir: &Path,
        ctx: &mut ExtractionContext,
        progress_tx: mpsc::Sender<ExtractionProgress>,
    ) -> Result<Vec<ExtractedFile>>;
}
```

**递归逻辑：**

1. 检测压缩格式（按魔数 magic bytes 判断）
2. 调用对应格式 Handler 解压
3. 对解压出的每个文件：若是归档格式 → 递归调用 `extract()`（深度限制）
4. 通过 `ctx.depth` 防止无限递归（默认最大深度 10）

#### 4.2 格式 Handler（统一实现 `ArchiveHandler` Trait）

```rust
#[async_trait]
pub trait ArchiveHandler: Send + Sync {
    fn can_handle(&self, path: &Path) -> bool;
    async fn extract(
        &self,
        archive_path: &Path,
        output_dir: &Path,
        ctx: &ExtractionContext,
    ) -> Result<Vec<ExtractedFile>>;
}
```

| Handler | 库 | 说明 |
|---------|-----|------|
| `ZipHandler` | `async_zip` | 异步 ZIP 解压，支持大文件 |
| `TarHandler` | `tokio-tar` | 异步 TAR 解压 |
| `GzHandler` | `flate2` | GZIP 解压（单文件） |
| `RarHandler` | `unrar`（可选 feature） | RAR 格式，依赖系统库 |
| `SevenZHandler` | `sevenz-rust` | 7-Zip 格式 |

**格式检测优先级：** 魔数检测 > 扩展名检测（防止扩展名被改）

```rust
// 魔数示例
const ZIP_MAGIC: &[u8] = &[0x50, 0x4B, 0x03, 0x04];
const GZIP_MAGIC: &[u8] = &[0x1F, 0x8B];
const RAR_MAGIC: &[u8] = &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07];
```

#### 4.3 解压上下文 (`extraction_context.rs`)

```rust
pub struct ExtractionContext {
    pub depth: u32,                  // 当前递归深度
    pub parent_archive_stack: Vec<String>,  // 父归档路径栈
    pub total_extracted_size: u64,   // 已解压总大小（zip 炸弹检测）
    pub start_time: Instant,
}

impl ExtractionContext {
    pub fn push_archive(&mut self, path: String) -> Result<()>;
    pub fn pop_archive(&mut self);
    pub fn check_depth_limit(&self, max: u32) -> Result<()>;
    pub fn add_size(&mut self, bytes: u64) -> Result<()>;  // 检测大小限制
}
```

#### 4.4 路径管理器 (`path_manager.rs`)

**职责：** 构建虚拟路径、处理编码问题、跨平台路径分隔符统一。

```rust
pub struct PathManager {
    workspace_root: PathBuf,
}

impl PathManager {
    // 构造虚拟路径：archive_path + "/" + entry_path
    pub fn build_virtual_path(
        &self,
        archive_virtual_path: &str,
        entry_name: &str,
    ) -> String;

    // 规范化路径分隔符（统一为 /）
    pub fn normalize_separators(path: &str) -> String;

    // 检测并修复 CP437/GBK/UTF-8 编码问题（使用 encoding_rs）
    pub fn decode_path_bytes(bytes: &[u8]) -> String;
}
```

#### 4.5 安全检测器 (`security_detector.rs`)

**职责：** 防御路径穿越攻击和 zip 炸弹。

```rust
pub struct SecurityDetector {
    pub zip_bomb_ratio_threshold: f64,  // 默认 100.0（压缩比超过即告警）
    pub max_total_size: u64,
}

impl SecurityDetector {
    // 检测路径穿越：含 ../ 或绝对路径
    pub fn check_path_traversal(&self, entry_path: &str) -> Result<()>;

    // 检测 zip 炸弹：压缩前大小 / 压缩后大小 > threshold
    pub fn check_zip_bomb(
        &self,
        compressed_size: u64,
        uncompressed_size: u64,
    ) -> Result<()>;
}
```

#### 4.6 断点续传 (`checkpoint_manager.rs`)

```rust
pub struct CheckpointManager {
    checkpoint_path: PathBuf,
}

impl CheckpointManager {
    // 保存当前解压进度到磁盘
    pub async fn save(&self, state: &ExtractionState) -> Result<()>;
    // 恢复进度
    pub async fn load(&self) -> Result<Option<ExtractionState>>;
    pub async fn clear(&self) -> Result<()>;
}

pub struct ExtractionState {
    pub archive_path: PathBuf,
    pub processed_entries: HashSet<String>,
    pub total_size_processed: u64,
}
```

---

## 二、后端主 Crate 模块

### 5. commands/ — Tauri IPC 命令层

**路径：** `src-tauri/src/commands/`

**职责：** 将业务逻辑暴露为 Tauri `invoke` 命令，负责参数解析、错误转换、AppState 管理和事件发送。每个命令函数对应一个 IPC 端点，通过 `#[tauri::command]` 宏标注。

命令层不包含业务逻辑，只做调用协调和错误包装。

#### 5.1 搜索命令 (`search.rs`)

**核心命令：**

```rust
#[tauri::command]
pub async fn search_logs(
    state: State<'_, AppState>,
    workspace_id: String,
    query: String,
    filters: Option<SearchFilters>,
) -> CommandResult<SearchStartResult>;

#[tauri::command]
pub async fn fetch_search_page(
    state: State<'_, AppState>,
    search_id: String,
    offset: usize,
    limit: usize,
) -> CommandResult<SearchPageResult>;

#[tauri::command]
pub async fn cancel_search(
    state: State<'_, AppState>,
    search_id: String,
) -> CommandResult<()>;
```

**`search_logs` 内部流程：**

1. 从 `AppState` 懒恢复工作区运行态（`workspace_dirs`、CAS、MetadataStore）
2. 调用 `QueryValidator::validate()` 校验查询
3. 构建 `CompiledSearchFilters`（过滤条件预编译，包含 Regex、时间解析）
4. 从 `MetadataStore` 获取文件列表，按 `filePattern` 早筛
5. 为每个文件：从 CAS 读取内容 → 检测编码 → 调用 `QueryExecutor`
6. 命中行经时间/级别过滤后写入 `DiskResultStore`
7. 返回 `{ search_id, total_count }`

**分段摘要优化（仅启用时间/级别过滤时激活）：**

```text
每 256 行建一个轻量分段摘要 SegmentSummary：
  - 级别位图（ERROR/WARN/INFO/DEBUG/TRACE 各 1 位）
  - 可解析时间戳的 min/max
若整个分段与过滤条件不可能相交，跳过该分段的 match_with_details 调用。
```

#### 5.2 工作区命令 (`workspace.rs`)

```rust
#[tauri::command]
pub async fn create_workspace(state, name, path) -> CommandResult<WorkspaceInfo>;
#[tauri::command]
pub async fn load_workspace(state, workspace_id) -> CommandResult<WorkspaceInfo>;
#[tauri::command]
pub async fn refresh_workspace(state, workspace_id) -> CommandResult<WorkspaceInfo>;
#[tauri::command]
pub async fn delete_workspace(state, workspace_id) -> CommandResult<()>;
#[tauri::command]
pub async fn get_workspace_time_range(state, workspace_id) -> CommandResult<TimeRange>;
#[tauri::command]
pub async fn get_workspace_status(state, workspace_id) -> CommandResult<WorkspaceStatus>;
```

工作区路径约定（统一，避免新旧路径分叉）：

```text
{app_data_dir}/workspaces/{workspace_id}/
  objects/       ← CAS 对象目录
  metadata.db    ← SQLite 元数据库
```

`load_workspace` 在内存中没有运行态时，自动从磁盘恢复 CAS + MetadataStore + SearchEngineManager。

#### 5.3 导入命令 (`import.rs`)

```rust
#[tauri::command]
pub async fn import_folder(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    workspace_id: String,
    folder_path: String,
) -> CommandResult<ImportResult>;

#[tauri::command]
pub async fn check_rar_support() -> CommandResult<bool>;
```

**导入流程：**

1. 扫描目录树，调用 `FileTypeFilter` 识别日志文件
2. 调用 `la-archive::ExtractionOrchestrator` 处理压缩文件（递归解压）
3. 每个文件：`StorageCoordinator::store_file()`（CAS + 元数据）
4. 初始化并注册 `SearchEngineManager`，批量建 Tantivy 索引
5. 通过 `state_sync` 广播 `import-progress` 事件

#### 5.4 异步搜索命令 (`async_search.rs`)

```rust
#[tauri::command]
pub async fn async_search_logs(state, workspace_id, query, filters) -> CommandResult<String>;
#[tauri::command]
pub async fn cancel_async_search(state, search_id) -> CommandResult<()>;
#[tauri::command]
pub async fn get_active_searches_count(state) -> CommandResult<usize>;
```

与 `search_logs` 的区别：通过 `TaskManager` 调度，支持后台执行多个搜索任务。

#### 5.5 文件监听命令 (`watch.rs`)

```rust
#[tauri::command]
pub async fn start_watch(state, app_handle, workspace_id, paths) -> CommandResult<()>;
#[tauri::command]
pub async fn stop_watch(state, workspace_id) -> CommandResult<()>;
```

监听器检测到文件变化后：

1. `TimestampParser` 解析新增行的时间戳
2. 新内容写入 CAS
3. 追加 Tantivy 索引
4. 通过 Tauri `emit` 发送 `file-change` 事件

#### 5.6 配置命令 (`config.rs`)

管理应用全局配置的持久化，涵盖缓存、搜索引擎、文件过滤和任务管理器配置。配置存储于 `{config_dir}/config.json`。

#### 5.7 性能监控命令 (`performance.rs`)

```rust
#[tauri::command]
pub async fn get_performance_metrics(state) -> CommandResult<PerformanceMetrics>;
#[tauri::command]
pub async fn get_historical_metrics(state, start, end) -> CommandResult<Vec<MetricSnapshot>>;
#[tauri::command]
pub async fn get_aggregated_metrics(state, period) -> CommandResult<AggregatedMetrics>;
```

#### 5.8 虚拟文件树命令 (`virtual_tree.rs`)

```rust
#[tauri::command]
pub async fn get_virtual_file_tree(state, workspace_id) -> CommandResult<Vec<TreeNode>>;
#[tauri::command]
pub async fn read_file_by_hash(state, hash) -> CommandResult<String>;
```

文件树从 `MetadataStore` 的 `virtual_path` 字段重建，通过 `CAS::retrieve()` 读取文件内容。

---

### 6. services/ — 业务逻辑层

**路径：** `src-tauri/src/services/`

**职责：** 实现搜索匹配、查询规划/验证、文件监听、类型识别等核心业务逻辑，不直接涉及 IPC。

#### 6.1 查询执行器 (`query_executor.rs`)

**核心职责：** 对单个文件的文本内容执行逐行匹配，返回命中行及命中详情。

```rust
pub struct QueryExecutor {
    regex_engine: Arc<dyn RegexEngine>,
}

impl QueryExecutor {
    pub fn execute(
        &self,
        content: &str,
        query: &SearchQuery,
        compiled_filters: &CompiledSearchFilters,
    ) -> Vec<(usize, LogEntry)>;  // (line_number, entry)
}
```

**OR 多关键词 Aho-Corasick 快速预检：**

当查询满足以下条件时，启用 Aho-Corasick 快速预检引擎：
- `global_operator == OR`
- 至少 2 个启用的非正则 term
- 所有 term 大小写敏感设置一致

```text
行 → Aho-Corasick 多模式一次扫描（快速判断是否命中任意关键词）
   ↓ 命中
→ 逐 term 正则/字符串匹配（提取 MatchDetail，保留高亮位置信息）
   ↓
→ 构建 LogEntry（包含 match_details 和 matched_keywords）
```

未命中行仅经过 Aho-Corasick 一次线性扫描，避免反复执行多次单词匹配。

#### 6.2 查询规划器 (`query_planner.rs`)

**职责：** 分析查询结构，生成 `ExecutionPlan`，指导 `QueryExecutor` 选择最优匹配策略。

```rust
pub struct QueryPlanner;

impl QueryPlanner {
    pub fn plan(&self, query: &SearchQuery) -> ExecutionPlan;
}

pub struct ExecutionPlan {
    pub steps: Vec<PlanStep>,
    pub use_aho_corasick: bool,      // 是否启用多模式预检
    pub fast_or_engine: Option<AhoCorasick>,  // 预构建的 AC 自动机
    pub estimated_complexity: QueryComplexity,
}

pub enum QueryComplexity { Simple, Moderate, Complex }
```

**策略选择逻辑：**

```text
全部非正则 + OR + ≥2 term → AhoCorasick 引擎
含正则 term              → StandardRegex 引擎
AND 操作符               → 顺序执行每个 term
```

#### 6.3 查询验证器 (`query_validator.rs`)

**职责：** 在查询执行前校验合法性，快速失败。

```rust
pub struct QueryValidator;

impl QueryValidation for QueryValidator {
    fn validate(&self, query: &SearchQuery) -> ValidationResult;
}
```

校验规则：
- 查询长度 ≤ 1024 字符
- 正则语法合法性（预编译验证）
- term 数量 ≤ 50
- 非空白字符存在

#### 6.4 正则引擎 (`regex_engine.rs`)

**职责：** 提供可切换的多种正则匹配实现。

```rust
pub trait RegexEngine: Send + Sync {
    fn is_match(&self, text: &str, pattern: &str) -> bool;
    fn find_all(&self, text: &str, pattern: &str) -> Vec<MatchDetail>;
}

pub struct StandardEngine {
    cache: Mutex<LruCache<String, Regex>>,  // LRU 缓存预编译正则
}

pub struct AhoCorasickEngine {
    // 多模式，不支持正则语法，但速度更快
    automaton: AhoCorasick,
}

pub struct AutomataEngine;  // 基于 roaring bitset 的实验性实现

pub struct RegexEngineManager {
    standard: StandardEngine,
    aho_corasick: AhoCorasickEngine,
}

impl RegexEngineManager {
    pub fn select_engine(&self, query: &SearchQuery) -> &dyn RegexEngine;
}
```

`StandardEngine` 使用 LRU 缓存避免相同正则重复编译，线程安全。

#### 6.5 文件监听服务 (`file_watcher.rs`)

```rust
pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    tx: mpsc::Sender<FileChangeEvent>,
}

pub struct TimestampParser {
    formats: Vec<&'static str>,
}

impl TimestampParser {
    // 支持格式：
    // - RFC3339: 2024-01-15T10:30:45Z
    // - datetime-local: 2024-01-15T10:30
    // - 常见日志格式: 2024-01-15 10:30:45.123
    // - Unix 秒/毫秒时间戳
    pub fn parse(&self, s: &str) -> Option<NaiveDateTime>;

    // 专用于 search filters 的 datetime-local 解析
    pub fn parse_naive_datetime(&self, s: &str) -> Option<NaiveDateTime>;
}
```

#### 6.6 文件类型过滤器 (`file_type_filter.rs`)

**职责：** 在导入时识别文件类型，过滤掉非日志的二进制文件。

```rust
pub struct FileTypeFilter {
    log_patterns: Vec<Regex>,        // 文件名模式
    binary_extensions: HashSet<String>,  // 已知二进制扩展名黑名单
}

pub enum ImportDecision {
    Include,  // 确认为文本/日志文件
    Exclude,  // 确认为二进制文件
    Unknown,  // 魔数检测无法确定，按配置处理
}

impl FileTypeFilter {
    // 综合：扩展名 + 魔数检测 + 文件名模式
    pub fn decide(&self, path: &Path, content_preview: &[u8]) -> ImportDecision;
}
```

---

### 7. storage/ — 存储适配层

**路径：** `src-tauri/src/storage/`

**职责：** 后端主 crate 内的存储适配代码，与 `la-storage` crate 协同工作，提供面向应用的高层存储接口，包括缓存管理和 CAS 读取封装。

```rust
pub struct CacheManager {
    l1_cache: Cache<String, Vec<u8>>,   // moka 异步缓存（内容缓存）
    monitor: Arc<CacheMonitor>,
}

impl CacheManager {
    // 从缓存读取；未命中则从 CAS 加载并写入缓存
    pub async fn get_or_load(
        &self,
        hash: &str,
        cas: &ContentAddressableStorage,
    ) -> Result<Vec<u8>>;
}
```

---

### 8. search_engine/ — 搜索引擎适配层

**路径：** `src-tauri/src/search_engine/`

**职责：** 后端主 crate 内对 `la-search` 的适配封装，包含 `SearchEngineManager`（Tantivy 索引生命周期管理）和搜索结果缓存。

```rust
pub struct SearchEngineManager {
    index: Index,
    writer: Mutex<IndexWriter>,
    reader: IndexReader,
    schema: LogSchema,
}

impl SearchEngineManager {
    pub async fn add_document(&self, entry: &LogEntry) -> Result<()>;
    pub async fn commit(&self) -> Result<()>;
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    pub fn get_time_range(&self) -> Result<(i64, i64)>;  // min/max timestamp
}
```

---

### 9. task_manager/ — 后台任务调度

**路径：** `src-tauri/src/task_manager/`

**职责：** 基于 Actor 模型的后台任务调度系统，管理导入、异步搜索等耗时操作的生命周期。

```rust
// Actor 消息类型
pub enum TaskMessage {
    Start { task_id: String, payload: TaskPayload },
    Cancel { task_id: String },
    Status { task_id: String, reply: oneshot::Sender<TaskStatus> },
    Shutdown,
}

pub struct TaskManager {
    sender: mpsc::Sender<TaskMessage>,
    active_tasks: Arc<DashMap<String, TaskHandle>>,
}

impl TaskManager {
    pub async fn submit<F>(&self, task_id: String, fut: F) -> Result<()>
    where F: Future<Output = Result<()>> + Send + 'static;

    pub async fn cancel(&self, task_id: &str) -> Result<()>;
    pub async fn shutdown(&self);  // 应用退出时优雅关闭
}
```

**取消机制：** 使用 `tokio_util::sync::CancellationToken`，任务内部定期检查取消信号。

```rust
pub struct TaskHandle {
    pub cancellation_token: CancellationToken,
    pub join_handle: JoinHandle<Result<()>>,
    pub status: Arc<Mutex<TaskStatus>>,
}
```

---

### 10. monitoring/ — 性能指标采集

**路径：** `src-tauri/src/monitoring/`

**职责：** 采集搜索延迟、导入吞吐量、缓存命中率等性能指标，持久化到 SQLite，支持历史查询和聚合统计。

```rust
pub struct MetricsStore {
    db: SqlitePool,
}

impl MetricsStore {
    pub async fn record_search_event(&self, event: SearchEvent) -> Result<()>;
    pub async fn get_search_events(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<SearchEvent>>;
    pub async fn get_aggregated_metrics(&self, period: Period) -> Result<AggregatedMetrics>;
    pub async fn cleanup_old_metrics(&self, before: i64) -> Result<()>;
}

pub struct SearchEvent {
    pub query: String,
    pub workspace_id: String,
    pub result_count: usize,
    pub duration_ms: u64,
    pub cache_hit: bool,
    pub created_at: i64,
}

pub struct AggregatedMetrics {
    pub avg_query_time_ms: f64,
    pub p95_query_time_ms: f64,
    pub total_searches: u64,
    pub cache_hit_rate: f64,
}
```

---

### 11. state_sync/ — 前后端状态同步

**路径：** `src-tauri/src/state_sync/`

**职责：** 管理从后端到前端的实时状态推送，包括工作区变更通知、导入进度、搜索事件等，并维护事件历史以支持前端重连后回放。

```rust
pub struct StateSync {
    app_handle: AppHandle,
    event_history: Arc<Mutex<VecDeque<StateEvent>>>,
    max_history: usize,  // 默认保留最近 100 个事件
}

impl StateSync {
    pub fn broadcast(&self, event: StateEvent) -> Result<()>;
    pub fn get_event_history(&self) -> Vec<StateEvent>;
}

pub enum StateEvent {
    WorkspaceUpdated { workspace_id: String },
    ImportProgress { workspace_id: String, progress: f32, message: String },
    SearchStarted { search_id: String },
    SearchCompleted { search_id: String, result_count: usize },
    FileChanged { workspace_id: String, path: String },
}
```

---

### 12. 核心状态容器 (`state.rs`)

**路径：** `src-tauri/src/state.rs`

**职责：** `AppState` 是整个后端的依赖注入根，通过 `Arc<Mutex<_>>` 和 `Arc<DashMap<_>>` 管理所有跨命令共享的运行态资源。

```rust
pub struct AppState {
    // 工作区路径映射（workspace_id → 磁盘路径）
    pub workspace_dirs: Arc<Mutex<BTreeMap<String, PathBuf>>>,

    // CAS 实例池（每个工作区一个独立 CAS）
    pub cas_instances: Arc<Mutex<HashMap<String, Arc<ContentAddressableStorage>>>>,

    // SQLite 元数据存储池
    pub metadata_stores: Arc<Mutex<HashMap<String, Arc<MetadataStore>>>>,

    // 后台任务调度器
    pub task_manager: Arc<Mutex<Option<TaskManager>>>,

    // 搜索取消令牌（search_id → CancellationToken）
    pub search_cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,

    // L1 内存缓存管理器
    pub cache_manager: Arc<Mutex<CacheManager>>,

    // 前后端状态同步通道
    pub state_sync: Arc<Mutex<Option<StateSync>>>,

    // Tantivy 索引管理器池（每个工作区一个）
    pub search_engine_managers: Arc<Mutex<HashMap<String, Arc<SearchEngineManager>>>>,

    // 虚拟搜索会话管理器（跨搜索共享）
    pub virtual_search_manager: Arc<VirtualSearchManager>,

    // 磁盘结果分页存储（跨搜索共享）
    pub disk_result_store: Arc<DiskResultStore>,

    // 性能指标存储
    pub metrics_store: Arc<MetricsStore>,
}
```

**锁策略：**

- `DashMap` 用于高频读写的映射（搜索取消令牌等）
- `parking_lot::Mutex` 代替 `std::sync::Mutex`（更小开销）
- `Arc` 包装所有共享资源，保证 Tauri 命令跨线程安全

---

## 三、前端模块

### 13. pages/ — 页面组件

| 页面 | 文件 | 职责 |
|------|------|------|
| 搜索 | `SearchPage.tsx` | 主搜索入口，无限滚动分页，关键词高亮 |
| 工作区 | `WorkspacesPage.tsx` | 工作区创建/加载/删除，导入归档 |
| 关键词 | `KeywordsPage.tsx` | 搜索历史与频率统计 |
| 任务 | `TasksPage.tsx` | 导入/导出后台任务监控 |
| 性能 | `PerformancePage.tsx` | 搜索延迟、缓存命中率等指标图表 |
| 设置 | `SettingsPage.tsx` | 缓存/搜索/日志级别配置 |

**`SearchPage.tsx` 分页机制：**

```text
useInfiniteSearch hook
  → TanStack Query useInfiniteQuery
  → 每次 fetchNextPage 调用 api.fetchSearchPage(search_id, offset, limit)
  → 虚拟滚动渲染（仅渲染可视区域内的行）
```

### 14. services/ — IPC 封装层

**`api.ts`** — 所有 Tauri IPC 调用的类型安全封装：

```typescript
export const api = {
  searchLogs: (params: SearchParams) =>
    invoke<SearchStartResult>('search_logs', params),

  fetchSearchPage: (searchId: string, offset: number, limit: number) =>
    invoke<SearchPageResult>('fetch_search_page', { searchId, offset, limit }),

  cancelSearch: (searchId: string) =>
    invoke<void>('cancel_search', { searchId }),

  createWorkspace: (name: string, path: string) =>
    invoke<WorkspaceInfo>('create_workspace', { name, path }),
  // ...
};
```

**`SearchQueryBuilder.ts`** — 将 UI 输入构建为后端 `SearchQuery` 结构。

### 15. hooks/ — React 自定义 Hook

| Hook | 职责 |
|------|------|
| `useInfiniteSearch` | TanStack Query 无限分页搜索 |
| `useWorkspaceSelection` | 工作区选择状态 |
| `useWorkspaceList` | 工作区列表（含刷新） |
| `useTauriEventListeners` | Tauri 事件订阅（import-progress 等） |
| `useToast` | 全局 Toast 通知 |

### 16. stores/ — Zustand 状态仓库

| 仓库 | 状态 |
|------|------|
| `appStore` | 全局应用状态（当前工作区、加载状态） |
| `workspaceStore` | 工作区列表与选中状态 |
| `taskStore` | 后台任务列表与进度 |
| `keywordStore` | 搜索历史与统计 |

---

## 四、真实搜索链路（当前行为）

```text
SearchPage.tsx: 用户输入 "error|timeout"

→ api.searchLogs({ workspaceId, query: "error|timeout", filters })
→ commands/search.rs: search_logs()
  1. 懒恢复工作区运行态（若重启后内存无记录）
  2. QueryValidator: 校验 query 长度/语法
  3. CompiledSearchFilters: 预编译过滤条件
     - 级别过滤 → HashSet<String>（已小写化）
     - 时间过滤 → Option<(NaiveDateTime, NaiveDateTime)>
     - 文件模式 → Option<Regex>（通配符转正则）
  4. MetadataStore::get_all_files() → Vec<FileMetadata>
  5. 文件级早筛（filePattern 过滤）
  6. 对每个候选文件：
     a. CAS::retrieve(hash) → 读取内容字节
     b. encoding_rs 检测编码，解码为 UTF-8
     c. QueryPlanner::plan() → 检测多关键词 OR 条件
        → 构建 fast_or_engine: AhoCorasick(["error","timeout"])
     d. QueryExecutor::execute():
        - 仅时间/级别过滤激活时：每 256 行建 SegmentSummary
          → 分段与过滤条件不相交 → 跳过整段
        - 对每行：fast_or_engine.is_match(line)?
          → Yes → 逐 term 匹配，提取 MatchDetail
          → No  → 跳过
     e. 命中行：时间/级别过滤（行级）
     f. 构建 LogEntry { id: 全局顺序递增, ... }
  7. DiskResultStore::write_results(search_id, entries)
  8. 返回 { search_id, total_count: 1234 }

→ 前端收到 search_id，启动 useInfiniteSearch
→ fetch_search_page(search_id, 0, 50) → 第一页 50 条
→ fetch_search_page(search_id, 50, 50) → 第二页 50 条
→ ...（虚拟滚动触发加载）
```

---

## 五、导入链路

```text
WorkspacesPage.tsx: 用户选择文件夹

→ api.importFolder(workspaceId, folderPath)
→ commands/import.rs: import_folder()
  1. 初始化工作区目录（若不存在）
  2. 创建 CAS + MetadataStore
  3. 扫描目录树
  4. FileTypeFilter 过滤非日志文件
  5. 对每个文件：
     - 普通文件：StorageCoordinator::store_file()
     - 压缩文件：ExtractionOrchestrator::extract()
       → SecurityDetector: 路径穿越检测
       → 递归解压，每个解压文件 → StorageCoordinator::store_file()
  6. 注册 SearchEngineManager（每个工作区一个 Tantivy 索引）
  7. 从 CAS + MetadataStore 批量回填 Tantivy 索引
  8. StateSync::broadcast(ImportProgress)
  9. 返回 ImportResult { total_files, total_size }
```

---

## 六、模块依赖关系

```text
la-core
  ↑ 依赖
la-storage ←─── la-core (error, models, traits, storage_types)
la-search  ←─── la-core (error, models, log_entry)
la-archive ←─── la-core (error)

主 crate (src-tauri/src/)
  ├── commands/ ←── services/, storage/, search_engine/, task_manager/
  ├── services/ ←── la-core (models, traits)
  ├── storage/  ←── la-storage, la-core
  ├── search_engine/ ←── la-search, la-core
  └── archive/  ←── la-archive, la-core
```

---

## 七、阅读建议

按以下顺序阅读代码可以最快建立整体认知：

1. `la-core/src/models/` — 领域模型定义
2. `src-tauri/src/commands/search.rs` — 主搜索命令实现
3. `src-tauri/src/services/query_executor.rs` — 匹配核心
4. `la-storage/src/cas.rs` + `metadata_store.rs` — 存储实现
5. `la-archive/src/extraction_orchestrator.rs` — 归档处理
6. `src/pages/SearchPage.tsx` — 前端搜索 UI
