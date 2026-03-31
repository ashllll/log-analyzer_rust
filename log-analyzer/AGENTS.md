<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# log-analyzer

## Purpose
Tauri 桌面应用主目录，包含前端 React 代码和 Rust 后端代码。

## Key Files

| File | Description |
|------|-------------|
| `package.json` | 前端依赖和脚本 |
| `tsconfig.json` | TypeScript 配置 |
| `vite.config.ts` | Vite 构建工具配置 |
| `tailwind.config.js` | Tailwind CSS 主题配置 |
| `index.html` | 应用入口 HTML |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `src/` | React 前端源码 (see `src/AGENTS.md`) |
| `src-tauri/` | Rust Tauri 后端 (see `src-tauri/AGENTS.md`) |
| `public/` | 静态资源 |
| `docs/` | 前端相关文档 |

## For AI Agents

### Working In This Directory
- 前端代码在 `src/`，后端在 `src-tauri/`
- 运行 `npm run tauri dev` 启动开发环境
- 前后端通过 Tauri IPC 通信

### Testing Requirements
- 前端: `npm test`
- 类型检查: `npm run type-check`
- Rust: `cd src-tauri && cargo test`

### Common Patterns
- 前端状态管理: Zustand
- 后端错误处理: thiserror
- 前后端字段命名统一使用 snake_case

## Dependencies

### Internal
- `src-tauri/src/` - Rust 后端服务

### External
- **@tauri-apps/api** - 前端调用后端 API
- **@tanstack/react-query** - 服务端状态管理
- **@tanstack/react-virtual** - 虚拟滚动
- **zustand** - 客户端状态管理

<!-- MANUAL: -->
