# 压缩包解压后无法搜索问题分析

## 问题描述

导入压缩包（如 `android_logs.zip`）后，解压成功但搜索无法找到结果。

## 根本原因

通过分析代码，发现了**路径映射不一致**的问题：

### 1. 解压过程中的路径映射

在 `archive/processor.rs` 的 `extract_and_process_archive` 函数中：

```rust
// 第 620-630 行
for extracted_file in &extracted_files {
    let relative_path = extracted_file.strip_prefix(&extract_dir)?;
    
    let new_virtual = format!(
        "{}/{}/{}",
        virtual_path,
        file_name,
        relative_path.to_string_lossy()
    );
    
    // 递归处理
    Box::pin(process_path_recursive(
        extracted_file,  // 真实路径：解压后的文件路径
        &new_virtual,    // 虚拟路径
        target_root,
        map,
        ...
    )).await;
}
```

**问题点**：
- `extracted_file` 是解压后的**临时目录**中的文件路径
- 例如：`C:\Users\...\AppData\extracted\1766340146117\android_logs_zip_xxx\logcat.txt`

### 2. 搜索过程中的路径使用

在 `commands/search.rs` 的 `search_logs` 函数中：

```rust
// 第 185 行
let files: Vec<(String, String)> = {
    let guard = path_map.lock();
    guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
};

// 第 450 行
fn search_single_file_with_details(
    real_path: &str,  // 从 path_map 获取的真实路径
    virtual_path: &str,
    ...
) -> Vec<LogEntry> {
    if let Ok(file) = File::open(real_path) {  // 尝试打开文件
        // 搜索逻辑
    }
}
```

**问题点**：
- 搜索时使用 `path_map` 中的 `real_path` 来打开文件
- 如果 `real_path` 指向的是**原始压缩包路径**而不是**解压后的文件路径**，则无法打开文件

## 可能的原因

### 场景 1：path_map 未正确更新

在 `process_path_recursive_inner` 函数中（第 320 行）：

```rust
// 普通文件处理
let real_path = path.to_string_lossy().to_string();
let normalized_virtual = normalize_path_separator(virtual_path);

map.insert(real_path, normalized_virtual.clone());
```

这里 `path` 应该是解压后的文件路径，但需要验证：
- 解压后的文件是否真的存在于磁盘上
- `path` 是否正确指向解压后的文件

### 场景 2：解压目录被提前清理

查看 `workspace.rs` 的 `delete_workspace` 函数，发现有清理逻辑：

```rust
// 删除解压目录
let extracted_dir = app_data_dir.join("extracted").join(workspace_id);
try_cleanup_temp_dir(&extracted_dir, &state.cleanup_queue);
```

但这应该只在删除工作区时执行，不应该影响正常导入。

### 场景 3：文件未实际解压到磁盘

需要检查 `extract_archive_async` 或 `ArchiveManager.extract_archive` 是否真的将文件写入磁盘。

## 调试步骤

### 1. 验证解压文件是否存在

在导入完成后，检查解压目录：

```
C:\Users\[用户名]\AppData\Roaming\[应用名]\extracted\[workspace_id]\
```

查看是否有解压后的文件。

### 2. 检查 path_map 内容

在搜索前添加日志，打印 `path_map` 的内容：

```rust
// 在 search_logs 函数中添加
eprintln!("[DEBUG] path_map entries:");
for (real, virtual) in files.iter().take(5) {
    eprintln!("  {} -> {}", real, virtual);
    eprintln!("  File exists: {}", std::path::Path::new(real).exists());
}
```

### 3. 检查文件打开失败

在 `search_single_file_with_details` 中添加错误日志：

```rust
if let Ok(file) = File::open(real_path) {
    // 搜索逻辑
} else {
    eprintln!("[ERROR] Failed to open file: {}", real_path);
}
```

## 修复方案（基于业内成熟实践）

### 采用的成熟解决方案

本修复将采用以下业内成熟的技术和模式：

#### 1. Lucene/Tantivy 文档索引模式

- **模式**: 使用规范化的绝对路径作为文档 ID
- **实现**: 在索引构建时使用 `Path::canonicalize()` 获取绝对路径
- **优势**: 避免路径歧义，确保索引和搜索使用相同的路径标识符

#### 2. Rust 标准库路径处理

- **工具**: `std::path::Path` 和 `PathBuf`
- **方法**: 
  - `canonicalize()`: 获取规范化的绝对路径
  - `exists()`: 验证路径有效性
  - `strip_prefix()`: 计算相对路径
- **优势**: 跨平台兼容，处理符号链接和路径规范化

#### 3. 流式文件处理

- **模式**: 使用迭代器和批处理避免内存峰值
- **实现**: 分批处理解压文件，避免一次性加载所有内容
- **优势**: 支持大型压缩包，内存使用可控

#### 4. 错误恢复机制

- **模式**: Fail-fast for critical errors, continue for non-critical errors
- **实现**: 单个文件失败不中断整体流程，记录错误并继续
- **优势**: 提高系统鲁棒性，部分失败不影响整体功能

### 具体实现方案

#### 方案 1：路径规范化（核心修复）

使用 Rust 标准库的 `canonicalize` 方法确保路径一致性：

```rust
// 在 process_path_recursive_inner 中
let real_path = path.canonicalize()
    .map_err(|e| AppError::validation_error(
        format!("Failed to canonicalize path {}: {}", path.display(), e)
    ))?
    .to_string_lossy()
    .to_string();
```

#### 方案 2：路径验证（防御性编程）

在关键操作前验证路径有效性：

```rust
// 在插入 path_map 前
if !path.exists() {
    return Err(AppError::validation_error(
        format!("File does not exist: {}", path.display())
    ));
}

// 在搜索时
if !Path::new(real_path).exists() {
    warn!("Skipping non-existent file: {}", real_path);
    continue;
}
```

#### 方案 3：结构化日志（可观测性）

使用 `tracing` crate 提供结构化日志：

```rust
use tracing::{info, warn, error, debug};

// 解压完成
info!(
    files_extracted = extracted_files.len(),
    archive = %file_name,
    "Archive extraction completed"
);

// 文件添加到索引
debug!(
    real_path = %real_path,
    virtual_path = %normalized_virtual,
    "Added file to path_map"
);

// 搜索失败
warn!(
    file = %real_path,
    error = %e,
    "Failed to open file for search"
);
```

#### 方案 4：索引一致性检查

在导入完成后验证索引完整性：

```rust
// 验证 path_map 中的所有文件都存在
let invalid_paths: Vec<_> = path_map
    .keys()
    .filter(|path| !Path::new(path).exists())
    .collect();

if !invalid_paths.is_empty() {
    warn!(
        count = invalid_paths.len(),
        "Found invalid paths in path_map"
    );
}
```

## 下一步行动

1. **创建规范文档**：✅ 已完成 - `.kiro/specs/archive-search-fix/requirements.md`
2. **设计技术方案**：基于成熟的 Lucene 索引模式和 Rust 标准库
3. **实现核心修复**：
   - 使用 `Path::canonicalize()` 规范化路径
   - 添加路径验证逻辑
   - 实现结构化日志
   - 添加索引一致性检查
4. **编写测试**：
   - 单元测试：路径规范化和验证
   - 集成测试：完整的导入-搜索流程
   - 属性测试：路径映射一致性
5. **验证修复**：使用真实压缩包测试完整流程

## 相关文件

- `log-analyzer/src-tauri/src/archive/processor.rs` - 压缩包处理
- `log-analyzer/src-tauri/src/commands/search.rs` - 搜索实现
- `log-analyzer/src-tauri/src/commands/import.rs` - 导入流程
- `log-analyzer/src-tauri/src/commands/workspace.rs` - 工作区管理
