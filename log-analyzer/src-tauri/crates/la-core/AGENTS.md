<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# la-core (核心库)

## Purpose
Workspace核心库，定义共享的数据模型、错误类型和工具函数。

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Crate配置 |
| `src/lib.rs` | 库入口 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `src/models/` | 共享数据模型 (see `src/models/AGENTS.md`) |
| `src/utils/` | 共享工具函数 (see `src/utils/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 只放真正跨crate共享的代码
- 避免循环依赖
- 保持轻量级

### Testing Requirements
- 所有公共API需要测试
- 文档测试验证示例代码

## Dependencies

### External
- **serde** - 序列化
- **thiserror** - 错误定义

<!-- MANUAL: -->
