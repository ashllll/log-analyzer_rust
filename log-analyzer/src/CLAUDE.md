[根目录](../../CLAUDE.md) > **src (React 前端)**

# React 前端架构文档

> React 19 + TypeScript 5.8.3 + Tailwind CSS | 最后更新: 2026-03-31

## 模块职责

- **现代化 UI**: Tailwind CSS 响应式设计
- **高性能渲染**: `@tanstack/react-virtual` 虚拟滚动
- **状态管理**: Zustand (UI 状态) + React Query (服务端缓存)
- **实时通信**: Tauri IPC invoke + 事件监听
- **类型安全**: TypeScript 严格模式，Zod 运行时校验
- **国际化**: i18next (zh / en)

## 入口与启动

- `main.tsx` - 应用入口 (React 19 + StrictMode)
- `App.tsx` - 主应用组件
  - `QueryClientProvider` (React Query)
  - `MemoryRouter`
  - `AppStoreProvider` (Zustand Store 初始化与事件订阅)
  - `AppContent` (侧边栏导航 + 路由 + Toaster)

### 根组件结构

```tsx
<QueryClientProvider>
  <MemoryRouter>
    <AppStoreProvider>
      <AppContent />
    </AppStoreProvider>
  </MemoryRouter>
</QueryClientProvider>
```

## 状态管理层级

| 层级 | 技术 | 用途 |
|------|------|------|
| **全局 UI 状态** | Zustand | `appStore`, `workspaceStore`, `keywordStore`, `taskStore` |
| **服务端缓存** | React Query | 搜索结果、工作区列表、性能指标 |
| **页面状态** | useReducer / useState | SearchPage 查询构建状态 |
| **局部状态** | useState | 组件级 UI 状态 |

### Zustand Stores (`stores/`)

| Store | 文件 | 职责 | 持久化 |
|-------|------|------|--------|
| `appStore` | `appStore.ts` | 全局初始化状态、activeWorkspaceId、Toast 队列 | 否 |
| `workspaceStore` | `workspaceStore.ts` | 工作区列表、选中状态、刷新逻辑 | 否 |
| `keywordStore` | `keywordStore.ts` | 关键词组 CRUD、高亮配置 | localStorage (persist) |
| `taskStore` | `taskStore.ts` | 任务列表、进度追踪 | 否 |

### Store 初始化 Hooks

- `useConfigInitializer.ts` - 加载应用配置
- `useTauriEventListeners.ts` - 监听后端 Tauri 事件
- `useEventBusSubscriptions.ts` - 订阅前端 EventBus

## 对外接口

### Tauri IPC 通信

前端统一通过 `services/api.ts` 调用后端命令，内置 Zod 校验、空值过滤、超时控制。

```typescript
import { api } from '@/services/api';

// 搜索日志
const searchId = await api.searchLogs({
  query: 'error timeout',
  workspaceId: 'ws-123',
  maxResults: 1000
});

// 加载工作区
const workspace = await api.loadWorkspace('ws-123');
```

后端事件监听在 `App.tsx` 和 `useTauriEventListeners.ts` 中统一处理。

## 页面组件 (`pages/`)

| 页面 | 文件 | 核心功能 |
|------|------|----------|
| WorkspacesPage | `WorkspacesPage.tsx` | 工作区管理、导入文件/文件夹 |
| SearchPage | `SearchPage.tsx` | 全文搜索、高级过滤、虚拟滚动结果 |
| KeywordsPage | `KeywordsPage.tsx` | 关键词组管理、颜色高亮 |
| TasksPage | `TasksPage.tsx` | 后台任务列表、进度查看 |
| PerformancePage | `PerformancePage.tsx` | 性能指标展示 |
| SettingsPage | `SettingsPage.tsx` | 应用设置 |

`SearchPage` 包含子组件目录 `SearchPage/components/`。

## 核心组件 (`components/`)

- `components/ui/` - 基础 UI 组件 (Button, Input, NavItem, Toast 等)
- `components/modals/` - 模态框 (FilterPalette, KeywordModal)
- `components/renderers/` - 渲染器 (HybridLogRenderer 虚拟滚动日志行)
- `components/search/` - 搜索相关组件 (KeywordStatsPanel)
- `components/ErrorBoundary.tsx` - 全局错误边界 + 错误处理初始化

## 服务层 (`services/`)

- `api.ts` - **统一 API 层**。封装所有 Tauri invoke，含 Zod 校验、超时、错误处理
- `SearchQueryBuilder.ts` - 结构化查询构建器 (Fluent API)
- `errors.ts` / `errorService.ts` - 错误类型定义与 API 错误创建
- `queryStorage.ts` - 查询历史 localStorage 持久化

## Hooks (`hooks/`)

- `useWorkspaceSelection.ts` / `useWorkspaceList.ts` - 工作区操作
- `useToast.ts` - Toast 通知
- `useInfiniteSearch.ts` / `useSearchListeners.ts` - 搜索与结果监听
- `useKeywordManager.ts` - 关键词管理
- `useTaskManager.ts` - 任务追踪
- `useKeyboardShortcuts.ts` - 全局快捷键

## 类型定义 (`types/`)

- `search.ts` - 搜索查询、SearchTerm、QueryValidation 类型
- `common.ts` - Workspace, LogEntry, KeywordGroup 通用类型
- `api-responses.ts` - Zod Schema + API 响应类型
- `ui.ts` - UI 相关类型

## 事件通信模式

- **前端 → 后端**: `invoke('command_name', params)` (封装在 `services/api.ts`)
- **后端 → 前端**: `app_handle.emit("event-name", data)` → 前端 `listen("event-name")`
- **前端内部**: EventBus 单例 (`events/`)，带 Zod 验证 + 版本号去重

## 关键依赖

```json
{
  "react": "^19.1.0",
  "@tanstack/react-query": "^5.71.0",
  "@tanstack/react-virtual": "^3.13.12",
  "zustand": "^5.0.3",
  "zod": "^3.24.2",
  "react-router-dom": "^7.4.1",
  "react-hot-toast": "^2.5.2",
  "i18next": "^25.7.1"
}
```

## 测试

```bash
# 前端测试
npm test
npm test -- --coverage

# 测试覆盖较好的模块
- services/__tests__/SearchQueryBuilder.test.ts
- stores/__tests__/appStore.test.ts
- stores/__tests__/workspaceStore.test.ts
- stores/__tests__/taskStore.test.ts
```

## 常见问题

### 状态未持久化，刷新页面丢失？
只有 `keywordStore` 使用 `persist` 中间件写入 localStorage。工作区列表等数据来自后端，由 React Query 缓存。

### 添加新页面？
1. 在 `pages/` 创建组件
2. 在 `App.tsx` 的 `Routes` 和 `navItems` 中添加

### 前端与后端字段名？
严格统一使用 `snake_case`。

---

*详细架构规范请参见根目录 [CLAUDE.md](../../CLAUDE.md)*
