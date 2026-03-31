<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# la-search (搜索库)

## Purpose
搜索引擎库，提供全文搜索、索引管理和并发搜索功能。

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Crate配置 |
| `src/lib.rs` | 库入口 |

## For AI Agents

### Working In This Directory
- 基于Tantivy的搜索实现
- 支持虚拟搜索管理
- 并发搜索使用ReaderPool

### Testing Requirements
- 搜索正确性测试
- 性能基准测试

## Dependencies

### Internal
- `la-core` - 核心模型

### External
- **tantivy** - 全文搜索
- **tokio** - 异步

<!-- MANUAL: -->
