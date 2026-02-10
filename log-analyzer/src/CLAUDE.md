[根目录](../../CLAUDE.md) > **src (React前端)**

# React 前端架构文档

> React 19 + TypeScript + Tailwind CSS | 版本: 0.0.76

## 模块职责

React 前端是用户交互的核心层，采用现代化的函数式组件 + Hooks 模式，提供：

- **现代化UI**: 基于 Tailwind CSS 的响应式设计
- **高性能渲染**: 虚拟滚动支持大量日志数据
- **结构化查询**: SearchQueryBuilder 流畅 API
- **实时通信**: Tauri IPC + 事件系统
- **国际化支持**: i18next 中英文切换
- **状态管理**: Context + Hooks 轻量级方案
- **类型安全**: TypeScript 严格模式

## 入口与启动

### 核心入口文件

**main.tsx** - 应用启动入口
- React 19 + StrictMode
- i18n 初始化
- AppProvider 根上下文

**App.tsx** - 主应用组件
- 侧边栏导航
- 页面路由管理
- 全局状态和 Toast 系统
- 工作区上下文集成

### 根组件结构
```tsx
<AppProvider>
  <AppContent>
    <Sidebar>           // 导航栏
      <NavItem />       // 工作区/搜索/关键词/任务
    </Sidebar>
    <MainContent>       // 主内容区
      <Header />        // 工作区信息
      <PageContent />   // 动态页面内容
    </MainContent>
    <ToastContainer />  // 全局消息提示
  </AppContent>
</AppProvider>
```

## 对外接口

### Tauri IPC 通信

#### 前端调用后端
```typescript
import { invoke } from '@tauri-apps/api/core';

// 搜索日志
await invoke('search_logs', {
  query: searchString,
  workspaceId: activeWorkspace?.id,
  maxResults: 1000,
  filters: searchFilters
});

// 导入文件/文件夹
await invoke('import_folder', {
  path: selectedPath,
  workspaceId: workspaceId
});
```

#### 后端事件监听
```typescript
import { listen } from '@tauri-apps/api/event';

// 监听搜索结果
await listen('search-results', (event) => {
  const results = event.payload as SearchResults;
  updateResults(results);
});

// 监听任务进度
await listen('task-progress', (event) => {
  const progress = event.payload as TaskProgress;
  updateProgress(progress);
});
```

### 插件使用
```typescript
// 文件对话框
import { save } from '@tauri-apps/plugin-dialog';
await save({
  filters: [{ name: 'CSV', extensions: ['csv'] }]
});

// 打开外部链接
import { open } from '@tauri-apps/plugin-opener';
await open('https://github.com/ashllll/log-analyzer');
```

## 核心组件 (components/)

### 1. UI 组件库 (components/ui/)

基础 UI 组件，基于 Tailwind CSS 构建。

| 组件 | 功能 | 特性 |
|-----|------|------|
| **Button** | 按钮组件 | 多种样式、加载状态、禁用 |
| **Input** | 输入框 | 验证、占位符、类型支持 |
| **Card** | 卡片容器 | 阴影、边框、响应式 |
| **NavItem** | 导航项 | 活跃状态、图标支持 |
| **ToastContainer** | 消息提示 | 自动消失、类型区分 |
| **Skeleton** | 加载骨架 | 模拟内容加载状态 |

### 2. 模态框 (components/modals/)

#### FilterPalette - 过滤器面板
- **功能**: 高级搜索过滤器
- **特性**:
  - 时间范围选择
  - 日志级别过滤
  - 文件模式匹配
  - 实时预览

#### KeywordModal - 关键词管理
- **功能**: 创建/编辑关键词组
- **特性**:
  - 多关键词配置
  - 颜色高亮设置
  - 正则表达式支持
  - 导入/导出配置

### 3. 渲染器 (components/renderers/)

#### HybridLogRenderer - 混合日志渲染器
- **核心功能**: 高性能日志渲染
- **特性**:
  - 虚拟滚动支持
  - 动态高度计算
  - 关键词高亮显示
  - 智能文本截断
  - 上下文片段展开

```typescript
interface LogRowProps {
  log: LogEntry;
  isActive: boolean;
  onClick: () => void;
  query: string;
  keywordGroups: KeywordGroup[];
  virtualStart: number;
  virtualKey: React.Key;
  measureRef: (node: Element | null) => void;
}
```

### 4. 搜索组件 (components/search/)

#### KeywordStatsPanel - 关键词统计面板
- **功能**: 搜索结果统计可视化
- **特性**:
  - 匹配数量统计
  - 占比进度条
  - 颜色编码区分
  - 交互式过滤

### 5. 错误处理 (components/ErrorBoundary.tsx)

#### 全局错误处理系统
- **功能**: 统一的错误捕获和显示机制
- **组件**:
  - **ErrorCard**: 错误信息卡片显示组件
  - **AppErrorBoundary**: 应用级错误边界（类组件）
  - **CompactErrorFallback**: 紧凑错误回退组件
  - **PageErrorBoundary**: 页面级错误边界
  - **PageErrorFallback**: 页面级错误回退组件（用于 react-error-boundary）
  - **initGlobalErrorHandlers**: 全局错误处理器初始化函数

**核心功能**:
- React Error Boundary 捕获组件树错误
- 全局未捕获错误处理（unhandledrejection）
- Promise rejection 处理
- 与错误服务集成（createApiError、ErrorCode）

**使用示例**:
```typescript
import { AppErrorBoundary, initGlobalErrorHandlers } from './components/ErrorBoundary';

// 初始化全局错误处理器
useEffect(() => {
  const cleanup = initGlobalErrorHandlers();
  return cleanup;
}, []);

// 使用应用级错误边界
<AppErrorBoundary>
  <YourComponent />
</AppErrorBoundary>

// 使用 react-error-boundary 的回退组件
import { ErrorBoundary } from 'react-error-boundary';
import { PageErrorFallback } from './components/ErrorBoundary';

<ErrorBoundary FallbackComponent={PageErrorFallback}>
  <YourComponent />
</ErrorBoundary>
```

**错误显示特性**:
- 错误代码显示（ErrorCode）
- 详细信息可展开查看
- 堆栈跟踪可展开查看
- 操作按钮（重试、返回首页、报告问题）
- 支持可重试错误判断（isRetryableError）

## 页面组件 (pages/)

### 1. SearchPage - 搜索页面
**核心功能**:
- 日志全文搜索
- 高级过滤器
- 虚拟滚动渲染
- 结果导出功能
- 关键词高亮

**技术实现**:
- `useVirtualizer` 高性能列表
- `useDeferredValue` 搜索优化
- React.memo 组件优化
- `useCallback` 事件优化

**代码片段**:
```typescript
const SearchPage: React.FC<SearchPageProps> = ({
  keywordGroups,
  addToast,
  searchInputRef,
  activeWorkspace
}) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<LogEntry[]>([]);
  const deferredQuery = useDeferredValue(query);

  // 虚拟滚动配置
  const rowVirtualizer = useVirtualizer({
    count: results.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 60,
    overscan: 10
  });

  // 搜索逻辑...
};
```

### 2. KeywordsPage - 关键词管理
- 创建/编辑关键词组
- 颜色高亮配置
- 批量导入/导出
- 预设模板

### 3. WorkspacesPage - 工作区管理
- 导入文件/文件夹
- 工作区列表展示
- 状态管理 (READY/PROCESSING/OFFLINE)
- 删除和刷新操作

### 4. TasksPage - 后台任务
- 任务列表展示
- 进度实时更新
- 任务历史记录
- 取消/重试操作

### 5. PerformancePage - 性能监控
- 搜索性能指标
- 缓存命中率
- 内存使用情况
- 系统资源监控

## 服务层 (services/)

### 1. SearchQueryBuilder - 查询构建器
**设计模式**: 流畅 API (Fluent API) 构建器模式

**核心功能**:
- 结构化查询构建
- 查询验证系统
- 查询持久化 (localStorage)
- 优化查询转换
- 导入/导出支持

**API 示例**:
```typescript
// 创建空查询
const builder = SearchQueryBuilder.create();

// 从字符串创建
const builder = SearchQueryBuilder.fromString('error | timeout');

// 添加条件
builder
  .addTerm('error', {
    source: 'preset',
    isRegex: true,
    priority: 10
  })
  .setGlobalOperator('OR');

// 验证查询
const validation = builder.validate();
if (!validation.isValid) {
  console.error(validation.issues);
}

// 生成查询字符串
const queryString = builder.toQueryString(); // "error | timeout"

// 导出/导入
const exported = builder.export();
const imported = SearchQueryBuilder.import(exported);
```

**测试覆盖**: 40+ 测试用例，完整功能测试

### 2. queryApi - 查询 API
- Tauri IPC 封装
- 错误处理
- 响应解析

### 3. queryStorage - 查询持久化
- localStorage 封装
- 版本兼容性
- 自动保存/恢复

## Hooks (hooks/)

### 1. useKeyboardShortcuts - 键盘快捷键
```typescript
// Cmd+K / Ctrl+K 聚焦搜索框
// Enter 执行搜索
// Esc 关闭详情面板
```

### 2. useKeywordManager - 关键词管理
- 关键词组 CRUD
- 本地持久化
- 导入/导出

### 3. useTaskManager - 任务管理
- 后台任务追踪
- 进度更新
- 状态管理

### 4. useWorkspaceOperations - 工作区操作
- 工作区 CRUD
- 文件导入
- 状态同步

## 类型定义 (types/)

### 核心类型

#### search.ts - 搜索相关类型
```typescript
export interface SearchTerm {
  id: string;
  value: string;
  operator: QueryOperator;  // AND | OR | NOT
  source: TermSource;       // user | preset
  presetGroupId?: string;
  isRegex: boolean;
  priority: number;
  enabled: boolean;
  caseSensitive: boolean;
}

export interface SearchQuery {
  id: string;
  terms: SearchTerm[];
  globalOperator: QueryOperator;
  filters?: SearchFilters;
  metadata: QueryMetadata;
}

export interface QueryValidation {
  isValid: boolean;
  issues: ValidationIssue[];
}
```

#### common.ts - 通用类型
```typescript
export interface Workspace {
  id: string;
  name: string;
  path: string;
  status: 'READY' | 'PROCESSING' | 'OFFLINE';
  createdAt: number;
  updatedAt: number;
  fileCount?: number;
  totalSize?: number;
}

export interface LogEntry {
  id: string;
  workspaceId: string;
  filePath: string;
  lineNumber: number;
  content: string;
  timestamp?: string;
  level?: string;
  matches: MatchDetail[];
}

export interface KeywordGroup {
  id: string;
  name: string;
  color: string;  // blue | green | orange | red | purple
  patterns: Array<{
    regex: string;
    description?: string;
  }>;
  enabled: boolean;
}
```

#### ui.ts - UI 相关类型
- Toast 类型
- 导航类型
- 表单类型

## 关键依赖 (package.json)

### 核心依赖
```json
{
  // Tauri API
  "@tauri-apps/api": "^2",
  "@tauri-apps/plugin-dialog": "^2.4.2",
  "@tauri-apps/plugin-opener": "^2",
  "@tauri-apps/plugin-shell": "^2.3.3",

  // React 生态
  "react": "^19.1.0",
  "react-dom": "^19.1.0",
  "react-i18next": "^16.4.0",

  // 性能优化
  "@tanstack/react-virtual": "^3.13.12",  // 虚拟滚动
  "framer-motion": "^12.23.24",           // 动画

  // 工具库
  "clsx": "^2.1.1",                       // 条件类名
  "tailwind-merge": "^3.4.0",             // Tailwind 合并
  "lucide-react": "^0.554.0",             // 图标
  "i18next": "^25.7.1"                    // 国际化
}
```

### 开发依赖
```json
{
  // 测试
  "@testing-library/react": "^16.3.0",
  "@testing-library/jest-dom": "^6.9.1",
  "@testing-library/user-event": "^14.6.1",
  "@types/jest": "^30.0.0",
  "jest": "^30.2.0",
  "ts-jest": "^29.4.6",

  // 代码质量
  "eslint": "^9.39.1",
  "typescript": "~5.8.3",
  "@typescript-eslint/eslint-plugin": "^8.48.0",
  "eslint-plugin-react": "^7.37.5",
  "eslint-plugin-react-hooks": "^7.0.1"
}
```

## 测试策略

### 当前状态
- **SearchQueryBuilder**: 完整测试覆盖 (40+ 测试用例)
- **其他组件**: 待完善测试

### 测试配置
```json
// jest.config.js
{
  testEnvironment: 'jsdom',
  setupFilesAfterEnv: ['<rootDir>/src/setupTests.ts'],
  moduleNameMapper: {
    '\\.(css|less|scss)$': 'identity-obj-proxy'
  }
}
```

### 测试示例
```typescript
describe('SearchQueryBuilder', () => {
  it('should create empty query', () => {
    const builder = SearchQueryBuilder.create();
    const query = builder.getQuery();

    expect(query.terms).toHaveLength(0);
    expect(query.globalOperator).toBe('AND');
  });

  it('should parse multiple keywords', () => {
    const builder = SearchQueryBuilder.fromString('error | timeout');
    const query = builder.getQuery();

    expect(query.terms).toHaveLength(2);
    expect(query.terms[0].value).toBe('error');
    expect(query.terms[1].value).toBe('timeout');
  });
});
```

### 运行测试
```bash
# 运行所有测试
npm test

# 监听模式
npm test -- --watch

# 代码覆盖率
npm test -- --coverage
```

## 性能优化

### 核心优化策略

#### 1. 虚拟滚动
- 使用 `@tanstack/react-virtual`
- 动态高度计算
- overscan 预渲染
- 内存使用优化

```typescript
const rowVirtualizer = useVirtualizer({
  count: results.length,
  getScrollElement: () => parentRef.current,
  estimateSize: () => 60,
  overscan: 10  // 预渲染10行
});
```

#### 2. 组件优化
- React.memo 避免不必要渲染
- useCallback 缓存事件处理
- useMemo 缓存计算结果
- useDeferredValue 延迟更新

#### 3. 状态优化
- Context 分片减少重渲染
- 局部状态优先
- 状态最小化原则

#### 4. 渲染优化
- 条件渲染避免空列表
- 列表 key 优化
- 避免深层嵌套

## 常见问题 (FAQ)

### Q: 如何优化大量日志渲染性能？
A:
1. 启用虚拟滚动 (`useVirtualizer`)
2. 使用 React.memo 包装行组件
3. 避免在渲染中执行复杂计算
4. 适当调整 overscan 值

### Q: 如何添加新的页面？
A:
1. 在 `pages/index.ts` 中导出
2. 在 `App.tsx` 中添加路由
3. 在侧边栏添加导航项
4. 实现响应式布局

### Q: 如何添加新的 Hook？
A:
1. 在 `hooks/index.ts` 中导出
2. 遵循命名规范 (useXxx)
3. 完整的类型定义
4. 添加文档注释

### Q: 如何处理异步状态？
A:
1. 使用 loading/error/success 三态
2. 错误边界捕获异常
3. Toast 提示用户
4. 自动重试机制

### Q: 如何处理应用中的错误？
A:
应用使用多层错误处理机制：
1. **全局错误处理器**: `initGlobalErrorHandlers()` 捕获未处理的 Promise rejection 和全局错误
2. **错误边界**: 使用 `AppErrorBoundary` 或 `PageErrorBoundary` 包裹组件树
3. **错误服务**: 使用 `createApiError()` 创建结构化错误，`isRetryableError()` 判断是否可重试
4. **用户反馈**: 通过 `ErrorCard` 显示友好的错误信息和恢复选项

```typescript
// 初始化全局错误处理（在 App.tsx 中）
useEffect(() => {
  const cleanup = initGlobalErrorHandlers();
  return cleanup;
}, []);

// 使用错误边界包裹应用
<ErrorBoundary FallbackComponent={PageErrorFallback}>
  <AppContent />
</ErrorBoundary>
```

### Q: 如何添加国际化？
A:
1. 在 `i18n/locales/` 添加语言文件
2. 使用 `useTranslation` Hook
3. 翻译键命名规范
4. 更新类型定义

## 相关文件清单

### 核心文件
- `main.tsx` - 应用入口
- `App.tsx` - 主应用组件
- `vite.config.ts` - Vite 配置

### 组件
- `components/ui/` - 基础 UI 组件
- `components/modals/` - 模态框组件
- `components/renderers/` - 渲染器组件
- `components/search/` - 搜索组件
- `components/ErrorBoundary.tsx` - 错误边界和全局错误处理
- `components/ErrorFallback.tsx` - 错误回退组件（兼容 react-error-boundary）

### 页面
- `pages/SearchPage.tsx` - 搜索页面
- `pages/KeywordsPage.tsx` - 关键词页面
- `pages/WorkspacesPage.tsx` - 工作区页面
- `pages/TasksPage.tsx` - 任务页面
- `pages/PerformancePage.tsx` - 性能页面

### 服务
- `services/SearchQueryBuilder.ts` - 查询构建器
- `services/queryApi.ts` - API 封装
- `services/queryStorage.ts` - 持久化

### Hooks
- `hooks/useKeyboardShortcuts.ts` - 快捷键
- `hooks/useKeywordManager.ts` - 关键词管理
- `hooks/useTaskManager.ts` - 任务管理
- `hooks/useWorkspaceOperations.ts` - 工作区操作

### 类型
- `types/search.ts` - 搜索类型
- `types/common.ts` - 通用类型
- `types/ui.ts` - UI 类型

### 配置
- `package.json` - 依赖配置
- `tsconfig.json` - TypeScript 配置
- `tailwind.config.js` - Tailwind 配置
- `vite.config.ts` - Vite 配置

### 测试
- `setupTests.ts` - 测试配置
- `services/__tests__/SearchQueryBuilder.test.ts` - 测试用例

---

## 变更记录 (Changelog)

### [2025-02-10] 错误处理和性能监控完善
- ✅ 完成全局错误处理系统（ErrorBoundary.tsx）
- ✅ 创建性能监控页面（PerformancePage.tsx）
- ✅ 在所有表单中添加验证（KeywordModal.tsx）
- ✅ 更新错误处理文档

### [2025-12-13] AI上下文初始化
- ✅ 完成前端架构分析
- ✅ 组件和页面梳理
- ✅ 服务层和 Hooks 整理
- ✅ 测试策略总结

### [历史版本]
- 详见根目录 CHANGELOG.md

---

*本文档由 AI 架构师自动生成，基于 React 前端代码分析*
