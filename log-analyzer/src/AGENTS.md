<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# src (前端源码)

## Purpose
React + TypeScript 前端应用，提供日志分析的用户界面。

## Key Files

| File | Description |
|------|-------------|
| `main.tsx` | 应用入口点 |
| `App.tsx` | 根组件 |
| `index.css` | 全局样式 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `components/` | 可复用UI组件 (see `components/AGENTS.md`) |
| `pages/` | 页面级组件 (see `pages/AGENTS.md`) |
| `hooks/` | 自定义React Hooks (see `hooks/AGENTS.md`) |
| `stores/` | Zustand状态管理 (see `stores/AGENTS.md`) |
| `services/` | API调用和工具服务 (see `services/AGENTS.md`) |
| `types/` | TypeScript类型定义 (see `types/AGENTS.md`) |
| `utils/` | 工具函数 (see `utils/AGENTS.md`) |
| `schemas/` | Zod验证Schema (see `schemas/AGENTS.md`) |
| `events/` | EventBus事件系统 (see `events/AGENTS.md`) |
| `i18n/` | 国际化配置 (see `i18n/AGENTS.md`) |
| `constants/` | 常量定义 (see `constants/AGENTS.md`) |
| `assets/` | 静态资源 (see `assets/AGENTS.md`) |
| `lib/` | 第三方库封装 (see `lib/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 使用函数式组件 + Hooks
- 样式使用 Tailwind CSS + clsx + tailwind-merge
- 所有UI文本走 i18n 国际化
- 严格 TypeScript，避免 any

### Testing Requirements
- 组件测试: `npm test -- ComponentName`
- 使用 React Testing Library
- 测试覆盖率目标 >80%

### Common Patterns
- Props类型使用 interface 定义
- 复杂状态使用 useReducer
- 服务端状态使用 React Query
- UI状态使用 Zustand

## Dependencies

### Internal
- `services/` - API调用
- `stores/` - 全局状态
- `types/` - 共享类型

### External
- **react** / **react-dom** - UI框架
- **@tauri-apps/api** - Tauri IPC
- **zustand** - 状态管理
- **@tanstack/react-query** - 数据获取
- **@tanstack/react-virtual** - 虚拟滚动
- **react-hot-toast** - 提示消息
- **zod** - 运行时验证

<!-- MANUAL: -->
