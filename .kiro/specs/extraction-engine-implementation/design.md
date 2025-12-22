# 设计文档

## 概述

本设计文档描述了提取引擎（ExtractionEngine）的完整实现方案。当前的`process_archive_file`方法只有占位符实现，导致所有集成测试失败。本设计将实现完整的归档文件提取逻辑，支持ZIP、RAR、TAR和GZ格式，并与现有的路径管理、安全检测和嵌套归档处理系统无缝集成。

设计的核心目标是：
1. 实现完整的归档文件提取逻辑
2. 支持多种归档格式（ZIP、RAR、TAR、GZ）
3. 处理嵌套归档的深度优先遍历
4. 集成路径管理和安全检测
5. 提供准确的性能指标和错误报告

## 架构

### 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    ExtractionEngine                          │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         process_archive_file (核心方法)              │   │
│  │  1. 识别归档格式                                      │   │
│  │  2. 选择合适的Handler                                 │   │
│  │  3. 提取文件                                          │   │
│  │  4. 识别嵌套归档                                      │   │
│  │  5. 应用安全检查                                      │   │
│  └──────────────────────────────────────────────────────┘   │
│                           │                                   │
│                           ▼                                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │           ArchiveHandler Trait                        │   │
│  │  - can_handle(path) -> bool                           │   │
│  │  - extract_with_limits(...) -> ExtractionSummary      │   │
│  └──────────────────────────────────────────────────────┘   │
│         │           │           │           │                 │
│         ▼           ▼           ▼           ▼                 │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │  Zip    │ │  Rar    │ │  Tar    │ │   Gz    │           │
│  │ Handler │ │ Handler │ │ Handler │ │ Handler │           │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
└─────────────────────────────────────────────────────────────┘
         │                                    │
         ▼                                    ▼
┌──────────────────┐              ┌──────────────────┐
│  PathManager     │              │ SecurityDetector │
│  - 路径缩短      │              │  - zip炸弹检测   │
│  - 路径映射      │              │  - 路径遍历检测  │
└──────────────────┘              └──────────────────┘
```

### 数据流

```
归档文件 → 格式识别 → Handler选择 → 提取文件 → 安全检查 → 路径处理 → 嵌套检测 → 结果返回
                                        │
                                        ├→ PathManager (长路径处理)
                                        ├→ SecurityDetector (安全检查)
                                        └→ ExtractionStack (嵌套归档)
```

## 组件和接口

### 1. ExtractionEngine核心方法

#### process_archive_file方法签名

```rust
async fn process_archive_file(
    &self,
    item: &ExtractionItem,
    stack: &mut ExtractionStack,
) -> Result<Vec<PathBuf>>
```

**职责：**
- 识别归档格式
- 选择合适的Handler
- 调用Handler提取文件
- 识别嵌套归档并添加到栈
- 应用路径管理和安全检测
- 返回提取的文件路径列表

### 2. ArchiveHandler选择器

#### HandlerRegistry结构

```rust
struct HandlerRegistry {
    handlers: Vec<Box<dyn ArchiveHandler>>,
}

impl HandlerRegistry {
    fn new() -> Self;
    fn register(&mut self, handler: Box<dyn ArchiveHandler>);
    fn find_handler(&self, path: &Path) -> Option<&dyn ArchiveHandler>;
}
```

**职责：**
- 管理所有可用的Handler
- 根据文件扩展名选择合适的Handler
- 提供Handler查找功能

### 3. 嵌套归档检测

#### is_archive_file函数

```rust
fn is_archive_file(path: &Path) -> bool {
    let extensions = ["zip", "rar", "tar", "gz", "tgz", "tar.gz"];
    // 检查文件扩展名
}
```

**职责：**
- 判断文件是否为归档文件
- 支持多种归档格式识别

### 4. 路径处理集成

#### resolve_extraction_path方法

```rust
async fn resolve_extraction_path(
    &self,
    workspace_id: &str,
    full_path: &Path,
) -> Result<(PathBuf, bool)>
```

**返回：**
- `(resolved_path, was_shortened)` - 解析后的路径和是否被缩短的标志

**职责：**
- 使用PathManager处理长路径
- 记录路径映射到元数据数据库
- 返回缩短标志用于警告记录

### 5. 安全检测集成

#### check_security方法

```rust
async fn check_security(
    &self,
    archive_path: &Path,
    entry_path: &Path,
    size: u64,
) -> Result<()>
```

**职责：**
- 检测路径遍历攻击
- 检测zip炸弹（高压缩比）
- 验证文件大小限制
- 拒绝非法路径

## 数据模型

### ExtractionItem (已存在)

```rust
pub struct ExtractionItem {
    pub archive_path: PathBuf,
    pub target_dir: PathBuf,
    pub depth: usize,
    pub parent_context: ExtractionContext,
}
```

### ExtractionSummary (已存在)

```rust
pub struct ExtractionSummary {
    pub files_extracted: usize,
    pub total_size: u64,
    pub errors: Vec<String>,
    pub extracted_files: Vec<PathBuf>,
}
```

### HandlerRegistry (新增)

```rust
struct HandlerRegistry {
    handlers: Vec<Box<dyn ArchiveHandler>>,
}
```

### FileEntry (内部使用)

```rust
struct FileEntry {
    path: PathBuf,
    size: u64,
    is_archive: bool,
}
```

## 正确性属性

*属性是一个特征或行为，应该在系统的所有有效执行中保持为真——本质上是关于系统应该做什么的正式陈述。属性作为人类可读规范和机器可验证正确性保证之间的桥梁。*

### 属性 1: 文件提取完整性

*对于任何*有效的归档文件和提取策略，提取后的文件数量应该等于归档中的文件数量（排除目录和被安全策略拒绝的文件）

**验证: 需求 1.1, 1.3**

### 属性 2: 嵌套归档识别

*对于任何*包含嵌套归档的归档文件，如果嵌套深度未超过限制，则所有嵌套归档都应该被识别并添加到提取栈

**验证: 需求 2.1, 2.2**

### 属性 3: 路径缩短一致性

*对于任何*超过系统限制的文件路径，缩短后的路径应该能够通过PathManager恢复到原始路径

**验证: 需求 3.1, 3.2, 3.4**

### 属性 4: 安全检测有效性

*对于任何*包含路径遍历尝试或异常压缩比的归档文件，SecurityDetector应该检测并拒绝提取

**验证: 需求 4.1, 4.2, 4.3**

### 属性 5: 格式处理正确性

*对于任何*支持的归档格式（ZIP、RAR、TAR、GZ），系统应该选择正确的Handler并成功提取

**验证: 需求 5.1, 5.2, 5.3, 5.4**

### 属性 6: 大小限制遵守

*对于任何*提取操作，总提取大小不应超过ExtractionPolicy中定义的max_total_size限制

**验证: 需求 6.3, 6.4**

### 属性 7: 并行提取安全性

*对于任何*并行提取操作，同时进行的提取任务数量不应超过max_parallel_files配置

**验证: 需求 7.1, 7.2**

### 属性 8: 结果准确性

*对于任何*提取操作，返回的ExtractionResult应该准确反映提取的文件数量、字节数和最大深度

**验证: 需求 8.1, 8.2, 8.3**

### 属性 9: 深度限制遵守

*对于任何*嵌套归档，当深度达到max_depth限制时，系统应该停止提取并记录警告，而不是继续提取

**验证: 需求 2.3**

### 属性 10: 错误处理鲁棒性

*对于任何*提取过程中的单个文件错误，系统应该记录警告但继续处理其他文件，而不是完全失败

**验证: 需求 7.3**

## 错误处理

### 错误类型

1. **格式错误**
   - 不支持的归档格式
   - 损坏的归档文件
   - 处理：返回UnsupportedFormat或CorruptedArchive错误

2. **安全错误**
   - 路径遍历尝试
   - Zip炸弹检测
   - 处理：立即停止提取，返回安全错误

3. **资源错误**
   - 磁盘空间不足
   - 文件大小超限
   - 处理：停止提取，返回资源错误

4. **权限错误**
   - 无法创建目录
   - 无法写入文件
   - 处理：记录警告，跳过该文件，继续处理

### 错误恢复策略

1. **单文件错误**：记录警告，继续处理其他文件
2. **归档级错误**：停止当前归档，继续处理栈中其他归档
3. **系统级错误**：停止所有提取，返回错误

## 测试策略

### 单元测试

1. **格式识别测试**
   - 测试各种文件扩展名的识别
   - 测试大小写不敏感
   - 测试复合扩展名（如.tar.gz）

2. **Handler选择测试**
   - 测试HandlerRegistry的注册和查找
   - 测试未知格式的处理

3. **路径处理测试**
   - 测试长路径的缩短
   - 测试路径映射的记录
   - 测试路径恢复

4. **安全检测测试**
   - 测试路径遍历检测
   - 测试压缩比检测
   - 测试文件大小限制

### 属性测试

1. **属性 1: 文件提取完整性测试**
   ```rust
   #[test]
   fn prop_extraction_completeness() {
       // 对于任何有效归档，提取的文件数应该正确
       // 生成随机归档文件
       // 提取并验证文件数量
   }
   ```

2. **属性 3: 路径缩短一致性测试**
   ```rust
   #[test]
   fn prop_path_shortening_roundtrip() {
       // 对于任何长路径，缩短后应该能恢复
       // 生成随机长路径
       // 缩短并恢复
       // 验证一致性
   }
   ```

3. **属性 6: 大小限制遵守测试**
   ```rust
   #[test]
   fn prop_size_limit_enforcement() {
       // 对于任何提取操作，不应超过大小限制
       // 生成随机归档和限制
       // 提取并验证总大小
   }
   ```

4. **属性 9: 深度限制遵守测试**
   ```rust
   #[test]
   fn prop_depth_limit_enforcement() {
       // 对于任何嵌套归档，应该遵守深度限制
       // 生成随机嵌套归档
       // 提取并验证最大深度
   }
   ```

### 集成测试

1. **基本提取测试**
   - 测试ZIP、RAR、TAR、GZ格式的提取
   - 验证文件内容正确性
   - 验证目录结构保持

2. **嵌套归档测试**
   - 测试单层嵌套
   - 测试多层嵌套
   - 测试深度限制

3. **长路径测试**
   - 测试超长文件名
   - 测试超长目录路径
   - 验证路径映射

4. **安全测试**
   - 测试路径遍历防护
   - 测试zip炸弹检测
   - 测试大小限制

5. **并发测试**
   - 测试并行提取
   - 测试并发限制
   - 测试资源竞争

### 测试数据生成

使用proptest库生成测试数据：

```rust
use proptest::prelude::*;

// 生成随机归档文件
prop_compose! {
    fn arb_archive_file()(
        file_count in 1..100usize,
        max_size in 1..1000000u64,
    ) -> Vec<(String, Vec<u8>)> {
        // 生成随机文件列表
    }
}

// 生成随机嵌套归档
prop_compose! {
    fn arb_nested_archive()(
        depth in 1..5usize,
        files_per_level in 1..10usize,
    ) -> NestedArchive {
        // 生成嵌套归档结构
    }
}
```

## 性能考虑

### 优化策略

1. **流式提取**
   - 使用缓冲区避免一次性加载大文件到内存
   - 配置的buffer_size用于批处理

2. **并行提取**
   - 使用Semaphore控制并行度
   - 遵守max_parallel_files配置

3. **路径缓存**
   - 使用DashMap缓存路径映射
   - 减少数据库查询

4. **Handler复用**
   - Handler实例在ExtractionEngine生命周期内复用
   - 避免重复初始化

### 性能指标

1. **提取速度**：MB/s
2. **内存使用**：峰值内存占用
3. **并发效率**：并行提取的加速比
4. **缓存命中率**：路径缓存的命中率

## 实现细节

### process_archive_file实现流程

```rust
async fn process_archive_file(
    &self,
    item: &ExtractionItem,
    stack: &mut ExtractionStack,
) -> Result<Vec<PathBuf>> {
    // 1. 确保目标目录存在
    fs::create_dir_all(&item.target_dir).await?;
    
    // 2. 创建Handler注册表
    let registry = self.create_handler_registry();
    
    // 3. 查找合适的Handler
    let handler = registry.find_handler(&item.archive_path)
        .ok_or_else(|| AppError::unsupported_format(&item.archive_path))?;
    
    // 4. 提取归档文件
    let summary = handler.extract_with_limits(
        &item.archive_path,
        &item.target_dir,
        self.policy.max_file_size,
        self.policy.max_total_size,
        usize::MAX, // 文件数量限制由外层控制
    ).await?;
    
    // 5. 处理提取的文件
    let mut extracted_files = Vec::new();
    
    for file_path in summary.extracted_files {
        // 5.1 检查是否为嵌套归档
        if is_archive_file(&file_path) && item.depth + 1 < self.policy.max_depth {
            // 创建嵌套提取项
            let nested_item = ExtractionItem::new(
                file_path.clone(),
                item.target_dir.join(file_path.file_stem().unwrap()),
                item.depth + 1,
                item.parent_context.clone(),
            );
            
            // 添加到栈
            stack.push(nested_item)?;
        }
        
        // 5.2 应用路径处理
        let (resolved_path, was_shortened) = self.resolve_extraction_path(
            &item.parent_context.workspace_id,
            &file_path,
        ).await?;
        
        // 5.3 记录缩短警告
        if was_shortened {
            // 警告将在外层记录
        }
        
        extracted_files.push(resolved_path);
    }
    
    Ok(extracted_files)
}
```

### HandlerRegistry实现

```rust
struct HandlerRegistry {
    handlers: Vec<Box<dyn ArchiveHandler>>,
}

impl HandlerRegistry {
    fn new() -> Self {
        let mut registry = Self {
            handlers: Vec::new(),
        };
        
        // 注册所有Handler
        registry.register(Box::new(ZipHandler));
        registry.register(Box::new(RarHandler));
        registry.register(Box::new(TarHandler));
        registry.register(Box::new(GzHandler));
        
        registry
    }
    
    fn register(&mut self, handler: Box<dyn ArchiveHandler>) {
        self.handlers.push(handler);
    }
    
    fn find_handler(&self, path: &Path) -> Option<&dyn ArchiveHandler> {
        self.handlers
            .iter()
            .find(|h| h.can_handle(path))
            .map(|h| h.as_ref())
    }
}
```

### 嵌套归档检测实现

```rust
fn is_archive_file(path: &Path) -> bool {
    let extensions = ["zip", "rar", "tar", "gz", "tgz"];
    
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if extensions.contains(&ext.to_lowercase().as_str()) {
            return true;
        }
        
        // 检查.tar.gz
        if ext.eq_ignore_ascii_case("gz") {
            if let Some(stem) = path.file_stem() {
                if let Some(stem_str) = stem.to_str() {
                    return stem_str.ends_with(".tar");
                }
            }
        }
    }
    
    false
}
```

## 依赖关系

### 内部依赖

- `archive::archive_handler::ArchiveHandler` - Handler trait
- `archive::zip_handler::ZipHandler` - ZIP处理器
- `archive::rar_handler::RarHandler` - RAR处理器
- `archive::tar_handler::TarHandler` - TAR处理器
- `archive::gz_handler::GzHandler` - GZ处理器
- `archive::path_manager::PathManager` - 路径管理
- `archive::security_detector::SecurityDetector` - 安全检测
- `archive::extraction_context::ExtractionContext` - 提取上下文
- `archive::extraction_context::ExtractionStack` - 提取栈

### 外部依赖

- `tokio::fs` - 异步文件系统操作
- `tracing` - 日志记录
- `dashmap::DashMap` - 并发HashMap
- `proptest` - 属性测试（测试依赖）

## 配置

### ExtractionPolicy配置项

```rust
pub struct ExtractionPolicy {
    pub max_depth: usize,           // 最大嵌套深度 (默认: 10)
    pub max_file_size: u64,         // 单文件最大大小 (默认: 100MB)
    pub max_total_size: u64,        // 总提取大小限制 (默认: 10GB)
    pub buffer_size: usize,         // 缓冲区大小 (默认: 64KB)
    pub dir_batch_size: usize,      // 目录批处理大小 (默认: 10)
    pub max_parallel_files: usize,  // 最大并行文件数 (默认: 4)
}
```

### 配置验证

所有配置项都通过`ExtractionPolicy::validate()`方法验证：
- max_depth: 1-20
- max_file_size: > 0
- max_total_size: > 0
- buffer_size: > 0
- dir_batch_size: > 0
- max_parallel_files: > 0

## 安全考虑

### 路径遍历防护

- 使用`path_security`模块验证所有提取路径
- 拒绝包含`..`的路径
- 拒绝绝对路径
- 清理非法字符

### Zip炸弹检测

- 使用SecurityDetector检测异常压缩比
- 限制单文件大小
- 限制总提取大小
- 限制文件数量

### 资源限制

- 内存：使用流式提取避免大文件占用内存
- 磁盘：检查可用空间，遵守大小限制
- CPU：使用Semaphore限制并发度

## 监控和日志

### 日志级别

- **DEBUG**: 详细的提取过程信息
- **INFO**: 提取开始/完成，统计信息
- **WARN**: 警告（深度限制、路径缩短等）
- **ERROR**: 提取失败，安全威胁

### 监控指标

- 提取成功率
- 平均提取时间
- 平均提取速度
- 路径缩短频率
- 安全事件频率
- 并发利用率

## 向后兼容性

### 与ArchiveManager的兼容性

- 保持相同的提取结果格式
- 支持相同的归档格式
- 提供相同的错误处理行为
- 集成测试验证兼容性

### 迁移策略

1. 新代码使用ExtractionEngine
2. 旧代码继续使用ArchiveManager
3. 逐步迁移到新系统
4. 最终废弃ArchiveManager

## 未来扩展

### 可能的增强

1. **更多格式支持**
   - 7z格式
   - bz2格式
   - xz格式

2. **增量提取**
   - 只提取变更的文件
   - 支持断点续传

3. **并行归档处理**
   - 同时处理多个归档
   - 更高的吞吐量

4. **智能缓存**
   - 缓存常用归档的索引
   - 加速重复提取

5. **压缩级别控制**
   - 支持不同压缩级别
   - 平衡速度和压缩比
