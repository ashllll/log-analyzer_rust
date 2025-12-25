# Nested Archive Processing - Industry-Standard Design

## Overview

本文档描述了处理嵌套压缩包和路径长度限制的完整解决方案，基于以下业内成熟方案：

- **7-Zip**: 使用短路径名和内容寻址存储
- **Git LFS**: 使用 SHA-256 哈希作为文件标识符
- **Docker**: 使用层级存储和内容寻址
- **WinRAR**: 使用扁平化存储结构避免路径嵌套

## Problem Statement

### 路径长度限制

**Windows**: 260 字符 (MAX_PATH)
**Linux**: 4096 字符
**macOS**: 1024 字符

### 嵌套压缩包场景

```
app_logs.zip
├── 2024-01-01_logs.zip
│   ├── server_logs.zip
│   │   ├── application.log
│   │   └── error.log
│   └── client_logs.zip
│       └── debug.log
└── 2024-01-02_logs.zip
    └── system.log
```

### 传统方案的问题

**简单嵌套解压**:
```
C:\Users\...\AppData\extracted\workspace_123\
  app_logs_zip_1234567890\
    2024-01-01_logs_zip_1234567891\
      server_logs_zip_1234567892\
        application.log  ❌ 路径超过 260 字符
```

## Industry-Standard Solution: Content-Addressable Storage (CAS)

### 核心概念

参考 Git LFS 和 Docker 的设计，使用**内容寻址存储**：

1. **扁平化存储**: 所有文件存储在扁平目录结构中
2. **哈希标识**: 使用 SHA-256 哈希作为文件名
3. **元数据映射**: 维护哈希到原始路径的映射表
4. **虚拟文件系统**: 前端展示完整的嵌套结构

### 架构图

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React)                      │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Virtual File Tree (用户看到的嵌套结构)            │ │
│  │  app_logs.zip/                                     │ │
│  │    ├── 2024-01-01_logs.zip/                       │ │
│  │    │   ├── server_logs.zip/                       │ │
│  │    │   │   └── application.log                    │ │
│  └────────────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────────────┘
                       │ IPC
                       ▼
┌─────────────────────────────────────────────────────────┐
│                Backend (Tauri/Rust)                      │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Archive Metadata Store (SQLite)                   │ │
│  │  ┌──────────────────────────────────────────────┐ │ │
│  │  │ file_id | sha256_hash | virtual_path | ...   │ │ │
│  │  │ 1       | a3f2...     | app_logs.zip/...     │ │ │
│  │  │ 2       | b7e1...     | app_logs.zip/...     │ │ │
│  │  └──────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────┘ │
│                       │                                  │
│                       ▼                                  │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Content-Addressable Storage (扁平化存储)          │ │
│  │  workspace_123/objects/                            │ │
│  │    ├── a3/f2e1... (application.log 的内容)        │ │
│  │    ├── b7/e145... (error.log 的内容)              │ │
│  │    └── c9/a234... (debug.log 的内容)              │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## Detailed Design

### 1. Content-Addressable Storage Layer

#### 存储结构

```
AppData/
└── log-analyzer/
    └── workspaces/
        └── {workspace_id}/
            ├── metadata.db          # SQLite 数据库
            └── objects/             # 内容存储（扁平化）
                ├── a3/
                │   └── f2e1d4c5...  # SHA-256 哈希的前2位作为目录
                ├── b7/
                │   └── e145a3b2...
                └── c9/
                    └── a234f1e8...
```

#### 哈希计算

使用 Rust 的 `sha2` crate（业内标准）：

```rust
use sha2::{Sha256, Digest};

pub struct ContentAddressableStorage {
    workspace_dir: PathBuf,
}

impl ContentAddressableStorage {
    /// 计算文件内容的 SHA-256 哈希
    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
    
    /// 存储文件内容，返回哈希值
    pub async fn store_content(&self, content: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(content);
        let object_path = self.get_object_path(&hash);
        
        // 创建目录（使用哈希的前2位）
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // 写入文件（如果不存在）
        if !object_path.exists() {
            fs::write(&object_path, content).await?;
        }
        
        Ok(hash)
    }
    
    /// 根据哈希获取文件路径
    fn get_object_path(&self, hash: &str) -> PathBuf {
        // Git 风格：使用前2位作为目录名
        let (prefix, suffix) = hash.split_at(2);
        self.workspace_dir
            .join("objects")
            .join(prefix)
            .join(suffix)
    }
    
    /// 读取文件内容
    pub async fn read_content(&self, hash: &str) -> Result<Vec<u8>> {
        let object_path = self.get_object_path(hash);
        fs::read(&object_path).await
            .map_err(|e| anyhow!("Failed to read object {}: {}", hash, e))
    }
}
```

### 2. Metadata Store (SQLite)

#### 数据库模式

```sql
-- 文件元数据表
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash TEXT NOT NULL UNIQUE,
    virtual_path TEXT NOT NULL,
    original_name TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_time INTEGER NOT NULL,
    mime_type TEXT,
    parent_archive_id INTEGER,
    depth_level INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- 压缩包元数据表
CREATE TABLE archives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash TEXT NOT NULL UNIQUE,
    virtual_path TEXT NOT NULL,
    original_name TEXT NOT NULL,
    archive_type TEXT NOT NULL,  -- zip, rar, tar, etc.
    parent_archive_id INTEGER,
    depth_level INTEGER NOT NULL DEFAULT 0,
    extraction_status TEXT NOT NULL,  -- pending, extracting, completed, failed
    created_at INTEGER NOT NULL,
    FOREIGN KEY (parent_archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- 虚拟路径索引（加速查询）
CREATE INDEX idx_files_virtual_path ON files(virtual_path);
CREATE INDEX idx_files_parent_archive ON files(parent_archive_id);
CREATE INDEX idx_archives_virtual_path ON archives(virtual_path);
CREATE INDEX idx_archives_parent ON archives(parent_archive_id);

-- 全文搜索索引
CREATE VIRTUAL TABLE files_fts USING fts5(
    virtual_path,
    original_name,
    content='files',
    content_rowid='id'
);
```

#### Rust 实现

使用 `sqlx` crate（业内标准的异步 SQL 库）：

```rust
use sqlx::{SqlitePool, Row};

pub struct MetadataStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
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

impl MetadataStore {
    pub async fn new(workspace_dir: &Path) -> Result<Self> {
        let db_path = workspace_dir.join("metadata.db");
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display())).await?;
        
        // 初始化数据库模式
        sqlx::query(include_str!("schema.sql"))
            .execute(&pool)
            .await?;
        
        Ok(Self { pool })
    }
    
    /// 插入文件元数据
    pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
        let id = sqlx::query(
            r#"
            INSERT INTO files (
                sha256_hash, virtual_path, original_name, size,
                modified_time, mime_type, parent_archive_id, depth_level, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&metadata.sha256_hash)
        .bind(&metadata.virtual_path)
        .bind(&metadata.original_name)
        .bind(metadata.size)
        .bind(metadata.modified_time)
        .bind(&metadata.mime_type)
        .bind(metadata.parent_archive_id)
        .bind(metadata.depth_level)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        
        Ok(id)
    }
    
    /// 根据虚拟路径查询文件
    pub async fn get_file_by_virtual_path(&self, virtual_path: &str) -> Result<Option<FileMetadata>> {
        let row = sqlx::query(
            "SELECT * FROM files WHERE virtual_path = ?"
        )
        .bind(virtual_path)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|r| FileMetadata {
            id: r.get("id"),
            sha256_hash: r.get("sha256_hash"),
            virtual_path: r.get("virtual_path"),
            original_name: r.get("original_name"),
            size: r.get("size"),
            modified_time: r.get("modified_time"),
            mime_type: r.get("mime_type"),
            parent_archive_id: r.get("parent_archive_id"),
            depth_level: r.get("depth_level"),
        }))
    }
    
    /// 获取压缩包的所有子文件
    pub async fn get_archive_children(&self, archive_id: i64) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query(
            "SELECT * FROM files WHERE parent_archive_id = ? ORDER BY virtual_path"
        )
        .bind(archive_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|r| FileMetadata {
            id: r.get("id"),
            sha256_hash: r.get("sha256_hash"),
            virtual_path: r.get("virtual_path"),
            original_name: r.get("original_name"),
            size: r.get("size"),
            modified_time: r.get("modified_time"),
            mime_type: r.get("mime_type"),
            parent_archive_id: r.get("parent_archive_id"),
            depth_level: r.get("depth_level"),
        }).collect())
    }
}
```

### 3. Archive Processor (重新设计)

#### 处理流程

```rust
pub struct ArchiveProcessor {
    cas: Arc<ContentAddressableStorage>,
    metadata: Arc<MetadataStore>,
    max_depth: usize,
}

impl ArchiveProcessor {
    /// 处理压缩包（递归）
    pub async fn process_archive(
        &self,
        archive_path: &Path,
        virtual_path: &str,
        parent_archive_id: Option<i64>,
        current_depth: usize,
    ) -> Result<ProcessingReport> {
        // 检查深度限制
        if current_depth >= self.max_depth {
            warn!("Max nesting depth reached: {}", current_depth);
            return Ok(ProcessingReport::depth_limit_reached());
        }
        
        // 1. 读取压缩包内容
        let archive_content = fs::read(archive_path).await?;
        
        // 2. 计算哈希并存储
        let archive_hash = self.cas.store_content(&archive_content).await?;
        
        // 3. 记录压缩包元数据
        let archive_id = self.metadata.insert_archive(&ArchiveMetadata {
            sha256_hash: archive_hash.clone(),
            virtual_path: virtual_path.to_string(),
            original_name: archive_path.file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            archive_type: detect_archive_type(archive_path)?,
            parent_archive_id,
            depth_level: current_depth as i32,
            extraction_status: "extracting".to_string(),
        }).await?;
        
        // 4. 解压到临时目录
        let temp_dir = tempfile::tempdir()?;
        extract_archive(archive_path, temp_dir.path()).await?;
        
        // 5. 处理解压后的文件
        let mut report = ProcessingReport::new();
        
        for entry in WalkDir::new(temp_dir.path()) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            
            let file_path = entry.path();
            let relative_path = file_path.strip_prefix(temp_dir.path())?;
            let file_virtual_path = format!("{}/{}", virtual_path, relative_path.display());
            
            // 检查是否是嵌套压缩包
            if is_archive_file(file_path) {
                // 递归处理嵌套压缩包
                let nested_report = self.process_archive(
                    file_path,
                    &file_virtual_path,
                    Some(archive_id),
                    current_depth + 1,
                ).await?;
                report.merge(nested_report);
            } else {
                // 处理普通文件
                let file_content = fs::read(file_path).await?;
                let file_hash = self.cas.store_content(&file_content).await?;
                
                // 记录文件元数据
                self.metadata.insert_file(&FileMetadata {
                    id: 0,  // 由数据库生成
                    sha256_hash: file_hash,
                    virtual_path: file_virtual_path,
                    original_name: file_path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    size: file_content.len() as i64,
                    modified_time: get_modified_time(file_path)?,
                    mime_type: detect_mime_type(file_path),
                    parent_archive_id: Some(archive_id),
                    depth_level: (current_depth + 1) as i32,
                }).await?;
                
                report.files_processed += 1;
            }
        }
        
        // 6. 更新压缩包状态
        self.metadata.update_archive_status(archive_id, "completed").await?;
        
        Ok(report)
    }
}
```

### 4. Search Engine Integration

#### 搜索实现

```rust
pub struct SearchEngine {
    metadata: Arc<MetadataStore>,
    cas: Arc<ContentAddressableStorage>,
}

impl SearchEngine {
    /// 搜索文件内容
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // 1. 从元数据库查询匹配的文件
        let files = self.metadata.search_files(query).await?;
        
        let mut results = Vec::new();
        
        for file in files {
            // 2. 从 CAS 读取文件内容
            let content = self.cas.read_content(&file.sha256_hash).await?;
            let content_str = String::from_utf8_lossy(&content);
            
            // 3. 在内容中搜索
            if content_str.contains(query) {
                results.push(SearchResult {
                    virtual_path: file.virtual_path,
                    original_name: file.original_name,
                    sha256_hash: file.sha256_hash,
                    matches: extract_matches(&content_str, query),
                });
            }
        }
        
        Ok(results)
    }
}
```

### 5. Frontend Integration

#### Virtual File Tree Component

```typescript
// React 组件展示虚拟文件树
interface VirtualFileNode {
  id: string;
  name: string;
  virtualPath: string;
  type: 'file' | 'archive' | 'folder';
  children?: VirtualFileNode[];
  sha256Hash?: string;
}

export function VirtualFileTree() {
  const [tree, setTree] = useState<VirtualFileNode[]>([]);
  
  useEffect(() => {
    // 从后端加载虚拟文件树
    invoke<VirtualFileNode[]>('get_virtual_file_tree', {
      workspaceId: currentWorkspace
    }).then(setTree);
  }, [currentWorkspace]);
  
  const handleFileClick = async (node: VirtualFileNode) => {
    if (node.type === 'file') {
      // 通过哈希读取文件内容
      const content = await invoke<string>('read_file_by_hash', {
        hash: node.sha256Hash
      });
      // 显示内容
      showFileContent(content);
    }
  };
  
  return (
    <Tree>
      {tree.map(node => (
        <TreeNode
          key={node.id}
          node={node}
          onClick={handleFileClick}
        />
      ))}
    </Tree>
  );
}
```



## Path Length Mitigation Strategies

### Strategy 1: Content-Addressable Storage (Primary)

**优势**:
- ✅ 完全避免路径嵌套问题
- ✅ 自动去重（相同内容只存储一次）
- ✅ 支持无限嵌套深度
- ✅ 业内成熟方案（Git, Docker）

**实现**:
```
物理存储: workspace/objects/a3/f2e1d4c5...
虚拟路径: app_logs.zip/2024-01-01_logs.zip/server_logs.zip/application.log
```

### Strategy 2: Short Path Names (Fallback)

如果需要保持传统文件系统结构，使用短路径名：

```rust
pub struct ShortPathGenerator {
    counter: AtomicU64,
}

impl ShortPathGenerator {
    /// 生成短路径名
    pub fn generate(&self, original_name: &str) -> String {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let ext = Path::new(original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        if ext.is_empty() {
            format!("f{:08x}", id)
        } else {
            format!("f{:08x}.{}", id, ext)
        }
    }
}

// 使用示例
let short_name = generator.generate("very_long_file_name_that_exceeds_limits.log");
// 结果: f00000001.log
```

### Strategy 3: Windows Long Path Support

在 Windows 上启用长路径支持：

```rust
#[cfg(windows)]
pub fn enable_long_paths() -> Result<()> {
    use std::os::windows::fs::MetadataExt;
    
    // 使用 \\?\ 前缀启用长路径
    // 参考: https://docs.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation
    
    // 在 Cargo.toml 中添加 manifest
    // [package.metadata.winres]
    // LongPathAware = true
    
    Ok(())
}
```

## Performance Optimization

### 1. Parallel Processing

使用 `rayon` 并行处理文件：

```rust
use rayon::prelude::*;

impl ArchiveProcessor {
    pub async fn process_files_parallel(
        &self,
        files: Vec<PathBuf>,
    ) -> Result<Vec<ProcessingResult>> {
        // 使用 rayon 并行处理
        let results: Vec<_> = files
            .par_iter()
            .map(|file| {
                // 在 tokio runtime 中执行异步操作
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.process_single_file(file).await
                    })
                })
            })
            .collect();
        
        Ok(results)
    }
}
```

### 2. Streaming Extraction

避免一次性加载整个压缩包到内存：

```rust
use async_compression::tokio::bufread::GzipDecoder;
use tokio::io::{AsyncReadExt, BufReader};

pub async fn stream_extract_gz(
    input: &Path,
    output: &Path,
) -> Result<()> {
    let file = File::open(input).await?;
    let buf_reader = BufReader::new(file);
    let mut decoder = GzipDecoder::new(buf_reader);
    
    let mut output_file = File::create(output).await?;
    
    // 流式解压，避免内存峰值
    tokio::io::copy(&mut decoder, &mut output_file).await?;
    
    Ok(())
}
```

### 3. Incremental Hashing

对大文件使用增量哈希计算：

```rust
pub async fn compute_hash_incremental(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192]; // 8KB 缓冲区
    
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}
```

## Error Handling and Recovery

### 1. Transactional Processing

使用 SQLite 事务确保原子性：

```rust
impl MetadataStore {
    pub async fn process_archive_transactional(
        &self,
        archive_id: i64,
        files: Vec<FileMetadata>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        
        for file in files {
            sqlx::query(
                "INSERT INTO files (...) VALUES (...)"
            )
            .bind(&file.sha256_hash)
            // ... 其他字段
            .execute(&mut *tx)
            .await?;
        }
        
        // 更新压缩包状态
        sqlx::query(
            "UPDATE archives SET extraction_status = 'completed' WHERE id = ?"
        )
        .bind(archive_id)
        .execute(&mut *tx)
        .await?;
        
        // 提交事务
        tx.commit().await?;
        
        Ok(())
    }
}
```

### 2. Corruption Detection

使用哈希验证文件完整性：

```rust
pub async fn verify_file_integrity(
    cas: &ContentAddressableStorage,
    hash: &str,
) -> Result<bool> {
    let content = cas.read_content(hash).await?;
    let computed_hash = ContentAddressableStorage::compute_hash(&content);
    
    Ok(computed_hash == hash)
}
```

### 3. Partial Failure Recovery

记录处理进度，支持断点续传：

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingCheckpoint {
    pub archive_id: i64,
    pub processed_files: usize,
    pub total_files: usize,
    pub last_processed_path: String,
    pub timestamp: i64,
}

impl ArchiveProcessor {
    pub async fn save_checkpoint(&self, checkpoint: &ProcessingCheckpoint) -> Result<()> {
        let checkpoint_path = self.workspace_dir.join("checkpoint.json");
        let json = serde_json::to_string_pretty(checkpoint)?;
        fs::write(&checkpoint_path, json).await?;
        Ok(())
    }
    
    pub async fn resume_from_checkpoint(&self) -> Result<Option<ProcessingCheckpoint>> {
        let checkpoint_path = self.workspace_dir.join("checkpoint.json");
        if !checkpoint_path.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&checkpoint_path).await?;
        let checkpoint = serde_json::from_str(&json)?;
        Ok(Some(checkpoint))
    }
}
```

## Testing Strategy

### 1. Path Length Tests

```rust
#[tokio::test]
async fn test_deeply_nested_archives() {
    // 创建 10 层嵌套的压缩包
    let test_archive = create_nested_archive(10).await;
    
    let processor = ArchiveProcessor::new(test_workspace()).await.unwrap();
    let report = processor.process_archive(&test_archive, "test.zip", None, 0).await.unwrap();
    
    // 验证所有文件都被正确处理
    assert_eq!(report.files_processed, 10);
    
    // 验证所有路径都在限制内
    let metadata = processor.metadata.get_all_files().await.unwrap();
    for file in metadata {
        let object_path = processor.cas.get_object_path(&file.sha256_hash);
        assert!(object_path.to_string_lossy().len() < 260);
    }
}
```

### 2. Content Integrity Tests

```rust
#[tokio::test]
async fn test_content_addressable_storage() {
    let cas = ContentAddressableStorage::new(test_workspace()).await.unwrap();
    
    let content = b"test content";
    let hash = cas.store_content(content).await.unwrap();
    
    // 验证哈希正确
    let expected_hash = "9a0364b9e99bb480dd25e1f0284c8555f420dec...";
    assert_eq!(hash, expected_hash);
    
    // 验证可以读取
    let retrieved = cas.read_content(&hash).await.unwrap();
    assert_eq!(retrieved, content);
    
    // 验证去重
    let hash2 = cas.store_content(content).await.unwrap();
    assert_eq!(hash, hash2);
}
```

### 3. Nested Archive Tests

```rust
#[tokio::test]
async fn test_nested_archive_processing() {
    // 创建测试结构:
    // outer.zip
    //   ├── inner1.zip
    //   │   └── file1.txt
    //   └── inner2.zip
    //       └── file2.txt
    
    let test_archive = create_test_nested_archive().await;
    let processor = ArchiveProcessor::new(test_workspace()).await.unwrap();
    
    let report = processor.process_archive(
        &test_archive,
        "outer.zip",
        None,
        0
    ).await.unwrap();
    
    // 验证所有文件都被处理
    assert_eq!(report.files_processed, 2);
    assert_eq!(report.archives_processed, 3); // outer + inner1 + inner2
    
    // 验证虚拟路径正确
    let files = processor.metadata.get_all_files().await.unwrap();
    assert!(files.iter().any(|f| f.virtual_path == "outer.zip/inner1.zip/file1.txt"));
    assert!(files.iter().any(|f| f.virtual_path == "outer.zip/inner2.zip/file2.txt"));
}
```

## Migration Path

### Phase 1: 实现 CAS 层（不影响现有功能）

1. 实现 `ContentAddressableStorage`
2. 实现 `MetadataStore`
3. 添加单元测试

### Phase 2: 集成到导入流程

1. 修改 `ArchiveProcessor` 使用 CAS
2. 保持向后兼容（同时支持旧格式）
3. 添加集成测试

### Phase 3: 前端适配

1. 实现虚拟文件树组件
2. 更新搜索界面
3. 添加 E2E 测试

### Phase 4: 数据迁移

1. 提供迁移工具将旧格式转换为新格式
2. 支持增量迁移
3. 验证数据完整性

## Monitoring and Observability

### Metrics

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveProcessingMetrics {
    pub total_archives: usize,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub deduplication_ratio: f64,  // 去重率
    pub avg_processing_time_ms: u64,
    pub max_nesting_depth: usize,
    pub storage_efficiency: f64,  // 存储效率
}

impl ArchiveProcessor {
    pub async fn collect_metrics(&self) -> Result<ArchiveProcessingMetrics> {
        let archives = self.metadata.count_archives().await?;
        let files = self.metadata.count_files().await?;
        let total_size = self.metadata.sum_file_sizes().await?;
        let storage_size = self.cas.get_storage_size().await?;
        
        Ok(ArchiveProcessingMetrics {
            total_archives: archives,
            total_files: files,
            total_size_bytes: total_size,
            deduplication_ratio: 1.0 - (storage_size as f64 / total_size as f64),
            avg_processing_time_ms: self.get_avg_processing_time().await?,
            max_nesting_depth: self.metadata.get_max_depth().await?,
            storage_efficiency: storage_size as f64 / total_size as f64,
        })
    }
}
```

## Security Considerations

### 1. Hash Collision Resistance

SHA-256 提供足够的碰撞抵抗性：
- 碰撞概率: 2^-256
- 业内标准（Git, Docker, Bitcoin）

### 2. Path Traversal Prevention

```rust
pub fn validate_virtual_path(path: &str) -> Result<()> {
    // 禁止路径遍历
    if path.contains("..") {
        return Err(anyhow!("Path traversal detected"));
    }
    
    // 禁止绝对路径
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(anyhow!("Absolute path not allowed"));
    }
    
    Ok(())
}
```

### 3. Resource Limits

```rust
pub struct ResourceLimits {
    pub max_file_size: u64,        // 100MB
    pub max_total_size: u64,       // 10GB
    pub max_nesting_depth: usize,  // 10
    pub max_files_per_archive: usize, // 10000
}
```

## References

- [Git Internals - Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects)
- [Docker Image Specification](https://github.com/moby/moby/blob/master/image/spec/v1.2.md)
- [Content-Addressable Storage](https://en.wikipedia.org/wiki/Content-addressable_storage)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)
- [Windows Long Path Support](https://docs.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation)
- [7-Zip Technical Information](https://www.7-zip.org/7z.html)

## Comparison with Simple Approach

| 特性 | 简单方案（嵌套解压） | CAS 方案（本设计） |
|------|---------------------|-------------------|
| 路径长度 | ❌ 受限于操作系统 | ✅ 无限制 |
| 嵌套深度 | ❌ 受路径长度限制 | ✅ 仅受配置限制 |
| 存储效率 | ❌ 重复文件多次存储 | ✅ 自动去重 |
| 完整性验证 | ❌ 需要额外实现 | ✅ 内置哈希验证 |
| 跨平台兼容 | ❌ Windows 路径限制 | ✅ 完全兼容 |
| 业内实践 | ❌ 非标准方案 | ✅ Git/Docker 标准 |
| 实现复杂度 | ✅ 简单 | ⚠️ 中等 |
| 性能 | ⚠️ 中等 | ✅ 优秀（并行+去重） |

## Conclusion

本设计采用业内成熟的 Content-Addressable Storage 方案，完全解决了嵌套压缩包和路径长度限制的问题。该方案：

1. ✅ **成熟可靠**: 基于 Git、Docker 等成熟系统的设计
2. ✅ **可扩展**: 支持无限嵌套深度和任意路径长度
3. ✅ **高效**: 自动去重，节省存储空间
4. ✅ **安全**: 内置完整性验证和安全检查
5. ✅ **可维护**: 清晰的架构和完善的测试

相比简单的嵌套解压方案，CAS 方案虽然实现复杂度略高，但提供了更好的可靠性、性能和用户体验，是处理复杂压缩包场景的最佳选择。
