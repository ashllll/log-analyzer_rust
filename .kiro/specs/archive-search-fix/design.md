# Design Document - Archive Search Fix

## Overview

本设计文档描述了修复压缩包解压后无法搜索问题的技术方案。该方案基于业内成熟的文件索引和路径管理模式，确保从压缩包导入到搜索查询的完整数据流一致性。

### 核心问题

当前系统在处理压缩包时存在路径映射不一致的问题：
- 解压后的文件存储在临时目录中
- Path Map 可能记录了错误的路径（原始压缩包路径而非解压后路径）
- 搜索引擎无法通过 Path Map 访问实际文件

### 解决方案概述

采用以下成熟的技术方案：
1. **路径规范化**: 使用 Rust 标准库的 `Path::canonicalize()` 获取绝对路径
2. **文档索引模式**: 参考 Lucene/Tantivy 的文档 ID 管理方式
3. **结构化日志**: 使用 `tracing` crate 提供可观测性
4. **防御性编程**: 在关键操作前验证路径有效性

## Architecture

### 系统架构图

```
┌─────────────────┐
│  User Interface │
└────────┬────────┘
         │ import_folder(zip_path)
         ▼
┌─────────────────────────────────────────┐
│      Import Command Handler             │
│  (commands/import.rs)                   │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│   Archive Processor                     │
│  (archive/processor.rs)                 │
│  - Detect archive type                  │
│  - Extract to workspace directory       │
│  - Build path mappings                  │
└────────┬────────────────────────────────┘
         │
         ├──────────────────┬──────────────────┐
         ▼                  ▼                  ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ Path Manager │   │ File Indexer │   │ Metadata     │
│              │   │              │   │ Collector    │
│ - Normalize  │   │ - Build map  │   │              │
│ - Validate   │   │ - Verify     │   │ - Size       │
│ - Canonicalize│  │ - Persist    │   │ - Modified   │
└──────────────┘   └──────────────┘   └──────────────┘
         │                  │                  │
         └──────────────────┴──────────────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │   Path Map       │
                   │  (AppState)      │
                   │                  │
                   │ Real → Virtual   │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │ Search Engine    │
                   │ (commands/       │
                   │  search.rs)      │
                   │                  │
                   │ - Query files    │
                   │ - Open & read    │
                   │ - Return results │
                   └──────────────────┘
```

### 数据流

1. **导入阶段**:
   ```
   ZIP File → Extract → Temp Dir → Canonicalize Paths → Path Map → Index File
   ```

2. **搜索阶段**:
   ```
   Query → Path Map Lookup → Validate Path → Open File → Search Content → Results
   ```

## Components and Interfaces

### 1. Path Manager (新增组件)

**职责**: 统一管理文件路径的规范化和验证

**接口**:
```rust
pub struct PathManager;

impl PathManager {
    /// 规范化路径为绝对路径
    pub fn canonicalize(path: &Path) -> Result<PathBuf, PathError>;
    
    /// 验证路径是否存在且可访问
    pub fn validate(path: &Path) -> Result<(), PathError>;
    
    /// 规范化并验证路径
    pub fn normalize_and_validate(path: &Path) -> Result<PathBuf, PathError>;
    
    /// 计算相对路径
    pub fn relative_to(path: &Path, base: &Path) -> Result<PathBuf, PathError>;
}
```

**实现位置**: `src-tauri/src/utils/path_manager.rs`

### 2. Archive Processor (修改现有组件)

**修改点**:
- 在添加文件到 Path Map 前使用 `PathManager::normalize_and_validate()`
- 添加结构化日志记录每个处理步骤
- 实现批量验证机制

**关键修改**:
```rust
// 在 process_path_recursive_inner 中
async fn process_path_recursive_inner(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
    workspace_id: &str,
) -> Result<()> {
    // ... 现有逻辑 ...
    
    // 修改：使用 PathManager 规范化路径
    let real_path = PathManager::normalize_and_validate(path)
        .map_err(|e| AppError::validation_error(
            format!("Failed to normalize path {}: {}", path.display(), e)
        ))?
        .to_string_lossy()
        .to_string();
    
    let normalized_virtual = normalize_path_separator(virtual_path);
    
    // 添加结构化日志
    debug!(
        real_path = %real_path,
        virtual_path = %normalized_virtual,
        "Adding file to path_map"
    );
    
    map.insert(real_path, normalized_virtual);
    
    Ok(())
}
```

### 3. Search Engine (修改现有组件)

**修改点**:
- 在打开文件前验证路径有效性
- 添加详细的错误日志
- 跳过无效文件而不中断搜索

**关键修改**:
```rust
fn search_single_file_with_details(
    real_path: &str,
    virtual_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();
    
    // 添加：验证路径
    let path = Path::new(real_path);
    if !path.exists() {
        warn!(
            file = %real_path,
            "Skipping non-existent file"
        );
        return results;
    }
    
    match File::open(real_path) {
        Ok(file) => {
            // ... 现有搜索逻辑 ...
        }
        Err(e) => {
            error!(
                file = %real_path,
                error = %e,
                "Failed to open file for search"
            );
        }
    }
    
    results
}
```

### 4. Index Validator (新增组件)

**职责**: 验证索引完整性和一致性

**接口**:
```rust
pub struct IndexValidator;

impl IndexValidator {
    /// 验证 path_map 中的所有路径
    pub fn validate_path_map(
        path_map: &HashMap<String, String>
    ) -> ValidationReport;
    
    /// 修复无效路径
    pub fn repair_path_map(
        path_map: &mut HashMap<String, String>
    ) -> RepairReport;
}

pub struct ValidationReport {
    pub total_paths: usize,
    pub valid_paths: usize,
    pub invalid_paths: Vec<String>,
    pub warnings: Vec<String>,
}
```

**实现位置**: `src-tauri/src/services/index_validator.rs`

## Data Models

### PathError (新增)

```rust
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Path does not exist: {0}")]
    NotFound(String),
    
    #[error("Path is not accessible: {0}")]
    AccessDenied(String),
    
    #[error("Failed to canonicalize path: {0}")]
    CanonicalizationFailed(String),
    
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),
}
```

### ValidationReport (新增)

```rust
#[derive(Debug, Serialize)]
pub struct ValidationReport {
    pub total_paths: usize,
    pub valid_paths: usize,
    pub invalid_paths: Vec<InvalidPathInfo>,
    pub warnings: Vec<String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Serialize)]
pub struct InvalidPathInfo {
    pub path: String,
    pub reason: String,
    pub virtual_path: Option<String>,
}
```

### ImportStatistics (增强现有)

```rust
#[derive(Debug, Serialize)]
pub struct ImportStatistics {
    pub total_files: usize,
    pub indexed_files: usize,
    pub failed_files: usize,
    pub skipped_files: usize,
    pub total_size: u64,
    pub duration: Duration,
    
    // 新增字段
    pub validation_report: ValidationReport,
    pub path_normalization_failures: usize,
}
```



## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Path Normalization Idempotence

*For any* file path, normalizing it multiple times should produce the same result

**Validates: Requirements 2.2, 7.2**

**Rationale**: 路径规范化必须是幂等的，确保无论何时调用都得到一致的结果。这是 Lucene 等成熟索引系统的核心原则。

### Property 2: Path Map Completeness

*For any* extracted file from an archive, if extraction succeeds, then that file's path must exist in the Path Map

**Validates: Requirements 1.2, 1.3**

**Rationale**: 确保所有成功解压的文件都被正确索引，这是搜索功能的前提条件。

### Property 3: Path Existence Consistency

*For any* path in the Path Map, that path must point to an existing file on the filesystem

**Validates: Requirements 1.5, 2.4**

**Rationale**: Path Map 中的所有路径必须有效，这是防御性编程的核心原则。

### Property 4: Search File Access

*For any* file returned by search, opening that file must succeed

**Validates: Requirements 1.4, 8.3**

**Rationale**: 搜索结果必须可访问，否则用户体验会严重受损。

### Property 5: Nested Archive Flattening

*For any* nested archive structure, all leaf files must be accessible through the Path Map regardless of nesting depth

**Validates: Requirements 4.1, 4.4**

**Rationale**: 嵌套压缩包的所有文件都应该被正确索引和搜索。

### Property 6: Path Canonicalization Consistency

*For any* two paths pointing to the same file, their canonicalized forms must be identical

**Validates: Requirements 7.3**

**Rationale**: 避免同一文件被多次索引，确保路径唯一性。

### Property 7: Error Recovery Isolation

*For any* single file processing failure, the remaining files must still be processed successfully

**Validates: Requirements 8.1, 8.4**

**Rationale**: 单个文件失败不应影响整体导入，这是鲁棒性的关键。

### Property 8: Cleanup Completeness

*For any* deleted workspace, all associated extracted files must be removed from the filesystem

**Validates: Requirements 5.1, 5.2**

**Rationale**: 防止磁盘空间泄漏，确保资源正确释放。

## Error Handling

### Error Categories

#### 1. Path Errors (PathError)

**处理策略**: 记录警告，跳过该文件，继续处理

**示例**:
- 文件不存在
- 路径规范化失败
- 权限不足

**实现**:
```rust
match PathManager::normalize_and_validate(path) {
    Ok(normalized) => {
        // 继续处理
    }
    Err(PathError::NotFound(p)) => {
        warn!(path = %p, "File not found, skipping");
        continue;
    }
    Err(PathError::AccessDenied(p)) => {
        warn!(path = %p, "Access denied, skipping");
        continue;
    }
    Err(e) => {
        error!(error = %e, "Path validation failed");
        continue;
    }
}
```

#### 2. Archive Errors (AppError::ArchiveError)

**处理策略**: 记录错误，尝试继续处理其他文件

**示例**:
- 压缩包损坏
- 不支持的格式
- 解压失败

**实现**:
```rust
match extract_archive(archive_path) {
    Ok(files) => {
        // 处理解压的文件
    }
    Err(e) => {
        error!(
            archive = %archive_path.display(),
            error = %e,
            "Failed to extract archive"
        );
        // 继续处理其他文件
    }
}
```

#### 3. I/O Errors

**处理策略**: 根据错误类型决定是否重试

**示例**:
- 磁盘空间不足 → 停止导入
- 临时网络问题 → 重试
- 文件被占用 → 跳过

**实现**:
```rust
match fs::create_dir_all(&extract_dir).await {
    Ok(_) => { /* 继续 */ }
    Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
        return Err(AppError::critical("Permission denied"));
    }
    Err(e) => {
        warn!(error = %e, "Failed to create directory, retrying");
        // 重试逻辑
    }
}
```

### Error Recovery Mechanisms

#### 1. Graceful Degradation

单个文件失败不影响整体导入：

```rust
let mut success_count = 0;
let mut failure_count = 0;

for file in files {
    match process_file(file) {
        Ok(_) => success_count += 1,
        Err(e) => {
            warn!(file = %file, error = %e, "Failed to process file");
            failure_count += 1;
        }
    }
}

info!(
    success = success_count,
    failures = failure_count,
    "Import completed with partial success"
);
```

#### 2. Validation and Repair

导入完成后验证并修复索引：

```rust
let validation_report = IndexValidator::validate_path_map(&path_map);

if !validation_report.invalid_paths.is_empty() {
    warn!(
        invalid_count = validation_report.invalid_paths.len(),
        "Found invalid paths in index"
    );
    
    let repair_report = IndexValidator::repair_path_map(&mut path_map);
    
    info!(
        repaired = repair_report.repaired_count,
        removed = repair_report.removed_count,
        "Index repair completed"
    );
}
```

## Testing Strategy

### Unit Tests

测试核心功能的正确性：

1. **PathManager Tests**
   - 路径规范化
   - 路径验证
   - 相对路径计算

2. **IndexValidator Tests**
   - 索引验证逻辑
   - 修复机制
   - 报告生成

**示例**:
```rust
#[test]
fn test_path_normalization_idempotence() {
    let path = Path::new("./test/file.txt");
    let normalized1 = PathManager::canonicalize(path).unwrap();
    let normalized2 = PathManager::canonicalize(&normalized1).unwrap();
    assert_eq!(normalized1, normalized2);
}
```

### Integration Tests

测试完整的导入-搜索流程：

1. **Archive Import Integration**
   - 导入 ZIP 文件
   - 验证 Path Map
   - 执行搜索
   - 验证结果

2. **Nested Archive Integration**
   - 导入嵌套压缩包
   - 验证所有文件可访问
   - 执行搜索

**示例**:
```rust
#[tokio::test]
async fn test_archive_import_and_search() {
    // 1. 导入压缩包
    let workspace_id = import_test_archive("test.zip").await.unwrap();
    
    // 2. 验证 Path Map
    let path_map = get_path_map(&workspace_id);
    assert!(!path_map.is_empty());
    
    // 3. 验证所有路径存在
    for (real_path, _) in path_map.iter() {
        assert!(Path::new(real_path).exists());
    }
    
    // 4. 执行搜索
    let results = search_logs("test query", &workspace_id).await.unwrap();
    assert!(!results.is_empty());
}
```

### Property-Based Tests

使用 `proptest` 验证通用属性：

**Property 1: Path Normalization Idempotence**
```rust
proptest! {
    #[test]
    fn prop_path_normalization_idempotent(path_str in ".*") {
        if let Ok(path) = PathBuf::from(&path_str).canonicalize() {
            let normalized1 = PathManager::canonicalize(&path).unwrap();
            let normalized2 = PathManager::canonicalize(&normalized1).unwrap();
            prop_assert_eq!(normalized1, normalized2);
        }
    }
}
```

**Property 2: Path Map Completeness**
```rust
proptest! {
    #[test]
    fn prop_extracted_files_in_path_map(
        archive_files in prop::collection::vec(any::<String>(), 1..100)
    ) {
        let extracted = extract_test_archive(archive_files);
        let path_map = build_path_map(&extracted);
        
        for file in extracted {
            prop_assert!(path_map.contains_key(&file.to_string_lossy().to_string()));
        }
    }
}
```

### End-to-End Tests

模拟真实用户场景：

1. **Complete User Workflow**
   - 用户导入压缩包
   - 等待导入完成
   - 执行搜索
   - 查看结果
   - 删除工作区

2. **Error Scenarios**
   - 导入损坏的压缩包
   - 导入超大压缩包
   - 磁盘空间不足
   - 权限问题

## Performance Considerations

### 1. 批量路径验证

避免逐个验证路径，使用批量操作：

```rust
// 不好的做法
for path in paths {
    if path.exists() {
        // 处理
    }
}

// 好的做法
let valid_paths: Vec<_> = paths
    .par_iter()  // 使用 rayon 并行处理
    .filter(|p| p.exists())
    .collect();
```

### 2. 延迟路径规范化

只在必要时规范化路径：

```rust
// 在插入 Path Map 时规范化
// 在搜索时使用已规范化的路径，无需再次规范化
```

### 3. 索引缓存

缓存验证结果避免重复检查：

```rust
struct PathCache {
    validated: DashMap<PathBuf, bool>,
}

impl PathCache {
    fn is_valid(&self, path: &Path) -> Option<bool> {
        self.validated.get(path).map(|v| *v)
    }
    
    fn mark_valid(&self, path: PathBuf, valid: bool) {
        self.validated.insert(path, valid);
    }
}
```

## Migration Strategy

### Phase 1: 添加新组件（不破坏现有功能）

1. 实现 `PathManager`
2. 实现 `IndexValidator`
3. 添加单元测试

### Phase 2: 集成新组件

1. 修改 `archive/processor.rs` 使用 `PathManager`
2. 修改 `commands/search.rs` 添加路径验证
3. 添加集成测试

### Phase 3: 增强日志和监控

1. 添加结构化日志
2. 实现索引验证报告
3. 添加性能监控

### Phase 4: 清理和优化

1. 移除临时调试代码
2. 优化性能
3. 更新文档

## Dependencies

### 新增依赖

```toml
[dependencies]
# 结构化日志
tracing = "0.1"
tracing-subscriber = "0.3"

# 属性测试
[dev-dependencies]
proptest = "1.0"
```

### 现有依赖

- `std::path`: 路径处理
- `tokio`: 异步运行时
- `walkdir`: 目录遍历
- `parking_lot`: 高性能锁

## Security Considerations

### 1. 路径遍历防护

确保解压的文件不会逃逸到工作区目录外：

```rust
fn validate_path_safety(path: &Path, base_dir: &Path) -> Result<()> {
    let canonical_path = path.canonicalize()?;
    let canonical_base = base_dir.canonicalize()?;
    
    if !canonical_path.starts_with(&canonical_base) {
        return Err(AppError::security_error("Path traversal detected"));
    }
    
    Ok(())
}
```

### 2. 符号链接处理

避免符号链接攻击：

```rust
if path.is_symlink() {
    warn!(path = %path.display(), "Skipping symbolic link");
    return Ok(());
}
```

### 3. 文件大小限制

防止解压炸弹：

```rust
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
const MAX_TOTAL_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10GB

if file_size > MAX_FILE_SIZE {
    warn!(size = file_size, "File too large, skipping");
    return Ok(());
}
```

## Monitoring and Observability

### Metrics

使用 `tracing` 记录关键指标：

```rust
#[instrument(skip(path_map))]
fn validate_index(path_map: &HashMap<String, String>) -> ValidationReport {
    let start = Instant::now();
    
    // 验证逻辑
    
    let duration = start.elapsed();
    info!(
        total_paths = path_map.len(),
        duration_ms = duration.as_millis(),
        "Index validation completed"
    );
    
    // 返回报告
}
```

### Logging Levels

- **ERROR**: 关键错误（导入失败、索引损坏）
- **WARN**: 可恢复错误（单个文件失败、路径无效）
- **INFO**: 重要事件（导入开始/完成、验证结果）
- **DEBUG**: 详细信息（每个文件处理、路径映射）
- **TRACE**: 极详细信息（内部状态变化）

## References

- [Rust std::path Documentation](https://doc.rust-lang.org/std/path/)
- [Lucene Index Design](https://lucene.apache.org/core/9_0_0/core/org/apache/lucene/codecs/lucene90/package-summary.html)
- [Tantivy Documentation](https://docs.rs/tantivy/)
- [Tracing Documentation](https://docs.rs/tracing/)
- [Property-Based Testing with Proptest](https://proptest-rs.github.io/proptest/)


## Architecture Comparison: Archive vs Folder Import

### Key Differences

#### 1. 文件存储位置

**文件夹导入**:
```
用户选择: D:\logs\app_logs\
文件位置: D:\logs\app_logs\file1.log (原地不动)
Path Map: D:\logs\app_logs\file1.log → app_logs/file1.log
```

**压缩包导入**:
```
用户选择: D:\logs\app_logs.zip
解压位置: C:\Users\...\AppData\extracted\{workspace_id}\app_logs_zip_{timestamp}\
文件位置: C:\Users\...\AppData\extracted\{workspace_id}\app_logs_zip_{timestamp}\file1.log
Path Map: C:\Users\...\AppData\extracted\{workspace_id}\...\file1.log → app_logs.zip/file1.log
```

**关键区别**: 
- 文件夹：文件保持在原位置，Path Map 直接指向用户目录
- 压缩包：文件被解压到应用数据目录，Path Map 指向临时解压目录

#### 2. 生命周期管理

**文件夹导入**:
```rust
// 导入时
- 扫描用户目录
- 构建 Path Map（指向用户目录）
- 保存索引

// 删除工作区时
- 删除索引文件
- 清除 Path Map
- 用户文件保持不变 ✓
```

**压缩包导入**:
```rust
// 导入时
- 解压到临时目录
- 扫描解压目录
- 构建 Path Map（指向临时目录）
- 保存索引

// 删除工作区时
- 删除索引文件
- 清除 Path Map
- 删除解压目录 ✓ (重要！)
```

**关键区别**:
- 文件夹：系统不拥有文件，不负责清理
- 压缩包：系统拥有解压文件，必须负责清理

#### 3. 路径稳定性

**文件夹导入**:
```
风险: 用户可能移动/删除/重命名文件
处理: 
- 文件监听器检测变化
- 增量刷新索引
- 路径可能失效
```

**压缩包导入**:
```
优势: 解压目录由系统完全控制
保证:
- 文件不会被外部修改
- 路径始终有效（直到工作区删除）
- 无需文件监听器
```

**关键区别**:
- 文件夹：路径不稳定，需要监听和刷新
- 压缩包：路径稳定，系统完全控制

#### 4. 数据流对比

**文件夹导入流程**:
```
┌─────────────┐
│ User Folder │ (用户目录)
└──────┬──────┘
       │ 直接扫描
       ▼
┌─────────────┐
│  Path Map   │ → 指向用户目录
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Search    │ → 直接访问用户文件
└─────────────┘
```

**压缩包导入流程**:
```
┌─────────────┐
│ Archive File│ (压缩包)
└──────┬──────┘
       │ 解压
       ▼
┌─────────────┐
│ Temp Dir    │ (临时目录)
└──────┬──────┘
       │ 扫描
       ▼
┌─────────────┐
│  Path Map   │ → 指向临时目录 ⚠️
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Search    │ → 访问临时目录文件
└─────────────┘
```

**关键区别**:
- 文件夹：一步到位（扫描 → 索引 → 搜索）
- 压缩包：多一步解压（解压 → 扫描 → 索引 → 搜索）

#### 5. 错误场景

**文件夹导入可能的问题**:
- 用户删除了文件 → 搜索时文件不存在
- 用户移动了文件夹 → 所有路径失效
- 权限问题 → 无法访问某些文件

**压缩包导入可能的问题**:
- 解压失败 → 部分文件缺失
- 磁盘空间不足 → 解压中断
- **Path Map 指向错误** → 这是当前的 bug！
- 临时目录被意外删除 → 所有文件丢失

### 当前 Bug 的根本原因

**问题**: Path Map 可能记录了错误的路径

**场景 1**: 记录了压缩包路径而非解压路径
```rust
// 错误的实现
let real_path = archive_path.to_string_lossy().to_string();  // ❌ 指向 .zip 文件
map.insert(real_path, virtual_path);

// 正确的实现
let real_path = extracted_file_path.to_string_lossy().to_string();  // ✓ 指向解压后的文件
map.insert(real_path, virtual_path);
```

**场景 2**: 路径未规范化导致不匹配
```rust
// 解压时记录的路径
"C:\\Users\\..\\AppData\\extracted\\file.log"

// 搜索时使用的路径
"C:/Users/.../AppData/extracted/file.log"

// 结果: 路径不匹配，无法找到文件
```

### 修复策略的差异

**文件夹导入的修复**（如果需要）:
- 主要关注路径监听和刷新
- 处理用户文件变更
- 优雅降级（文件不存在时）

**压缩包导入的修复**（当前任务）:
- ✅ 确保 Path Map 指向解压后的文件
- ✅ 使用规范化的绝对路径
- ✅ 验证路径在索引构建和搜索时都有效
- ✅ 确保解压目录生命周期管理正确

### 统一的路径管理策略

尽管有差异，我们使用统一的 `PathManager` 处理两种场景：

```rust
impl PathManager {
    /// 规范化路径 - 适用于文件夹和压缩包
    pub fn canonicalize(path: &Path) -> Result<PathBuf> {
        path.canonicalize()
            .map_err(|e| PathError::CanonicalizationFailed(e.to_string()))
    }
    
    /// 验证路径 - 适用于文件夹和压缩包
    pub fn validate(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(PathError::NotFound(path.display().to_string()));
        }
        Ok(())
    }
}
```

**使用场景**:

```rust
// 文件夹导入
let user_file = Path::new("D:\\logs\\file.log");
let normalized = PathManager::canonicalize(user_file)?;
// 结果: D:\logs\file.log (规范化的绝对路径)

// 压缩包导入
let extracted_file = Path::new("C:\\Users\\...\\AppData\\extracted\\...\\file.log");
let normalized = PathManager::canonicalize(extracted_file)?;
// 结果: C:\Users\...\AppData\extracted\...\file.log (规范化的绝对路径)
```

### 总结

| 特性 | 文件夹导入 | 压缩包导入 |
|------|-----------|-----------|
| 文件位置 | 用户目录 | 应用临时目录 |
| 路径稳定性 | 不稳定（用户可修改） | 稳定（系统控制） |
| 生命周期 | 独立于应用 | 由应用管理 |
| 清理责任 | 无 | 必须清理 |
| 监听需求 | 需要 | 不需要 |
| 当前 Bug | 无 | Path Map 路径错误 |
| 修复重点 | 路径监听 | 路径规范化和验证 |

**核心洞察**: 压缩包导入的复杂性在于它引入了一个中间层（解压目录），这个中间层的路径管理是当前 bug 的根源。我们的修复方案通过 `PathManager` 统一处理路径规范化，确保无论是文件夹还是压缩包，Path Map 中的路径都是有效且一致的。
