<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# stores (Zustand状态管理)

## Purpose
全局状态管理，使用 Zustand 实现，支持持久化和中间件。

## Key Files

| File | Description |
|------|-------------|
| `appStore.ts` | 应用级状态（主题、初始化状态） |
| `workspaceStore.ts` | 工作区管理 |
| `keywordStore.ts` | 关键词组管理（带persist持久化） |
| `taskStore.ts` | 任务状态（运行时状态，不持久化） |
| `types.ts` | Store类型定义 |

## For AI Agents

### Working In This Directory
- 使用 persist 中间件实现状态持久化
- 临时状态（loading/error）不持久化
- 状态更新使用 immer 或直接赋值

### Testing Requirements
- 使用 `create` 创建测试store实例
- 每个store有对应的单元测试

### Common Patterns
- 使用 selector 订阅特定状态片段
- 复杂更新使用 set 函数
- 派生状态使用计算属性

## Dependencies

### External
- **zustand** - 状态管理核心
- **zustand/middleware** - persist中间件
- **immer** - 不可变更新（可选）

<!-- MANUAL: -->
