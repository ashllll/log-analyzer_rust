[根目录../../../CLAUDE.md) > [src-tauri](../) > **archive (压缩包处理)**

# 压缩包处理模块文档

> 支持 ZIP/TAR/GZ/RAR 多格式递归解压 | v0.0.47+ Builder 模式重构

## 模块职责

Archive 模块负责统一处理各种压缩格式的日志文件，采用策略模式设计，支持：

- **多格式支持**: ZIP、TAR、TAR.GZ、TGZ、GZ、RAR
- **递归解压**: 自动处理嵌套压缩包
- **安全限制**: 文件大小/数量/总大小限制
- **统一接口**: ArchiveHandler Trait 标准化
- **错误处理**: 详细错误信息和路径追踪
- **跨平台**: 内置多平台 unrar 二进制
- **Builder 模式**: v0.0.47+ 采用 Builder 模式重构参数管理

## v0.0.47 重大更新：Builder 模式重构

### 解决的问题

- **参数超限**: `process_path_recursive` 8参数 → 0参数
- **类型安全**: 编译时参数验证
- **可读性**: 链式配置提升代码可读性
- **向后兼容**: 保留旧函数并标记为 deprecated

### 新旧 API 对比

**旧方式 (v0.0.46-)**:
```rust
process_path_recursive_with_metadata(
    path,
    virtual_path,
    target_root,
    &mut map,
    &mut metadata,
    &app,
    task_id,
    workspace_id,
    &state,
)
.await;
```

**新方式 (v0.0.47+)**:
```rust
ProcessBuilderWithMetadata::new(
    path.to_path_buf(),
    virtual_path.to_string(),
    &mut map,
    &mut metadata,
    &app,
    &state,
)
.target_root(target_root.to_path_buf())
.task_id(task_id.to_string())
.workspace_id(workspace_id.to_string())
.execute()
.await;
```

### Builder 结构体

#### ProcessBuilder
```rust
pub struct ProcessBuilder<'a> {
    path: PathBuf,
    virtual_path: String,
    target_root: PathBuf,
    map: &'a mut HashMap<String, String>,
    app: &'a AppHandle,
    task_id: String,
    workspace_id: String,
    state: &'a AppState,
}
```

#### ProcessBuilderWithMetadata
```rust
pub struct ProcessBuilderWithMetadata<'a> {
    path: PathBuf,
    virtual_path: String,
    target_root: PathBuf,
    map: &'a mut HashMap<String, String>,
    file_metadata: &'a mut HashMap<String, FileMetadata>,
    app: &'a AppHandle,
    task_id: String,
    workspace_id: String,
    state: &'a AppState,
}
```

## 架构设计

### 核心Trait - ArchiveHandler

```rust
#[async_trait::async_trait]
pub trait ArchiveHandler {
    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary>;

    fn can_handle(&self, path: &Path) -> bool;

    fn get_supported_extensions(&self) -> &'static [&'static str];
}
```

### 策略模式实现

```mermaid
graph TD
    A[ArchiveManager] --> B[find_handler]
    B --> C[选择处理器]
    C --> D[ZipHandler]
    C --> E[TarHandler]
    C --> F[GzHandler]
    C --> G[RarHandler]

    D --> H[extract_with_limits]
    E --> H
    F --> H
    G --> H
```

## 支持的格式

### 1. ZIP 格式 - ZipHandler

**技术实现**: `zip` crate (0.6)

**特性**:
- 标准 ZIP 格式支持
- 密码保护文件检测
- 目录结构保持
- 多平台兼容性

**支持扩展名**: `.zip`, `.ZIP`

**示例**:
```rust
let handler = ZipHandler;
let summary = handler.extract_with_limits(
    Path::new("logs.zip"),
    Path::new("./extracted"),
    100 * 1024 * 1024,  // 100MB
    1024 * 1024 * 1024, // 1GB
    1000                 // 1000文件
).await?;
```

### 2. TAR 格式 - TarHandler

**技术实现**: `tar` crate (0.4) + `flate2` (1.0)

**支持格式**:
- 纯 TAR: `.tar`
- 压缩 TAR.GZ: `.tar.gz`, `.tgz`
- 压缩 TAR.GZ (大写): `.TAR.GZ`, `.TGZ`

**特性**:
- 自动检测压缩格式
- 保持目录结构
- 软链接处理
- 权限信息保留

**处理逻辑**:
1. 检查扩展名确定压缩类型
2. 使用对应解压策略
3. 过滤非日志文件（可选）

### 3. GZ 格式 - GzHandler

**技术实现**: `flate2` crate (1.0)

**支持格式**: `.gz`, `.GZ`

**特性**:
- 纯 gzip 文件处理
- 单文件解压
- 自动检测文件类型
- 内存高效解压

**注意**:
- 纯 GZ 格式仅解压单文件
- TAR.GZ 由 TarHandler 处理

### 4. RAR 格式 - RarHandler

**技术实现**: `rar` crate (0.4) 纯 Rust + `unrar` 二进制 Fallback

**支持格式**: `.rar`, `.RAR`

**架构设计**: 双模式策略
- **主模式**: `rar` crate - 纯 Rust 实现，支持基础 RAR4 格式
- **Fallback**: `unrar` 二进制 - 处理复杂 RAR5/多部分/加密文件

**特性**:
- 纯 Rust 实现，无需 C 库依赖
- 自动检测并处理不兼容格式
- 跨平台兼容性 (Windows/macOS/Linux)
- 无需外置 RAR 软件

**依赖**:
```toml
# Cargo.toml
rar = "0.4"  # Pure Rust RAR library
```

**fallback 二进制路径**:
```
binaries/
├── unrar-x86_64-pc-windows-msvc.exe
├── unrar-x86_64-apple-darwin
├── unrar-aarch64-apple-darwin
└── unrar-x86_64-unknown-linux-gnu
```

**平台选择逻辑**:
```rust
fn get_unrar_path() -> Result<PathBuf> {
    let (arch, os, ext) = detect_platform();
    let binary_name = format!("unrar-{arch}-{os}{ext}");
    Ok(assets_dir().join("binaries").join(binary_name))
}
```

## 核心组件

### ArchiveManager - 处理器管理器

```rust
pub struct ArchiveManager {
    handlers: Vec<Box<dyn ArchiveHandler>>,
    max_file_size: u64,      // 单文件最大大小
    max_total_size: u64,     // 总大小限制
    max_file_count: usize,   // 文件数量限制
}
```

**核心方法**:

1. **find_handler** - 查找合适处理器
```rust
pub fn find_handler(&self, path: &Path) -> Option<&dyn ArchiveHandler> {
    self.handlers
        .iter()
        .find(|h| h.can_handle(path))
        .map(|h| h.as_ref())
}
```

2. **extract_archive** - 统一解压入口
```rust
pub async fn extract_archive(
    &self,
    source: &Path,
    target_dir: &Path,
) -> Result<ExtractionSummary> {
    let handler = self.find_handler(source)
        .ok_or_else(|| AppError::archive_error(...))?;

    handler.extract_with_limits(
        source, target_dir,
        self.max_file_size,
        self.max_total_size,
        self.max_file_count
    ).await
}
```

### ExtractionSummary - 解压摘要

```rust
pub struct ExtractionSummary {
    pub extracted_files: Vec<(PathBuf, u64)>,  // (路径, 大小)
    pub total_size: u64,
    pub file_count: usize,
    pub errors: Vec<String>,
    pub source_path: PathBuf,
}
```

**方法**:
- `add_file` - 添加提取文件
- `add_error` - 记录错误
- `is_empty` - 检查是否为空

## 安全特性

### 1. 文件大小限制
```rust
max_file_size: 100 * 1024 * 1024,  // 100MB
```
- 防止超大文件耗尽内存
- 大文件自动跳过并记录

### 2. 总大小限制
```rust
max_total_size: 1024 * 1024 * 1024,  // 1GB
```
- 防止解压耗尽磁盘空间
- 实时累计检查

### 3. 文件数量限制
```rust
max_file_count: 1000,  // 1000文件
```
- 防止目录遍历攻击
- 大量小文件保护

### 4. 路径安全
- 防止路径穿越攻击
- 相对路径规范化
- Windows UNC 路径支持

## 错误处理

### AppError 扩展
```rust
AppError::Archive {
    message: String,
    path: Option<PathBuf>,
}
```

**错误场景**:
- 不支持的压缩格式
- 文件损坏/无法读取
- 密码保护（暂不支持）
- 磁盘空间不足
- 权限不足

**错误上下文**:
```rust
pub fn with_context(self, context: impl Into<String>) -> Self {
    match self {
        AppError::Archive { message, path } => AppError::Archive {
            message: format!("{}: {}", context.into(), message),
            path,
        },
        other => other,
    }
}
```

## 递归解压

### 处理嵌套压缩包

```mermaid
graph LR
    A[logs.zip] --> B[archive.tar.gz]
    B --> C[log1.gz]
    B --> D[log2.txt]
    C --> E[actual_log.log]
```

**处理流程**:
1. 解压第一层压缩包
2. 扫描提取的文件
3. 检测新的压缩文件
4. 递归处理直到所有文件解压完成
5. 过滤出日志文件

**实现代码**:
```rust
pub async fn process_path_recursive_with_metadata(
    path: &Path,
    extracted_files: &mut Vec<ExtractedFile>,
    workspace_id: &str,
    file_metadata: &mut HashMap<String, FileMetadata>,
) -> Result<()> {
    // 1. 检测是否为压缩文件
    if let Some(handler) = archive_manager.find_handler(path) {
        // 2. 解压到临时目录
        let temp_dir = tempfile::tempdir()?;
        handler.extract_with_limits(...).await?;

        // 3. 递归处理解压的文件
        for entry in walkdir::WalkDir::new(temp_dir.path()) {
            let entry = entry?;
            if entry.file_type().is_file() {
                process_file(entry.path(), extracted_files, workspace_id, file_metadata).await?;
            }
        }
    } else {
        // 4. 直接处理普通文件
        process_file(path, extracted_files, workspace_id, file_metadata).await?;
    }

    Ok(())
}
```

## 测试策略

### 测试覆盖
- **单元测试**: 各处理器独立测试
- **集成测试**: 完整解压流程测试
- **边界测试**: 错误场景和限制测试

### TarHandler 测试示例
```rust
#[tokio::test]
async fn test_extract_tar_file() {
    let temp_dir = TempDir::new().unwrap();
    let tar_file = temp_dir.path().join("test.tar");

    // 创建测试 TAR 文件
    let data1 = b"This is test file 1";
    let data2 = b"This is test file 2 with more content";

    let header1 = Header::new_gnu();
    header1.set_path("file1.txt").unwrap();
    header1.set_size(data1.len() as u64).unwrap();
    header1.set_cksum();

    let header2 = Header::new_gnu();
    header2.set_path("file2.txt").unwrap();
    header2.set_size(data2.len() as u64).unwrap();
    header2.set_cksum();

    // 验证处理能力
    assert!(handler.can_handle(Path::new("test.tar")));
    assert!(handler.can_handle(Path::new("test.tar.gz")));
    assert!(!handler.can_handle(Path::new("test.gz"))); // 纯GZ由GzHandler处理
}
```

### ArchiveManager 测试示例
```rust
#[tokio::test]
async fn test_extract_unsupported_format() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.txt");

    fs::write(&source_file, "test content").unwrap();

    let manager = ArchiveManager::new();
    let result = manager.extract_archive(&source_file, temp_dir.path()).await;

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, AppError::Archive { .. }));
    }
}
```

## 性能优化

### 1. 流式处理
- 大文件分块读取
- 内存占用优化
- 避免一次性加载全部内容

### 2. 并行解压
- 多个压缩包并行处理
- 独立文件并行索引
- Rayon 线程池加速

### 3. 缓存机制
- 解压结果缓存
- 文件元数据缓存
- 处理器实例缓存

## 常见问题 (FAQ)

### Q: 如何添加新的压缩格式？
A:
1. 实现 `ArchiveHandler` Trait
2. 在 `ArchiveManager::new()` 中注册
3. 添加对应的测试用例
4. 更新文档

### Q: 密码保护的 ZIP 文件？
A: 当前版本不支持密码保护文件，检测到会跳过并记录警告。

### Q: RAR5 格式支持？
A: 使用 `unrar` crate 0.5 版本，已支持 RAR5 格式。

### Q: 如何调整安全限制？
A: 修改 `ArchiveManager::new()` 中的默认值：
```rust
max_file_size: 200 * 1024 * 1024,  // 200MB
max_total_size: 2 * 1024 * 1024 * 1024,  // 2GB
max_file_count: 2000,  // 2000文件
```

## 相关文件清单

### 核心文件
- `archive_handler.rs` - Handler Trait 定义
- `mod.rs` - 管理器和模块入口

### 具体处理器
- `zip_handler.rs` - ZIP 格式处理
- `tar_handler.rs` - TAR/TAR.GZ 处理
- `gz_handler.rs` - 纯 GZ 处理
- `rar_handler.rs` - RAR 格式处理

### 工具文件
- `processor.rs` - 递归处理工具

### 测试文件
- 各处理器内的 `#[cfg(test)] mod tests`
- `tests/` 目录下的集成测试

---

## 变更记录 (Changelog)

### [2026-01-09] CAS架构性能优化

- ✅ **CAS对象存在性缓存优化**
  - 使用 `DashSet` 替代文件系统检查，减少 I/O 操作
  - 线程安全的高性能并发访问
  - 自动缓存新创建的对象

- ✅ **存储大小计算优化**
  - 使用 `walkdir` 替代递归遍历，显著提升大目录性能
  - 单次遍历计算所有文件大小

- ✅ **SQLite 性能优化**
  - 启用 WAL 模式，支持并发读写
  - 设置 `synchronous = NORMAL` 提升写入性能
  - 增加连接池大小 (5 → 10)
  - 增大缓存大小 (-8000 pages ≈ 8MB)

### [2026-01-09] RAR处理器纯Rust重构

- ✅ **新增 rar crate 纯 Rust 支持**
  - 使用 `rar = "0.4"` 替代部分 unrar C 绑定依赖
  - 主模式使用纯 Rust 实现，基础 RAR4 格式无需外部二进制
  - 保持 unrar 二进制作为 Fallback，处理 RAR5/加密/多部分文件
- ✅ **解决 macOS ARM64 构建问题**
  - 移除 unrar Rust crate（依赖 C 库，有平台兼容性问题）
  - 采用 sidecar 二进制方案，纯 Rust 主库 + 平台特定二进制
- ✅ **代码质量**
  - cargo check 通过
  - cargo clippy 无警告
  - 530+ 测试用例通过

### [2025-12-13] AI上下文初始化
- ✅ 完整架构分析
- ✅ Handler Trait 设计梳理
- ✅ 各格式处理器功能总结
- ✅ 安全特性和测试策略整理

### [历史版本]
- 详见根目录 CHANGELOG.md

---

*本文档由 AI 架构师自动生成，基于压缩处理模块代码分析*
