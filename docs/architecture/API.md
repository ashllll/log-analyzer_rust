# API Documentation

## Overview

This document describes the API interfaces for the Log Analyzer application, which uses a Content-Addressable Storage (CAS) architecture for efficient file management and search.

## Table of Contents

- [Backend API](#backend-api)
  - [CAS Storage](#cas-storage)
  - [Metadata Store](#metadata-store)
  - [Search Statistics](#search-statistics)
  - [Tauri Commands](#tauri-commands)
  - [Tauri Events](#tauri-events)
- [Frontend API](#frontend-api)
  - [TypeScript Types](#typescript-types)
  - [React Components](#react-components)
  - [Hooks](#hooks)
- [Architecture Overview](#architecture-overview)
- [Migration from Legacy Format](#migration-from-legacy-format)
- [Performance Characteristics](#performance-characteristics)
- [Error Handling](#error-handling)
- [Testing](#testing)

---

## Backend API

### CAS Storage

The Content-Addressable Storage system provides Git-style object storage with automatic deduplication.

**Location**: `src-tauri/src/storage/cas.rs`

**Key Methods**:

```rust
impl ContentAddressableStorage {
    /// Create a new CAS instance
    pub fn new(workspace_dir: PathBuf) -> Self;
    
    /// Compute SHA-256 hash from bytes
    pub fn compute_hash(content: &[u8]) -> String;
    
    /// Store content and return hash
    pub async fn store_content(&self, content: &[u8]) -> Result<String>;
    
    /// Store file using streaming (memory-efficient for large files)
    pub async fn store_file_streaming(&self, path: &Path) -> Result<String>;
    
    /// Read content by hash
    pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>>;
    
    /// Check if hash exists in storage
    pub fn exists(&self, hash: &str) -> bool;
    
    /// Get object path from hash
    pub fn get_object_path(&self, hash: &str) -> PathBuf;
}
```

**Example Usage**:
```rust
let cas = ContentAddressableStorage::new(workspace_dir);

// Store a file
let hash = cas.store_file_streaming(&file_path).await?;

// Read content later
let content = cas.read_content(&hash).await?;
```

---

### Metadata Store

SQLite-based metadata storage with FTS5 full-text search support.

**Location**: `src-tauri/src/storage/metadata_store.rs`

**Key Methods**:

```rust
impl MetadataStore {
    /// Initialize database and create tables
    pub async fn new(workspace_dir: &Path) -> Result<Self>;
    
    /// Insert file metadata
    pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64>;
    
    /// Insert archive metadata
    pub async fn insert_archive(&self, metadata: &ArchiveMetadata) -> Result<i64>;
    
    /// Search files using FTS5
    pub async fn search_files(&self, query: &str) -> Result<Vec<FileMetadata>>;
    
    /// Get file by virtual path
    pub async fn get_file_by_virtual_path(&self, path: &str) -> Result<Option<FileMetadata>>;
    
    /// Get all files in workspace
    pub async fn get_all_files(&self) -> Result<Vec<FileMetadata>>;
    
    /// Get archive children
    pub async fn get_archive_children(&self, archive_id: i64) -> Result<Vec<FileMetadata>>;
    
    /// Delete workspace data
    pub async fn delete_workspace(&self) -> Result<()>;
}
```

**Data Models**:

```rust
pub struct FileMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub size: i64,
    pub modified_time: i64,
    pub mime_type: Option<String>,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
}

pub struct ArchiveMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub archive_type: String,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
    pub extraction_status: String,
}
```

---

### Search Statistics

Statistics for multi-keyword search results.

**Location**: `src-tauri/src/models/search_statistics.rs`

> **注意**: `KeywordStatistics` 和 `SearchResultSummary` 使用 `#[serde(rename = "...")]` 将 Rust 的 snake_case 字段序列化为 camelCase JSON。这是为了与前端 TypeScript 类型保持一致（前端使用 camelCase）。

#### `KeywordStatistics`

Represents statistics for a single keyword.

**Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordStatistics {
    /// The keyword text
    pub keyword: String,
    
    /// Number of log entries matching this keyword
    #[serde(rename = "matchCount")]
    pub match_count: usize,
    
    /// Percentage of total matches (0-100)
    #[serde(rename = "matchPercentage")]
    pub match_percentage: f32,
}
```

**Methods**:
```rust
impl KeywordStatistics {
    /// Create a new keyword statistics entry
    pub fn new(keyword: String, match_count: usize, total_matches: usize) -> Self
}
```

#### `SearchResultSummary`

Contains summary information about a search operation.

**Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResultSummary {
    /// Total number of matching log entries
    #[serde(rename = "totalMatches")]
    pub total_matches: usize,
    
    /// Statistics for each keyword
    #[serde(rename = "keywordStats")]
    pub keyword_stats: Vec<KeywordStatistics>,
    
    /// Search duration in milliseconds
    #[serde(rename = "searchDurationMs")]
    pub search_duration_ms: u64,
    
    /// Whether results were truncated due to limit
    pub truncated: bool,
}
```

---

### Tauri Commands

<!-- AUTO-GENERATED - 从 src-tauri/src/commands/ 提取 -->

所有命令通过 `invoke('command_name', params)` 调用。注意前端参数使用 **camelCase**（对应 Rust 端 `#[allow(non_snake_case)]` 标注）。

#### 搜索命令 (`commands/search.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `search_logs` | `query, workspaceId?, max_results?, filters?` | `Vec<LogEntry>` | 全文搜索日志 |
| `cancel_search` | `searchId` | `()` | 取消进行中的搜索 |
| `search_logs_paged` | `query, page_size?, page_index, searchId?, workspaceId?` | `PagedSearchResult` | 分页搜索 |
| `fetch_search_page` | `search_id, offset, limit` | `SearchPageResult` | 获取分页搜索结果页 |
| `cleanup_paged_search_cache` | `max_age_secs?` | `usize` | 清理过期分页缓存 |
| `get_paged_search_cache_stats` | - | `JSON` | 获取分页缓存统计 |
| `register_search_session` | `search_id, query, entries` | `String` | 注册搜索会话 |
| `get_search_session_info` | `search_id` | `Option<JSON>` | 获取会话信息 |
| `get_search_total_count` | `search_id` | `usize` | 获取搜索结果总数 |
| `remove_search_session` | `search_id` | `bool` | 移除搜索会话 |
| `cleanup_expired_search_sessions` | `max_age_secs?` | `usize` | 清理过期会话 |
| `get_virtual_search_stats` | - | `JSON` | 获取虚拟搜索统计 |

#### 导入命令 (`commands/import.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `import_folder` | `path, workspaceId` | `String` | 导入文件/文件夹到工作区 |
| `check_rar_support` | - | `JSON` | 检查 RAR 格式支持状态 |

#### 工作区命令 (`commands/workspace.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `load_workspace` | `workspaceId` | `WorkspaceLoadResponse` | 加载工作区 |
| `delete_workspace` | `workspaceId` | `()` | 删除工作区 |

#### 虚拟文件树命令 (`commands/virtual_tree.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `read_file_by_hash` | `workspaceId, hash` | `String` | 通过哈希读取文件内容 |
| `get_virtual_file_tree` | `workspaceId` | `Vec<VirtualTreeNode>` | 获取虚拟文件树 |

#### 文件监听命令 (`commands/watch.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `start_watch` | `workspaceId, path` | `()` | 开始监听文件变化 |
| `stop_watch` | `workspaceId` | `()` | 停止监听 |

#### 导出命令 (`commands/export.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `export_results` | `results, format, savePath` | `()` | 导出搜索结果 |

#### 配置命令 (`commands/config.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `load_config` | - | `AppConfig` | 加载应用配置 |
| `save_config` | `config: AppConfig` | `()` | 保存应用配置 |

#### 性能监控命令 (`commands/performance.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `get_performance_metrics` | - | `PerformanceMetrics` | 获取性能指标 |
| `get_historical_metrics` | `range: TimeRangeDto` | `HistoricalMetricsData` | 获取历史指标 |
| `get_aggregated_metrics` | `range, interval_seconds` | `Vec<MetricsSnapshot>` | 获取聚合指标 |
| `get_search_events` | `range, workspaceId?` | `Vec<SearchEvent>` | 获取搜索事件 |
| `get_metrics_stats` | - | `MetricsStoreStats` | 获取指标统计 |
| `cleanup_metrics_data` | - | `MetricsStoreStats` | 清理指标数据 |

#### 缓存命令 (`commands/cache.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `invalidate_workspace_cache` | `workspaceId` | `usize` | 清除工作区缓存 |

#### 状态同步命令 (`commands/state_sync.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `init_state_sync` | - | `()` | 初始化状态同步 |
| `get_workspace_state` | `workspaceId` | `Option<WorkspaceState>` | 获取工作区状态 |

#### 错误报告命令 (`commands/error_reporting.rs`)

| 命令 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `report_frontend_error` | `report: FrontendErrorReport` | `()` | 上报前端错误 |
| `submit_user_feedback` | `feedback: UserFeedback` | `()` | 提交用户反馈 |
| `get_error_statistics` | - | `ErrorStatistics` | 获取错误统计 |

---

### Tauri Events

<!-- AUTO-GENERATED - 从 src-tauri/src/events/constants.rs 提取 -->

所有事件名称使用 **kebab-case**，通过 `listen('event-name', callback)` 监听。

#### 搜索事件

| 事件名称 | 描述 | Payload 类型 |
|----------|------|-------------|
| `search-start` | 搜索开始 | `{ query: string }` |
| `search-progress` | 搜索进度更新 | `{ progress: number }` |
| `search-results` | 搜索结果数据 | `Vec<LogEntry>` |
| `search-summary` | 搜索摘要统计 | `SearchResultSummary` |
| `search-complete` | 搜索完成 | `{ result_count: number }` |
| `search-error` | 搜索错误 | `{ error: string }` |
| `search-cancelled` | 搜索取消 | `{}` |

#### 异步搜索事件

| 事件名称 | 描述 |
|----------|------|
| `async-search-start` | 异步搜索开始 |
| `async-search-progress` | 异步搜索进度更新 |
| `async-search-results` | 异步搜索结果数据 |
| `async-search-complete` | 异步搜索完成 |
| `async-search-error` | 异步搜索错误 |

#### 分页搜索事件

| 事件名称 | 描述 |
|----------|------|
| `paged-search-results` | 分页搜索结果 |
| `paged-search-meta` | 分页搜索元数据 |

#### 任务事件

| 事件名称 | 描述 | Payload 类型 |
|----------|------|-------------|
| `task-update` | 任务进度更新 | `TaskProgress` |
| `import-complete` | 导入完成 | `{ workspace_id, file_count }` |

#### 文件监控事件

| 事件名称 | 描述 |
|----------|------|
| `file-changed` | 文件变化通知 |
| `new-logs` | 新日志条目通知 |

#### 系统事件（通常不转发到前端）

| 事件名称 | 描述 |
|----------|------|
| `system-error` | 系统错误 |
| `system-warning` | 系统警告 |
| `system-info` | 系统信息 |

#### `search-summary` Payload 示例

```json
{
  "totalMatches": 100,
  "keywordStats": [
    {
      "keyword": "error",
      "matchCount": 60,
      "matchPercentage": 60.0
    },
    {
      "keyword": "timeout",
      "matchCount": 55,
      "matchPercentage": 55.0
    }
  ],
  "searchDurationMs": 156,
  "truncated": false
}
```

---

## Frontend API

### TypeScript Types

<!-- AUTO-GENERATED - 从 src/types/ 和 src/services/ 提取 -->

#### `LogEntry`

**Location**: `src/types/api-responses.ts`

```typescript
export type LogEntry = {
  id: string;
  timestamp: string;
  level: string;
  file: string;
  real_path: string;
  line: number;
  content: string;
  tags: string[];
  match_details?: MatchDetail[];
  matched_keywords?: string[];
};
```

#### `MatchDetail`

**Location**: `src/types/api-responses.ts`

```typescript
export type MatchDetail = {
  term_id: string;
  term_value: string;
  priority: number;
  match_position?: [number, number];
};
```

#### `WorkspaceLoadResponse`

**Location**: `src/types/api-responses.ts`

```typescript
export type WorkspaceLoadResponseValidated = {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'PROCESSING' | 'OFFLINE';
  fileCount?: number;
  totalSize?: number;
};
```

#### `WorkspaceState`

**Location**: `src/types/api-responses.ts`

```typescript
export type WorkspaceState = {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'PROCESSING' | 'OFFLINE' | 'ERROR';
  last_accessed?: number;
};
```

#### `VirtualFileNode`

**Location**: `src/types/api-responses.ts`

```typescript
export type VirtualFileNode = {
  name: string;
  path: string;
  is_directory: boolean;
  size?: number;
  children?: VirtualFileNode[];
};
```

#### `SearchResultSummary`

**Location**: `src/types/search.ts`

```typescript
export interface SearchResultSummary {
  totalMatches: number;
  keywordStats: KeywordStatistics[];
  searchDurationMs: number;
  truncated: boolean;
}
```

#### `KeywordStatistics`

**Location**: `src/types/search.ts`

```typescript
export interface KeywordStatistics {
  keyword: string;
  matchCount: number;
  matchPercentage: number;
}
```

#### `KeywordStat`

**Location**: `src/types/search.ts`

```typescript
export interface KeywordStat extends KeywordStatistics {
  color: string;
}
```

#### `PerformanceMetrics`

**Location**: `src/types/api-responses.ts`

```typescript
export type PerformanceMetrics = {
  searchLatency: { current, average, p95, p99 };
  searchThroughput: { current, average, peak };
  cacheMetrics: { hitRate, missCount, hitCount, size, capacity, evictions };
  memoryMetrics: { used, total, heapUsed, external };
  taskMetrics: { total, running, completed, failed, averageDuration };
  indexMetrics: { totalFiles, indexedFiles, totalSize, indexSize };
};
```

#### `FileFilterConfig`

**Location**: `src/types/api-responses.ts`

```typescript
export type FileFilterConfig = {
  enabled: boolean;
  binary_detection_enabled: boolean;
  mode: 'whitelist' | 'blacklist';
  filename_patterns: string[];
  allowed_extensions: string[];
  forbidden_extensions: string[];
};
```

#### `SearchQuery`

**Location**: `src/types/search.ts`

```typescript
export interface SearchQuery {
  id: string;
  terms: SearchTerm[];
  globalOperator: QueryOperator;
  filters?: SearchFilters;
  metadata: QueryMetadata;
}
```

---

### React Components

#### `KeywordStatsPanel`

Displays keyword statistics in a collapsible panel.

**Location**: `src/components/search/KeywordStatsPanel.tsx`

**Props**:
```typescript
interface KeywordStatsPanelProps {
  keywords: Array<{
    value: string;
    matchCount: number;
    color: string;
  }>;
  totalMatches: number;
  searchDurationMs: number;
}
```

**Example Usage**:
```tsx
import { KeywordStatsPanel } from '@/components/search/KeywordStatsPanel';

function SearchPage() {
  const [keywordStats, setKeywordStats] = useState<KeywordStat[]>([]);
  const [totalMatches, setTotalMatches] = useState(0);
  const [searchDuration, setSearchDuration] = useState(0);

  return (
    <KeywordStatsPanel
      keywords={keywordStats}
      totalMatches={totalMatches}
      searchDurationMs={searchDuration}
    />
  );
}
```

---

### Hooks

#### Event Listeners

**Example: Listen for search summary**:
```typescript
import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { SearchResultSummary } from '@/types/search';

function SearchPage() {
  const [summary, setSummary] = useState<SearchResultSummary | null>(null);

  useEffect(() => {
    const unlisten = listen<SearchResultSummary>('search-summary', (event) => {
      setSummary(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  return (
    // ... render component with summary data
  );
}
```

---

## Architecture Overview

### CAS-Based Search Flow

```
User Query
    ↓
MetadataStore.search_files() → FTS5 Index Query
    ↓
List of FileMetadata (with SHA-256 hashes)
    ↓
For each file:
    CAS.read_content(hash) → Read file content
    ↓
    Search content for keywords
    ↓
    Collect matches
    ↓
Return SearchResults + emit search-summary event
```

### Import Flow

```
User selects file/folder
    ↓
Detect if archive → Extract if needed
    ↓
For each file:
    Compute SHA-256 hash (streaming)
    ↓
    Check if exists in CAS
    ↓
    If not exists: Store in objects/ab/cdef...
    ↓
    Insert metadata to SQLite
    ↓
    Emit import-progress event
    ↓
Complete → Emit workspace-updated event
```

---

## Migration from Legacy Format

### Detection

The application automatically detects legacy workspaces (using old path_map format) and prompts for migration.

**Detection Criteria**:
- Presence of `.idx.gz` files
- Absence of `metadata.db` and `objects/` directory

### Migration Process

1. Read old path mappings
2. For each file:
   - Read content
   - Compute SHA-256 hash
   - Store in CAS
   - Insert metadata to SQLite
3. Verify all files accessible
4. Mark workspace as migrated

---

## Performance Characteristics

### CAS Operations

- **Hash computation**: ~50 MB/s (streaming)
- **Deduplication check**: O(1) - hash lookup
- **Storage write**: Limited by disk I/O

### Metadata Operations

- **FTS5 search**: ~100,000 records/second
- **Hash-based retrieval**: O(1) - direct file access
- **Metadata insert**: ~10,000 records/second

### Memory Usage

- **Streaming hash**: 8 KB buffer (constant)
- **SQLite connection**: ~10 MB per workspace
- **CAS cache**: Configurable (default: 100 MB)

---

## Error Handling

All API functions return `Result<T, String>` or `Result<T, AppError>`.

**Common Error Types**:

- `WorkspaceNotFound` - Workspace ID doesn't exist
- `FileNotFound` - File hash not found in CAS
- `DatabaseError` - SQLite operation failed
- `IOError` - File system operation failed
- `InvalidHash` - Malformed SHA-256 hash

**Example Error Handling**:

```rust
match cas.read_content(&hash).await {
    Ok(content) => { /* process content */ },
    Err(e) => {
        error!("Failed to read content: {}", e);
        return Err(format!("File not found: {}", hash));
    }
}
```

---

## Testing

### Backend Tests

Run tests with:
```bash
cd log-analyzer/src-tauri
cargo test
```

Key test modules:
- `storage::cas::tests` - CAS operations
- `storage::metadata_store::tests` - SQLite operations
- `archive::processor::tests` - Archive processing
- `commands::tests` - Tauri command integration

### Frontend Tests

Run tests with:
```bash
cd log-analyzer
npm test
```

---

## Related Documentation

- [CAS Architecture](./CAS_ARCHITECTURE.md) - Detailed CAS design
- [Module Architecture](./modules/MODULE_ARCHITECTURE.md) - Complete module documentation
- [Advanced Search Features](./ADVANCED_SEARCH_FEATURES_EXPLANATION.md) - Search capabilities
- [User Guide](../guides/MULTI_KEYWORD_SEARCH_GUIDE.md) - End-user documentation
- [Contributing Guide](../CONTRIB.md) - Development workflow

---

## Support

For questions or issues:
- Create a GitHub issue with the `api` label
- Check the documentation for usage examples
- Review the CHANGELOG for recent changes
