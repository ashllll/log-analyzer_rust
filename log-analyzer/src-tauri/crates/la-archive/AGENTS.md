<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# la-archive (压缩包库)

## Purpose
压缩包处理库，支持多格式解压和递归处理。

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Crate配置 |
| `src/lib.rs` | 库入口 |
| `src/lib_archive.rs` | 主要实现 |

## For AI Agents

### Working In This Directory
- 策略模式处理多格式
- 支持ZIP/TAR/GZ/RAR/7Z
- 递归解压嵌套压缩包

### Testing Requirements
- 各格式测试
- 安全限制测试

## Dependencies

### Internal
- `la-core` - 核心模型

### External
- **zip** / **tar** / **flate2** - 压缩格式
- **unrar** - RAR支持
- **sevenz-rust** - 7Z支持

<!-- MANUAL: -->
