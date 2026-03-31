<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# internal (内部模块)

## Purpose
la-archive crate 的内部实现细节，不对外暴露的辅助模块。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 内部模块入口 |
| `file_type_filter.rs` | 文件类型过滤逻辑（6.5KB） |
| `metadata_db.rs` | 元数据数据库操作（5.3KB） |

## For AI Agents

### Working In This Directory
- 内部模块不暴露于 public API
- 供 crate 内部其他模块使用
- 实现细节可随版本变化

### Common Patterns
- 内部辅助函数
- 实现细节封装
- 性能优化专用代码

<!-- MANUAL: -->
