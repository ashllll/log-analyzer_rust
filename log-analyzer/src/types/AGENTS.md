<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# types (TypeScript类型)

## Purpose
全局TypeScript类型定义，供前端各模块共享。

## Key Files

| File | Description |
|------|-------------|
| `search.ts` | 搜索相关类型 |
| `workspace.ts` | 工作区类型 |
| `task.ts` | 任务类型 |
| `api.ts` | API类型 |

## For AI Agents

### Working In This Directory
- 类型与Rust后端保持一致
- 使用snake_case字段名
- 复杂类型使用interface
- 简单类型使用type alias

### Common Patterns
- 共享类型放这里
- 组件专属类型放组件文件内
- 使用Zod schema进行运行时验证

## Dependencies

### Internal
- 与Rust models保持一致

<!-- MANUAL: -->
