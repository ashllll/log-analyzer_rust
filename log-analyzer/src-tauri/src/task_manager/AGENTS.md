<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# task_manager (任务管理)

## Purpose
Actor模型实现的异步任务管理，支持任务创建、取消、进度追踪。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 模块入口，TaskManager定义 |
| `actor.rs` | Actor消息处理 |
| `messages.rs` | 消息类型定义 |
| `types.rs` | 任务类型定义 |
| `error.rs` | 任务错误类型 |

## For AI Agents

### Working In This Directory
- 使用 mpsc 通道实现 Actor 通信
- 任务使用版本号实现幂等性
- 使用 CancellationToken 支持取消

### Testing Requirements
- 测试 Actor 消息处理逻辑
- 测试任务取消场景

### Common Patterns
- Actor 模式处理并发
- 消息枚举定义所有操作
- 状态变更发送事件通知

## Dependencies

### Internal
- `models/` - 任务模型

### External
- **tokio** - 异步运行时和通道
- **tokio-util** - CancellationToken

<!-- MANUAL: -->
