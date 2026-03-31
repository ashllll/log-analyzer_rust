<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# src (la-archive 源码)

## Purpose
la-archive crate 的源代码目录，实现压缩包处理的核心逻辑。

## Key Files

| File | Description |
|------|-------------|
| `lib.rs` | Crate 入口，模块导出 |
| `archive_handler.rs` | 归档处理器统一接口 |
| `extraction_engine.rs` | 解压引擎核心实现 |
| `extraction_orchestrator.rs` | 解压编排器 |
| `processor.rs` | 文件处理器主逻辑 |
| `public_api.rs` | 公共 API 定义 |
| `security_detector.rs` | 安全检测器 |
| `checkpoint_manager.rs` | 检查点管理器 |
| `path_manager.rs` | 路径管理器 |
| `zip_handler.rs` | ZIP 格式处理器 |
| `tar_handler.rs` | TAR 格式处理器 |
| `gz_handler.rs` | GZ 格式处理器 |
| `rar_handler.rs` | RAR 格式处理器 |
| `sevenz_handler.rs` | 7Z 格式处理器 |
| `extraction_context.rs` | 解压上下文 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `internal/` | 内部实现细节（文件类型过滤、元数据数据库） |

## For AI Agents

### Working In This Directory
- 压缩处理器统一实现 `ArchiveHandler` trait
- 解压过程支持检查点和恢复
- 安全检测防止 Zip Slip 攻击

### Common Patterns
- Handler trait 统一接口
- 流式解压处理大文件
- 递归解压支持嵌套压缩包

<!-- MANUAL: -->
