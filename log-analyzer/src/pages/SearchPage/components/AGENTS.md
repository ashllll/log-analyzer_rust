<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# components (SearchPage专属组件)

## Purpose
SearchPage页面专用的业务组件，不与其它页面共享。

## Key Files

| File | Description |
|------|-------------|
| `SearchInput.tsx` | 搜索输入框 |
| `ResultList.tsx` | 结果列表 |
| `LogLine.tsx` | 单行日志渲染 |
| `FilterChips.tsx` | 过滤器标签 |

## For AI Agents

### Working In This Directory
- 组件仅用于SearchPage
- 如需复用，提升到 `src/components/`

### Testing Requirements
- 单元测试在 `__tests__/` 目录

### Common Patterns
- 使用 forwardRef 支持ref
- Props类型定义完整

<!-- MANUAL: -->
