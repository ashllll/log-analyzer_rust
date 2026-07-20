# CAS 存储架构

本文档说明项目存储层的核心设计：内容寻址存储（Content-Addressable Storage，CAS）、SQLite 元数据库、搜索读取链路，以及各组件的职责边界。

## 为什么使用 CAS

传统"路径映射"方案将文件内容与导入路径绑定，存在以下问题：

- 重复文件（相同内容，不同路径）占用多份磁盘空间
- 嵌套压缩包解压后产生大量临时路径，需要长期维护
- 文件移动或重命名会破坏搜索索引与内容的对应关系

CAS 的核心思路：**以内容哈希（SHA-256）为唯一标识，内容只存一份，虚拟路径独立维护。**

优势：

| 特性 | 说明 |
|------|------|
| 内容去重 | 相同内容只写入一次，哈希相同则直接复用 |
| 路径解耦 | 虚拟路径存在 SQLite 中，可独立修改不影响内容 |
| 嵌套归档支持 | 压缩包内的文件通过虚拟路径层级关系追踪，无需保留临时目录 |
| 稳定数据源 | 搜索、虚拟树、导出均从 CAS 读取，不依赖原始导入路径存在 |

---

## 核心组成

### 1. CAS 对象存储

**实现位置：** `la-storage/src/cas.rs`

**磁盘布局：**

```text
{app_data_dir}/workspaces/{workspace_id}/
├── objects/
│   ├── ab/
│   │   └── cdef1234567890abcdef...（文件内容，文件名为完整 SHA-256 后 62 位）
│   ├── 12/
│   │   └── 3456abcdef...
│   └── ...（按哈希前 2 位分 256 个子目录）
└── metadata.db
```

分 256 个子目录是参考 Git 对象库的做法，避免单目录文件数过多导致 ext4/NTFS 文件系统性能下降（Linux ext4 单目录推荐 < 100k 文件）。

**写入流程（幂等）：**

```text
1. 计算 content 的 SHA-256：hash = "abcdef1234..."
2. 构建路径：objects/ab/cdef1234...
3. 若路径已存在 → 直接返回 hash（不重复写）
4. 先写临时文件 objects/ab/cdef1234....tmp
5. 原子重命名为 objects/ab/cdef1234...
6. 返回 hash
```

原子重命名保证并发写入同一内容不会产生损坏文件（两个进程同时写同一 hash，最终结果幂等）。

**读取：**

```rust
pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>> {
    let (prefix, suffix) = hash.split_at(2);
    let path = self.base_dir.join("objects").join(prefix).join(suffix);
    Ok(tokio::fs::read(path).await?)
}
```

另提供 `read_content_sync`（阻塞读取，供 `spawn_blocking` 中的搜索路径使用）与 `read_content_mmap_sync`（mmap 零拷贝）。

### 2. SQLite 元数据库

**实现位置：** `la-storage/src/metadata_store/`

**数据库路径：** `{workspace_dir}/metadata.db`

**运行模式：** WAL（Write-Ahead Log），支持多读单写并发，读取不阻塞写入。

**核心表结构：**

```sql
-- 文件记录（每个导入文件对应一行）
CREATE TABLE files (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash       TEXT NOT NULL UNIQUE,     -- CAS 对象标识（唯一）
    virtual_path      TEXT NOT NULL,            -- 工作区内虚拟路径
    original_name     TEXT NOT NULL,            -- 原始文件名
    size              INTEGER NOT NULL,         -- 文件大小（字节）
    modified_time     INTEGER NOT NULL,         -- 修改时间（Unix 时间戳）
    mime_type         TEXT,                     -- MIME 类型
    parent_archive_id INTEGER REFERENCES archives(id) ON DELETE CASCADE,
    depth_level       INTEGER NOT NULL DEFAULT 0, -- 嵌套深度（0=顶层）
    created_at        INTEGER NOT NULL          -- 创建时间（Unix 时间戳）
);

-- 归档文件记录（压缩包本身）
CREATE TABLE archives (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash       TEXT NOT NULL UNIQUE,     -- 归档内容哈希
    virtual_path      TEXT NOT NULL,            -- 虚拟路径
    original_name     TEXT NOT NULL,            -- 原始文件名
    archive_type      TEXT NOT NULL,            -- zip/tar/gz/rar/7z
    parent_archive_id INTEGER,                  -- 父归档 ID（嵌套归档）
    depth_level       INTEGER NOT NULL DEFAULT 0, -- 嵌套深度
    extraction_status TEXT NOT NULL,            -- 提取状态
    created_at        INTEGER NOT NULL,         -- 创建时间（Unix 时间戳）
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- 性能索引
CREATE INDEX idx_files_hash           ON files(sha256_hash);
CREATE INDEX idx_files_virtual_path   ON files(virtual_path);
CREATE INDEX idx_files_parent_archive ON files(parent_archive_id);
CREATE INDEX idx_files_depth          ON files(depth_level);
CREATE INDEX idx_archives_virtual_path ON archives(virtual_path);
CREATE INDEX idx_archives_parent      ON archives(parent_archive_id);
CREATE INDEX idx_archives_hash        ON archives(sha256_hash);
CREATE INDEX idx_archives_depth       ON archives(depth_level);

-- FTS5 全文搜索虚拟表
CREATE VIRTUAL TABLE files_fts USING fts5(virtual_path, original_name);
```

**虚拟路径约定：**

```text
顶层文件：       /logs/app.log
压缩包内文件：   /archives/service.zip/service/2024-01-15.log
嵌套压缩包内：   /archives/all.tar.gz/service.zip/service/2024-01-15.log
```

层级深度（`depth_level`）记录：
- 0 = 直接导入的文件或归档
- 1 = 压缩包内的文件
- 2 = 嵌套压缩包内的文件
- 以此类推

**应用退出时 WAL checkpoint：**

```rust
// main.rs 的 RunEvent::ExitRequested 处理中
metadata_store.close().await;
// close() 内部执行 PRAGMA wal_checkpoint(RESTART)，
// 将 WAL 文件合并回主数据库，确保下次启动读到完整数据
```

### 3. 导入写入协调

**实现位置：** `la-archive/src/processor.rs`（`process_path_with_cas*`），由 `WorkspaceServiceImpl` 的 `ImportService` 调用。

CAS 与元数据的写入没有独立的协调器抽象，由导入流水线直接编排：

```text
process_path_with_cas(path, ...)
  ↓
1. cas.store_file_zero_copy(path) / cas.store_content(&content) → hash
2. 构建 FileMetadata { sha256_hash: hash, virtual_path, depth_level, ... }
3. metadata_store.insert_file(&metadata)（批量路径用 insert_files_batch_smart）
```

**一致性策略：**

- 先写 CAS（磁盘文件）：即使元数据写入失败，CAS 对象保留，不丢内容
- 再写 SQLite（元数据）：若失败，CAS 对象成为孤儿（当前版本暂无自动 GC，见下文"垃圾回收"一节）
- 不使用分布式事务，依赖写入顺序保证"读路径不会遇到缺失对象"

**为什么不先写 SQLite？**

若先写元数据再写 CAS，且 CAS 写入失败，则 SQLite 中存在指向不存在对象的记录，后续读取时会产生 "object not found" 错误，比"孤儿对象"更难处理。

---

## 搜索读取链路

### 当前主链路（文件扫描 + 逐行匹配）

```text
search_logs(workspace_id, query, filters)
  ↓
1. MetadataStore::get_all_files()
   → SELECT * FROM files WHERE depth_level >= 0
   → 返回 Vec<FileMetadata>（包含 sha256_hash 和 virtual_path）

2. 文件级早筛（filePattern 过滤）
   → 过滤掉虚拟路径不匹配的文件（减少 CAS 读取次数）

3. 逐文件处理（SearchBatch 分批，在 spawn_blocking 中执行）：
   a. CasLogFileRepository::read_content_sync(sha256_hash) → Vec<u8>（原始字节）
      → 直接从 CAS 磁盘读取（当前版本无文件内容缓存层）
   b. encoding_rs 检测编码（UTF-8/GBK/CP437 等），解码为 &str
   c. QueryEngineLogSearcher 逐行匹配（regex / Aho-Corasick / memchr）
   d. 命中行构建 LogEntry（包含 virtual_path 作为 file 字段）

4. DiskResultStore::append_entries(search_id, entries)（经 SearchSessionManager）
   → 序列化为 bincode，追加写入临时文件

5. 返回 { search_id, total_count }
```

### CAS 对象存在性缓存

CAS 内部维护一个 moka LRU 缓存（`existence_cache: Cache<String, ()>`，见 `cas.rs`）：

```text
key:   SHA-256 hash
value: ()（仅标记"该对象已确认存在"）
策略:  LRU + TTL/TTI
作用:  exists() 与写入前去重检查走缓存快路径，避免每次 store 都做 stat 系统调用
```

注意边界：

- 该缓存只加速"对象是否存在"的判断，**不缓存文件内容**；搜索读取仍直接从磁盘读（由 OS page cache 兜底）
- 缓存命中后仍会校验文件真实存在，能容忍外部删除导致的过期条目（见 cas.rs 的 cache consistency 测试）

---

## 导入链路

```text
import_folder(workspace_id, folder_path)
  ↓
扫描目录树
  ↓ 对每个文件
┌─────────────────────────────────────────────────────┐
│ 普通文件（文本/日志）                                 │
│   FileTypeFilter::should_import_file_safe() == true   │
│   → cas.store_file_zero_copy() → objects/ab/cdef...   │
│   → metadata_store.insert_file()   → SQLite           │
└─────────────────────────────────────────────────────┘
  ↓ 压缩文件
┌─────────────────────────────────────────────────────┐
│ 压缩文件（.zip/.tar/.gz/.rar/.7z）                   │
│   ExtractionOrchestrator::extract()                  │
│     SecurityDetector: 路径穿越检测、zip 炸弹检测      │
│     → 递归解压，对每个解压文件：                      │
│       → cas.store_content() + insert_file()          │
│         depth_level++，parent_archive_id = 父归档 ID │
└─────────────────────────────────────────────────────┘
  ↓ 导入完成
SearchEngineManager 批量建索引（Tantivy）
  → 遍历 MetadataStore 中所有文件
  → 每文件从 CAS 读取内容，解析日志行，写入 Tantivy
  → 定期 commit（每批次），完成后 commit_and_wait_merge
```

---

## CAS 与压缩包的关系

压缩包处理（`la-archive`）与 CAS 存储（`la-storage`）职责分离：

| 组件 | 职责 |
|------|------|
| `la-archive` | 识别格式、安全解压、递归展开嵌套归档 |
| `la-storage` | 保存解压后的实际内容（按 SHA-256），维护虚拟路径和归档层级关系 |

压缩包本身也会被写入 CAS（保留原始归档内容），并在 `archives` 表中记录元数据。解压出的每个文件通过 `parent_archive_id` 引用所属归档。

**嵌套归档虚拟路径示例：**

```text
导入文件: backup.tar.gz
  → archives 表: { id: 1, virtual_path: "/backup.tar.gz", archive_type: "tar.gz" }

解压出: service-logs.zip (depth=1)
  → archives 表: { id: 2, virtual_path: "/backup.tar.gz/service-logs.zip", parent=1 }

再解压出: app.log (depth=2)
  → files 表: {
      sha256_hash: "abcd...",
      virtual_path: "/backup.tar.gz/service-logs.zip/app.log",
      parent_archive_id: 2,
      depth_level: 2
    }
```

---

## 垃圾回收（GC）

**当前状态：尚未实现。**

当前版本没有自动垃圾回收：元数据写入失败或中途取消导入时产生的孤儿对象（CAS 有、SQLite 无引用）不会被自动清理。删除整个工作区会连带删除其 CAS 目录（`delete_workspace` → `remove_dir_all`），可整体回收该工作区的存储空间。

设计方向（预留）：

- 增量 GC：分片游标轮询 `objects/` 下的 shard 目录，每批处理固定数量对象，低 I/O 占用
- 全量 GC：一次性扫描全部对象，用于手动触发或异常恢复
- 安全边界：只删除孤儿对象，不删除任何有元数据记录的对象

---

## 存储完整性校验

**触发方式：** 通过性能监控命令手动触发，或 CI 测试中调用

```text
verify_workspace_integrity(cas, metadata)
  1. 遍历 SQLite 中所有文件记录
  2. 检查 CAS 中对应对象是否存在
  3. 检查对象内容的 SHA-256 是否与记录匹配
  4. 返回 IntegrityReport { total_files, valid_files, invalid_files,
                           missing_objects, corrupted_objects, warnings, ... }
```

---

## 当前边界

| 说明 | 当前状态 |
|------|---------|
| CAS 是搜索的内容来源 | 是，主搜索链路直接从 CAS 读文件内容 |
| SQLite 用于全文搜索 | 否，SQLite 只存元数据，全文搜索走 CAS 扫描 |
| Tantivy 是主搜索引擎 | 否，Tantivy 索引导入时建立，但当前主搜索走 CAS 扫描路径 |
| 导入时建立 Tantivy 索引 | 是，导入完成后批量回填 Tantivy |
| 多工作区支持 | 是，每个工作区有独立的 CAS 目录和 metadata.db |

---

## 相关文档

- [文档首页](../index.md)
- [文档索引](../README.md)
