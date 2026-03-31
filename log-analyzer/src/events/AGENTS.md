<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# events (事件系统)

## Purpose
EventBus事件总线，用于前端内部模块通信。

## Key Files

| File | Description |
|------|-------------|
| `index.ts` | EventBus实现 |
| `types.ts` | 事件类型定义 |

## For AI Agents

### Working In This Directory
- 使用Zod验证事件数据
- 支持幂等性和版本号去重
- 事件名称使用常量定义

### Testing Requirements
- 测试事件发布/订阅
- 测试幂等性逻辑

### Common Patterns
- 单例EventBus导出
- 使用on/emit方法
- 返回unsubscribe函数

## Dependencies

### External
- **zod** - 事件数据验证

<!-- MANUAL: -->
