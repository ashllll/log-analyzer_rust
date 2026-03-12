# ArchiveHandler Trait 重构设计方案

## 1. 设计概述

### 1.1 当前问题分析

基于对现有代码的分析，发现以下主要问题：

| 问题 | 位置 | 影响 |
|------|------|------|
| 安全检查逻辑重复 | `zip_handler.rs:71-101`, `tar_handler.rs:309-340`, `rar_handler.rs:82-89`, `sevenz_handler.rs:72-77` | 维护困难，易遗漏 |
| 参数过多 | `extract_with_limits()` 有5个参数 | API 使用不便，容易传错 |
| 缺乏抽象基类 | 无法复用公共代码 | 代码冗余 |
| 错误处理不一致 | RAR/7z handler 缺少详细日志 | 问题排查困难 |

### 1.2 设计目标

- ✅ 消除安全检查逻辑重复
- ✅ 使用配置对象替代多个参数
- ✅ 提供默认实现的基类 trait
- ✅ 保持向后兼容性
- ✅ 统一错误处理和日志输出

---

## 2. 新的 Trait 层次结构

### 2.1 架构图

```mermaid
classDiagram
    direction TB
    
    class ArchiveHandler {
        <<trait>>
        +can_handle(path: &Path) bool
        +extract(source: &Path, target: &Path, config: &ExtractionConfig) Result~ExtractionSummary~
        +extract_with_limits(source: &Path, target: &Path, max_file_size: u64, max_total_size: u64, max_file_count: usize) Result~ExtractionSummary~
        +file_extensions() Vec~&str~
        +list_contents(path: &Path) Result~Vec~ArchiveEntry~~
        +read_file(path: &Path, file_name: &str) Result~String~
    }
    
    class ArchiveHandlerBase {
        <<trait>>
        +extract(source: &Path, target: &Path, config: &ExtractionConfig) Result~ExtractionSummary~
        +check_limits(file_size: u64, ctx: &mut ExtractionContext) Result~()~
        +validate_and_track(path: &str, size: u64, ctx: &mut ExtractionContext) Result~PathBuf~
    }
    
    class ExtractionConfig {
        +max_file_size: u64
        +max_total_size: u64
        +max_file_count: usize
        +max_depth: u32
        +security_config: SecurityConfig
        +default() ExtractionConfig
    }
    
    class ExtractionContext {
        +summary: ExtractionSummary
        +config: &ExtractionConfig
        +depth: u32
        +check_limits(size: u64) Result~()~
        +add_file(path: PathBuf, size: u64)
        +add_error(message: String)
    }
    
    class ExtractionLimiter {
        +check(size: u64) Result~()~
        +track(path: PathBuf, size: u64)
    }
    
    class ExtractionError {
        <<enum>>
        +FileSizeExceeded { max: u64, actual: u64 }
        +TotalSizeExceeded { max: u64, actual: u64 }
        +FileCountExceeded { max: usize, actual: usize }
        +DepthExceeded { max: u32, actual: u32 }
        +PathSecurity Violated { path: String }
    }
    
    class ZipHandler {
        +can_handle(path: &Path) bool
        +extract_with_config(archive: &mut ZipArchive, target: &Path, ctx: &mut ExtractionContext) Result~()~
    }
    
    class TarHandler { }
    class GzHandler { }
    class RarHandler { }
    class SevenZHandler { }
    
    ArchiveHandler <|-- ArchiveHandlerBase
    ArchiveHandlerBase <|-- ZipHandler
    ArchiveHandlerBase <|-- TarHandler
    ArchiveHandlerBase <|-- GzHandler
    ArchiveHandlerBase <|-- RarHandler
    ArchiveHandlerBase <|-- SevenZHandler
    
    ArchiveHandlerBase ..> ExtractionConfig: uses
    ArchiveHandlerBase ..> ExtractionContext: uses
    ArchiveHandlerBase ..> ExtractionLimiter: uses
    ExtractionContext --> ExtractionError
```

### 2.2 核心 Trait 定义

#### 2.2.1 ExtractionConfig 结构体

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 提取配置 - 封装所有提取限制参数
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// 单个文件最大大小（字节）
    pub max_file_size: u64,
    /// 解压后总大小限制（字节）
    pub max_total_size: u64,
    /// 解压文件数量限制
    pub max_file_count: usize,
    /// 最大解压深度（防止zip炸弹）
    pub max_depth: u32,
    /// 读取文件时的最大大小（用于预览）
    pub max_read_size: u64,
    /// 安全配置
    pub security_config: SecurityConfig,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,      // 100MB
            max_total_size: 1024 * 1024 * 1024,    // 1GB
            max_file_count: 1000,
            max_depth: 10,
            max_read_size: 10 * 1024 * 1024,       // 10MB
            security_config: SecurityConfig::default(),
        }
    }
}

impl ExtractionConfig {
    /// 创建带有自定义限制的配置
    pub fn with_limits(
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Self {
        Self {
            max_file_size,
            max_total_size,
            max_file_count,
            ..Default::default()
        }
    }
}

/// 安全配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 是否允许符号链接
    pub allow_symlinks: bool,
    /// 是否允许绝对路径
    pub allow_absolute_paths: bool,
    /// 是否允许父目录遍历 (..)
    pub allow_parent_traversal: bool,
    /// 路径黑名单
    pub path_blacklist: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_symlinks: false,
            allow_absolute_paths: false,
            allow_parent_traversal: false,
            path_blacklist: vec![
                "/etc".to_string(),
                "/Windows".to_string(),
                "/System Volume Information".to_string(),
            ],
        }
    }
}
```

#### 2.2.2 ExtractionContext 结构体

```rust
use std::path::PathBuf;
use crate::archive::archive_handler::{ExtractionSummary, ArchiveEntry};
use crate::error::Result;

/// 提取上下文 - 维护提取过程中的状态
pub struct ExtractionContext {
    /// 当前配置引用
    pub config: ExtractionConfig,
    /// 提取摘要
    pub summary: ExtractionSummary,
    /// 当前深度
    pub depth: u32,
    /// 源文件路径
    pub source_path: PathBuf,
    /// 目标目录
    pub target_dir: PathBuf,
}

impl ExtractionContext {
    pub fn new(config: ExtractionConfig, source: PathBuf, target: PathBuf) -> Self {
        Self {
            config,
            summary: ExtractionSummary::new(),
            depth: 0,
            source_path: source,
            target_dir: target,
        }
    }

    /// 检查大小限制
    pub fn check_limits(&self, file_size: u64) -> Result<()> {
        // 检查单个文件大小
        if file_size > self.config.max_file_size {
            return Err(ExtractionError::file_size_exceeded(
                self.config.max_file_size,
                file_size,
            ).into());
        }
        
        // 检查总大小
        if self.summary.total_size + file_size > self.config.max_total_size {
            return Err(ExtractionError::total_size_exceeded(
                self.config.max_total_size,
                self.summary.total_size + file_size,
            ).into());
        }
        
        // 检查文件数量
        if self.summary.files_extracted + 1 > self.config.max_file_count {
            return Err(ExtractionError::file_count_exceeded(
                self.config.max_file_count,
                self.summary.files_extracted + 1,
            ).into());
        }
        
        // 检查深度
        if self.depth > self.config.max_depth {
            return Err(ExtractionError::depth_exceeded(
                self.config.max_depth,
                self.depth,
            ).into());
        }
        
        Ok(())
    }

    /// 添加成功提取的文件
    pub fn add_file(&mut self, path: PathBuf, size: u64) {
        self.summary.add_file(path, size);
    }

    /// 添加错误信息
    pub fn add_error(&mut self, error: String) {
        self.summary.add_error(error);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        self.summary.has_errors()
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f32 {
        self.summary.success_rate()
    }
}
```

#### 2.2.3 ExtractionError 枚举

```rust
use thiserror::Error;

/// 提取错误枚举
#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error("文件大小 {actual} bytes 超过限制 {max} bytes")]
    FileSizeExceeded { max: u64, actual: u64 },

    #[error("总大小 {actual} bytes 超过限制 {max} bytes")]
    TotalSizeExceeded { max: u64, actual: u64 },

    #[error("文件数量 {actual} 超过限制 {max}")]
    FileCountExceeded { max: usize, actual: usize },

    #[error("深度 {actual} 超过限制 {max}")]
    DepthExceeded { max: u32, actual: u32 },

    #[error("路径安全违规: {0}")]
    PathSecurityViolated(String),

    #[error("不支持的压缩格式: {0}")]
    UnsupportedFormat(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("提取错误: {0}")]
    ExtractionFailed(String),
}

impl ExtractionError {
    pub fn file_size_exceeded(max: u64, actual: u64) -> Self {
        Self::FileSizeExceeded { max, actual }
    }

    pub fn total_size_exceeded(max: u64, actual: u64) -> Self {
        Self::TotalSizeExceeded { max, actual }
    }

    pub fn file_count_exceeded(max: usize, actual: usize) -> Self {
        Self::FileCountExceeded { max, actual }
    }

    pub fn depth_exceeded(max: u32, actual: u32) -> Self {
        Self::DepthExceeded { max, actual }
    }

    pub fn path_security_violated(path: &str) -> Self {
        Self::PathSecurityViolated(path.to_string())
    }
}
```

#### 2.2.4 ArchiveHandler Trait (改进版)

```rust
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::error::{AppError, Result};
use super::{ExtractionConfig, ExtractionContext};

/// 压缩文件处理器 Trait - 核心接口（保持向后兼容）
#[async_trait]
pub trait ArchiveHandler: Send + Sync {
    /// 检查是否能处理该文件
    fn can_handle(&self, path: &Path) -> bool;

    /// 提取压缩文件内容（带配置）
    async fn extract(
        &self,
        source: &Path,
        target_dir: &Path,
        config: &ExtractionConfig,
    ) -> Result<ExtractionSummary>;

    /// 提取压缩文件内容（带限制参数 - 向后兼容）
    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        let config = ExtractionConfig::with_limits(max_file_size, max_total_size, max_file_count);
        self.extract(source, target_dir, &config).await
    }

    /// 获取支持的文件扩展名
    fn file_extensions(&self) -> Vec<&str>;

    /// 列出压缩包内容
    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>>;

    /// 读取单个文件内容
    async fn read_file(&self, path: &Path, file_name: &str) -> Result<String>;
}
```

#### 2.2.5 ArchiveHandlerBase Trait (新增)

```rust
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::error::{AppError, Result};
use super::{ExtractionConfig, ExtractionContext, ExtractionSummary};
use crate::utils::path_security::{validate_and_sanitize_archive_path, PathValidationResult};

/// ArchiveHandler 基类 Trait - 提供默认实现
/// 
/// 这个 trait 封装了所有公共的安全检查和限制验证逻辑，
/// 子类只需实现 `extract_entries` 方法即可
#[async_trait]
pub trait ArchiveHandlerBase: ArchiveHandler {
    /// 提取条目 - 子类必须实现
    async fn extract_entries<R>(
        &self,
        archive: &mut R,
        ctx: &mut ExtractionContext,
    ) -> Result<()>
    where
        R: Send;

    /// 验证并跟踪文件 - 提供默认实现
    fn validate_and_track(
        &self,
        entry_name: &str,
        size: u64,
        ctx: &mut ExtractionContext,
    ) -> Result<PathBuf> {
        // 1. 检查限制
        ctx.check_limits(size)?;

        // 2. 验证路径安全
        let validation = validate_and_sanitize_archive_path(
            entry_name,
            &ctx.config.security_config,
        );

        let safe_path = match validation {
            PathValidationResult::Unsafe(reason) => {
                tracing::warn!(entry = %entry_name, reason = %reason, "Skipping unsafe path");
                ctx.add_error(format!("Path security violation: {}", reason));
                return Err(ExtractionError::path_security_violated(entry_name).into());
            }
            PathValidationResult::Valid(p) => PathBuf::from(p),
            PathValidationResult::RequiresSanitization(_, p) => PathBuf::from(p),
        };

        // 3. 跟踪文件
        ctx.add_file(safe_path.clone(), size);

        Ok(safe_path)
    }

    /// 提取实现 - 使用默认逻辑
    async fn extract_impl(
        &self,
        source: &Path,
        target_dir: &Path,
        config: ExtractionConfig,
    ) -> Result<ExtractionSummary> {
        // 创建目标目录
        tokio::fs::create_dir_all(target_dir).await?;

        // 创建提取上下文
        let mut ctx = ExtractionContext::new(
            config,
            source.to_path_buf(),
            target_dir.to_path_buf(),
        );

        // 调用子类实现
        // 注意：具体实现需要处理特定格式的归档读取
        // 这里只是一个框架，子类需要override这个方法或者使用专门的extract_entries

        Ok(ctx.summary)
    }

    /// 创建目录的默认实现
    fn create_parent_dirs(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}
```

---

## 3. Handler 迁移示例

### 3.1 ZipHandler 迁移示例

#### Before (当前实现)

```rust
// log-analyzer/src-tauri/src/archive/zip_handler.rs

async fn extract_with_limits(
    &self,
    source: &Path,
    target_dir: &Path,
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
) -> Result<ExtractionSummary> {
    // 重复的安全检查逻辑 (约30行代码)
    if size > max_file_size
        || summary.total_size + size > max_total_size
        || summary.files_extracted + 1 > max_file_count
    {
        if size > max_file_size {
            warn!(...);
        } else if summary.total_size + size > max_total_size {
            warn!(...);
        } else {
            warn!(...);
        }
        continue;
    }
    // ... 提取逻辑
}
```

#### After (新设计)

```rust
// log-analyzer/src-tauri/src/archive/zip_handler.rs

use crate::archive::archive_handler::{ArchiveEntry, ArchiveHandler, ExtractionSummary};
use crate::archive::base::{ArchiveHandlerBase, ExtractionConfig, ExtractionContext};
use crate::error::{AppError, Result};
use crate::utils::path_security::{validate_and_sanitize_archive_path, PathValidationResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;
use zip::ZipArchive;

/// ZIP文件处理器 - 使用基类默认实现
pub struct ZipHandler {}

#[async_trait]
impl ArchiveHandler for ZipHandler {
    fn can_handle(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("zip"))
            .unwrap_or(false)
    }

    async fn extract(
        &self,
        source: &Path,
        target_dir: &Path,
        config: &ExtractionConfig,
    ) -> Result<ExtractionSummary> {
        fs::create_dir_all(target_dir).await?;

        let source_path = source.to_path_buf();
        let target_path = target_dir.to_path_buf();
        let config = config.clone();

        let summary = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&source_path)?;
            let mut archive = ZipArchive::new(file)
                .map_err(|e| AppError::archive_error(e.to_string(), None))?;
            
            let mut ctx = ExtractionContext::new(
                config,
                source_path.clone(),
                target_path.clone(),
            );

            for i in 0..archive.len() {
                let mut file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("Failed to read zip entry {}: {}", i, e);
                        continue;
                    }
                };

                let name = file.name().to_string();
                let size = file.size();

                if file.is_dir() {
                    let _ = std::fs::create_dir_all(&target_path.join(&name));
                } else {
                    // 使用基类提供的统一验证逻辑
                    match self.validate_and_track(&name, size, &mut ctx) {
                        Ok(safe_path) => {
                            let out_path = target_path.join(&safe_path);
                            self.create_parent_dirs(&out_path)?;
                            
                            match std::fs::File::create(&out_path) {
                                Ok(mut out_file) => {
                                    if let Err(e) = std::io::copy(&mut file, &mut out_file) {
                                        warn!("Failed to extract file {:?}: {}", out_path, e);
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to create file {:?}: {}", out_path, e);
                                }
                            }
                        }
                        Err(e) => {
                            // validate_and_track 已经记录了错误
                        }
                    }
                }
            }

            Ok::<ExtractionSummary, AppError>(ctx.summary)
        })
        .await
        .map_err(|e| AppError::archive_error(e.to_string(), None))??;

        Ok(summary)
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["zip"]
    }

    async fn list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        // ... 保持不变
    }

    async fn read_file(&self, path: &Path, file_name: &str) -> Result<String> {
        // ... 保持不变
    }
}
```

### 3.2 迁移对比

| 方面 | Before | After |
|------|--------|-------|
| 安全检查 | 每个handler重复约30行 | 基类统一实现，handler只需调用 |
| 参数数量 | 5个参数 | 1个配置对象 |
| 错误日志 | 不一致（有的warn，有的silent skip） | 统一日志输出 |
| 新增handler | 需要复制粘贴安全检查代码 | 继承基类即可 |

---

## 4. 实施计划

### 4.1 Phase 1: 基础设施

- [ ] 创建 `ExtractionConfig` 结构体
- [ ] 创建 `ExtractionContext` 结构体  
- [ ] 创建 `ExtractionError` 枚举
- [ ] 单元测试覆盖

### 4.2 Phase 2: Trait 定义

- [ ] 创建 `ArchiveHandlerBase` trait
- [ ] 实现默认的 `extract` 方法
- [ ] 更新 `ArchiveHandler` trait 添加 `extract` 方法
- [ ] 保持向后兼容

### 4.3 Phase 3: Handler 迁移

- [ ] 迁移 `ZipHandler`
- [ ] 迁移 `TarHandler`
- [ ] 迁移 `GzHandler`
- [ ] 迁移 `RarHandler`
- [ ] 迁移 `SevenZHandler`

### 4.4 Phase 4: 清理

- [ ] 更新 `ArchiveManager` 使用新 API
- [ ] 删除 `find_handler` 独立函数
- [ ] 运行完整测试
- [ ] 性能基准对比

---

## 5. 设计决策说明

### 5.1 为什么使用配置对象？

**问题**: `extract_with_limits` 有5个参数，容易传错顺序

**解决方案**: 使用 `ExtractionConfig` 配置对象

```rust
// Before
handler.extract_with_limits(source, target, 100_000_000, 1_000_000_000, 1000).await

// After  
let config = ExtractionConfig::with_limits(100_000_000, 1_000_000_000, 1000);
handler.extract(source, target, &config).await
```

### 5.2 为什么使用 trait 继承？

**问题**: 安全检查逻辑在每个 handler 中重复

**解决方案**: 使用 `ArchiveHandlerBase` 提供默认实现

- 子类可以 override 特定方法
- 默认实现消除了重复代码
- 保持 `ArchiveHandler` 简单，只定义接口

### 5.3 为什么使用 thiserror？

**问题**: 错误处理不一致，部分返回 String

**解决方案**: 使用 `thiserror` 定义结构化错误

- 编译时类型检查
- 丰富的错误上下文信息
- 与 `?` 操作符完美集成

### 5.4 向后兼容性

- 保留原有的 `extract_with_limits` 方法
- 新 `extract` 方法内部调用 `extract_with_limits`
- 现有代码无需修改即可运行

---

## 6. 预期收益

| 指标 | 改善 |
|------|------|
| 代码重复减少 | ~25% (约100行重复代码消除) |
| API 易用性 | 配置对象替代5个参数 |
| 维护性 | 安全检查逻辑集中在一处 |
| 错误一致性 | 统一使用 `thiserror` 错误类型 |

---

## 7. 附录

### 7.1 文件变更清单

| 文件 | 操作 |
|------|------|
| `archive/base.rs` | 新增 - 基类定义 |
| `archive/archive_handler.rs` | 修改 - 添加 `extract` 方法 |
| `archive/extraction_config.rs` | 新增 - 配置结构体 |
| `archive/extraction_context.rs` | 新增 - 上下文结构体 |
| `archive/extraction_error.rs` | 新增 - 错误枚举 |
| `archive/zip_handler.rs` | 修改 - 使用基类 |
| `archive/tar_handler.rs` | 修改 - 使用基类 |
| `archive/gz_handler.rs` | 修改 - 使用基类 |
| `archive/rar_handler.rs` | 修改 - 使用基类 |
| `archive/sevenz_handler.rs` | 修改 - 使用基类 |
| `archive/mod.rs` | 修改 - 导出新类型 |

### 7.2 依赖项

无需新增依赖，使用现有 `thiserror` crate。
