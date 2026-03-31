<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# SearchPage (搜索页面)

## Purpose
核心搜索功能页面，包含搜索输入、结果展示、过滤器等完整功能。

## Key Files

| File | Description |
|------|-------------|
| `index.tsx` | 页面主组件 |
| `SearchContainer.tsx` | 搜索容器组件 |
| `ResultsPanel.tsx` | 结果面板 |
| `FilterPanel.tsx` | 过滤器面板 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `components/` | 页面专属组件 (see `components/AGENTS.md`) |
| `hooks/` | 页面专属hooks (see `hooks/AGENTS.md`) |
| `types/` | 页面类型定义 (see `types/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 页面级状态使用 useReducer
- 子组件通过 props 接收数据和回调
- 复杂逻辑抽离到 hooks

### Testing Requirements
- 集成测试在 `__tests__/` 目录
- 测试用户交互流程

### Common Patterns
- 容器/展示组件分离
- 使用 React.memo 优化性能
- 虚拟滚动处理大量结果

## Dependencies

### Internal
- `components/` - 复用UI组件
- `hooks/` - 复用业务hooks
- `services/` - API调用

<!-- MANUAL: -->
