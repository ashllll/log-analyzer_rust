<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# components (UI组件)

## Purpose
可复用的React UI组件库，包含基础组件、业务组件和渲染器。

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `ui/` | 基础UI组件 (Button, Input, Modal等) (see `ui/AGENTS.md`) |
| `modals/` | 弹窗组件 (see `modals/AGENTS.md`) |
| `renderers/` | 日志渲染组件 (see `renderers/AGENTS.md`) |
| `search/` | 搜索相关组件 (see `search/AGENTS.md`) |
| `charts/` | 图表组件 (see `charts/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 每个组件单独文件，使用 index.ts 导出
- Props使用 interface 定义，添加 JSDoc 注释
- 样式使用 Tailwind CSS，避免硬编码颜色
- 支持主题切换

### Testing Requirements
- 组件测试放在 `__tests__/` 子目录
- 使用 React Testing Library
- 测试用户交互和渲染输出

### Common Patterns
- 使用 forwardRef 支持 ref 转发
- 复杂组件使用 compound component 模式
- 使用 clsx + tailwind-merge 处理条件样式

## Dependencies

### Internal
- `hooks/` - 复用自定义hooks
- `types/` - 组件Props类型

### External
- **clsx** - 条件类名
- **tailwind-merge** - Tailwind类合并
- **lucide-react** - 图标库

<!-- MANUAL: -->
