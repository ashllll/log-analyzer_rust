<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# archive (压缩包处理)

## Purpose
多格式压缩包解压模块，支持 ZIP/TAR/GZ/RAR/7Z 格式的递归解压。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 模块入口，ArchiveManager |
| `archive_handler.rs` | Handler trait定义 |
| `zip_handler.rs` | ZIP格式处理 |
| `tar_handler.rs` | TAR/TAR.GZ处理 |
| `rar_handler.rs` | RAR格式处理 |
| `gz_handler.rs` | GZ格式处理 |
| `sevenz_handler.rs` | 7Z格式处理 |

## For AI Agents

### Working In This Directory
- 使用策略模式，每个格式一个Handler
- 递归解压，自动处理嵌套压缩包
- 安全限制：文件大小/数量/总大小

### Testing Requirements
- 各格式处理器单元测试
- 边界条件测试（空文件、损坏文件）

### Common Patterns
- 实现 ArchiveHandler trait
- 使用 Builder 模式构建处理流程
- 错误使用 AppError::Archive 类型

## Dependencies

### External
- **zip** - ZIP处理
- **tar** - TAR处理
- **flate2** - GZIP压缩
- **unrar** - RAR处理
- **sevenz-rust** - 7Z处理

<!-- MANUAL: -->
