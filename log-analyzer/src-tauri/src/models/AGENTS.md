<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# models (数据模型)

## Purpose
应用状态和数据模型定义，包括AppState、配置模型、搜索模型等。

## Key Files

| File | Description |
|------|-------------|
| `state.rs` | AppState应用状态定义 |
| `search.rs` | 搜索相关模型 |
| `config.rs` | 配置模型 |
| `mod.rs` | 模块导出 |

## For AI Agents

### Working In This Directory
- 使用 derive 宏实现标准 trait
- 复杂类型使用嵌套 struct
- 状态字段使用 Arc<Mutex<>> 包装

### Testing Requirements
- 模型序列化/反序列化测试
- 状态默认值测试

### Common Patterns
- 使用 serde 实现序列化
- 使用 Default trait 定义默认值
- 大状态使用 parking_lot::Mutex

## Dependencies

### External
- **serde** - 序列化
- **parking_lot** - 高性能锁
- **tokio** - 异步类型

<!-- MANUAL: -->
