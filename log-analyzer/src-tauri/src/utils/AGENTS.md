<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# utils (工具模块)

## Purpose
通用工具函数和辅助模块，被各个业务模块复用。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 模块导出 |
| `cache.rs` | TTL缓存实现 |
| `cache_manager.rs` | 缓存管理器 |
| `log_config.rs` | 日志配置管理 |
| `path.rs` | 路径处理 |
| `validation.rs` | 输入验证 |
| `retry.rs` | 重试机制 |
| `cleanup.rs` | 资源清理队列 |
| `command_validation.rs` | 命令参数验证 |

## For AI Agents

### Working In This Directory
- 工具函数尽量纯函数，无副作用
- 复杂工具使用 struct + impl
- 添加充分的单元测试

### Testing Requirements
- 工具函数测试覆盖率应高
- 测试边界条件和错误场景

### Common Patterns
- 使用 `#[derive(Default)]`
- 错误处理使用 `Result<T, E>`
- 并发使用 `parking_lot` 锁

## Dependencies

### External
- **parking_lot** - 高性能锁
- **moka** - 并发缓存
- **dashmap** - 并发HashMap

<!-- MANUAL: -->
