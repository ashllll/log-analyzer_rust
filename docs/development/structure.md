# 项目结构

```text
log-analyzer_rust/
├── .github/
│   ├── actions/setup-tauri-linux/   # Linux 原生依赖复用 action
│   └── workflows/                   # CI、发布、覆盖率、性能与文档 Pages
├── docs/                            # VitePress 文档源与长期维护文档
├── log-analyzer/
│   ├── src/                         # React 19 + TypeScript 前端
│   ├── src-tauri/
│   │   ├── src/                     # Tauri 应用后端
│   │   └── crates/                  # la-core/storage/search/archive
│   ├── package.json
│   └── vite.config.ts
├── scripts/                         # CI、IPC 与发布校验脚本
├── package.json                     # 文档站工具与 scripts
└── README.md
```

## 前端结构

| 目录 | 关注点 |
| --- | --- |
| `pages/` | Workspaces、Search、Keywords、Tasks、Settings 等页面 |
| `components/` | 应用布局、错误边界、UI 与搜索组件 |
| `hooks/` | 工作区、搜索、配置、任务和 Tauri 生命周期组合 |
| `services/` | Tauri API、错误映射、查询与配置持久化规则 |
| `stores/` | Zustand 状态容器 |
| `events/` | EventBus 与 Tauri event projection |
| `schemas/` | Zod 边界验证 |

## Rust 后端结构

| 目录 | 关注点 |
| --- | --- |
| `commands/` | Tauri IPC 参数校验与委托 |
| `application/` | 搜索、导入、监听、配置、导出等 use cases |
| `infrastructure/` | domain traits 的文件系统 / 存储 / 事件适配器 |
| `services/` | 查询规划、正则、过滤与文件监听引擎 |
| `models/` | `AppState` 和 typed registries |
| `utils/` | 路径、编码、缓存、重试、取消与资源工具 |
| `state_sync/` | 前后端状态同步模型 |
| `task_manager/` | 异步任务生命周期 |

## 依赖方向

application 依赖 `la-core` 中的 domain traits；infrastructure 实现这些 traits。Tauri commands 只作为外部接口适配器。新增功能时优先保持这个方向，避免让 `la-core` 依赖 Tauri 或具体 SQLite / 文件系统类型。

## 从哪里开始改

- 新页面交互：从 `src/pages/` 和相关 hook 开始。
- 新 IPC 请求：先定义输入 / 输出边界，再补 command 与 application use case。
- 新存储能力：在 `la-core` 定义必要接口，由 `la-storage` 或 infrastructure 实现。
- 新搜索算法：优先放在 `la-search`，通过 `LogSearcher` 接入应用层。
- 新归档格式或安全规则：在 `la-archive` 内聚实现。

