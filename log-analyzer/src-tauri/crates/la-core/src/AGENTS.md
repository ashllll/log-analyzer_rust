<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# src (la-core 源码)

## Purpose
la-core crate 的源代码目录，为整个 workspace 提供共享的类型定义、错误类型和 trait。

## Key Files

| File | Description |
|------|-------------|
| `lib.rs` | Crate 入口，统一导出所有模块 |
| `error.rs` | 共享错误类型 `AppError`、`CommandError` |
| `traits.rs` | 共享 trait 定义 |
| `storage_types.rs` | 存储相关类型 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `models/` | 数据模型（配置、搜索、验证等） |
| `utils/` | 工具函数（路径安全、验证） |

## For AI Agents

### Working In This Directory
- 所有类型需实现 Serialize/Deserialize
- 错误类型使用 thiserror 定义
- 保持核心 crate 轻量，避免过多依赖

### Common Patterns
- newtype 模式增强类型安全
- 使用 builder 模式构建复杂对象
- 验证与解析分离

<!-- MANUAL: -->
