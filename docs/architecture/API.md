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

#### Workspace Management

```rust
/// Import a file or folder into a workspace
#[tauri::command]
pub async fn import_path(
    path: String,
    workspace_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String>

/// Delete a workspace
#[tauri::command]
pub async fn delete_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<(), String>

/// Get workspace information
#[tauri::command]
pub async fn get_workspace_info(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceInfo, String>
```

#### Search Commands

```rust
/// Search logs in a workspace
#[tauri::command]
pub async fn search_logs(
    workspace_id: String,
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<LogEntry>, String>

/// Get file content by hash
#[tauri::command]
pub async fn get_file_content(
    workspace_id: String,
    file_hash: String,
    state: State<'_, AppState>,
) -> Result<String, String>
```

#### Legacy Detection

```rust
/// Detect if a workspace uses legacy format
#[tauri::command]
pub async fn detect_legacy_workspace(
    workspace_path: String,
) -> Result<LegacyDetectionResult, String>
```

---

### Tauri Events

#### `search-summary`

Emitted after a search operation completes, providing keyword statistics.

**Payload Type**: `SearchResultSummary`

**Example Payload**:
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

#### `import-progress`

Emitted during file import to report progress.

**Payload Type**:
```typescript
{
  workspace_id: string;
  progress: number;  // 0-100
  current_file: string;
  total_files: number;
  processed_files: number;
}
```

#### `workspace-updated`

Emitted when workspace data changes.

**Payload Type**:
```typescript
{
  workspace_id: string;
  action: 'created' | 'updated' | 'deleted';
}
```

---

## Frontend API

### TypeScript Types

#### `WorkspaceInfo`

**Location**: `src/types/common.ts`

```typescript
export interface WorkspaceInfo {
  id: string;
  name: string;
  path: string;
  created_at: number;
  file_count: number;
  total_size: number;
  status: 'ready' | 'processing' | 'error';
}
```

#### `FileMetadata`

```typescript
export interface FileMetadata {
  id: number;
  sha256Hash: string;
  virtualPath: string;
  originalName: string;
  size: number;
  modifiedTime: number;
  mimeType?: string;
  parentArchiveId?: number;
  depthLevel: number;
}
```

#### `SearchResultSummary`

**Location**: `src/types/search.ts`

```typescript
export interface SearchResultSummary {
  totalMatches: number;
  keywordStats: Array<{
    keyword: string;
    matchCount: number;
    matchPercentage: number;
  }>;
  searchDurationMs: number;
  truncated: boolean;
}
```

#### `KeywordStat`

```typescript
export interface KeywordStat {
  value: string;
  matchCount: number;
  color: string;
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
- [Migration Guide](../MIGRATION_GUIDE.md) - Legacy format migration
- [Troubleshooting](../TROUBLESHOOTING.md) - Common issues
- [User Guide](../guides/MULTI_KEYWORD_SEARCH_GUIDE.md) - End-user documentation

---

## Support

For questions or issues:
- Create a GitHub issue with the `api` label
- Check the documentation for usage examples
- Review the CHANGELOG for recent changes
