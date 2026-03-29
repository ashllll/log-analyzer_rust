# Log Analyzer 完整架构分析报告

> **版本**: 1.2.53
> **技术栈**: Tauri 2.0 + Rust 1.70+ + React 19.1.0 + TypeScript 5.8.3
> **分析日期**: 2026-03-29
> **分析方法**: 4 个专业代理并行分析（后端架构、前端架构、官方文档调研、开源项目参考）

---

## 目录

1. [整体架构概览](#1-整体架构概览)
2. [后端模块架构 (Rust)](#2-后端模块架构-rust)
3. [前端模块架构 (React)](#3-前端模块架构-react)
4. [架构问题清单](#4-架构问题清单)
5. [官方文档验证的优化方向](#5-官方文档验证的优化方向)
6. [开源项目参考与借鉴](#6-开源项目参考与借鉴)
7. [优化计划与实施路线图](#7-优化计划与实施路线图)

---

## 1. 整体架构概览

### 1.1 架构模式

```
┌─────────────────────────────────────────────────────────────────┐
│                    Log Analyzer Architecture                     │
├──────────────────────────┬──────────────────────────────────────┤
│  Frontend (React 19)     │  Backend (Rust)                      │
│  ┌────────────────────┐  │  ┌────────────────────────────────┐  │
│  │ Pages              │  │  │ commands/ (16 IPC 命令模块)     │  │
│  │ - SearchPage       │  │  └────────────────────────────────┘  │
│  │ - WorkspacesPage   │  │  ┌────────────────────────────────┐  │
│  │ - KeywordsPage     │◄─┼─►│ services/ (业务逻辑层)          │  │
│  │ - TasksPage        │  │  │ - QueryExecutor (三层架构)      │  │
│  │ - PerformancePage  │  │  │ - RegexEngine (Aho-Corasick)   │  │
│  │ - SettingsPage     │  │  │ - EventBus (事件总线)           │  │
│  └────────────────────┘  │  └────────────────────────────────┘  │
│  ┌────────────────────┐  │  ┌─────────────┬──────────────────┐  │
│  │ State Management   │  │  │ search_engine/│ storage/        │  │
│  │ - Zustand (4 Store)│  │  │ (Tantivy 0.22)│ (CAS + SQLite)  │  │
│  │ - React Query      │  │  └─────────────┴──────────────────┘  │
│  │ - useReducer       │  │  ┌────────────────────────────────┐  │
│  └────────────────────┘  │  │ archive/ (ZIP/RAR/GZ/TAR/7Z)   │  │
│  ┌────────────────────┐  │  ├────────────────────────────────┤  │
│  │ Services           │  │  │ task_manager/ (Actor Model)     │  │
│  │ - api.ts (Zod)     │  │  ├────────────────────────────────┤  │
│  │ - SearchQueryBuildr│  │  │ monitoring/ (Metrics + OTEL)    │  │
│  │ - nullSafeApi      │  │  ├────────────────────────────────┤  │
│  └────────────────────┘  │  │ domain/ + application/ (DDD)    │  │
│                          │  └────────────────────────────────┘  │
└──────────────────────────┴──────────────────────────────────────┘
```

### 1.2 数据流

```
用户操作 → React组件 → Zustand Store (UI状态)
                   → React Query → Tauri IPC invoke() → commands/
                                                                → services/ (业务逻辑)
                                                                     → search_engine/ (搜索)
                                                                     → storage/ (CAS+SQLite)
                                                                     → archive/ (解压)
                                                                → TaskManager (异步任务)
                                                                → EventBus → Tauri emit() → 前端监听
```

### 1.3 文件统计

| 模块 | Rust 文件数 | TypeScript 文件数 |
|------|------------|-------------------|
| commands/ | 16 | - |
| search_engine/ | 10+ | - |
| services/ | 15+ | - |
| storage/ | 8+ | - |
| archive/ | 42 | - |
| domain/application/ | 8 | - |
| 其他后端模块 | 20+ | - |
| components/ | - | 15+ |
| pages/ | - | 6+ |
| hooks/ | - | 17 |
| stores/ | - | 6 |
| services/ | - | 7 |
| **总计** | **~140** | **~70** |

---

## 2. 后端模块架构 (Rust)

### 2.1 main.rs / lib.rs — 应用入口

**职责**: 配置 Tauri 应用、注入全局状态、注册全部 IPC 命令、管理生命周期退出清理

**核心状态** (5 个独立 State):
| State | 职责 | 核心字段数 |
|-------|------|-----------|
| `AppState` | 全局共享状态 | 15+ 字段 |
| `WorkspaceState` | 工作区状态 | 少量 |
| `SearchState` | 搜索状态 | 少量 |
| `CacheState` | 缓存状态 | 少量 |
| `MetricsState` | 指标状态 | 少量 |

**关键问题**:
- `AppState` 直接持有 15+ 个 `Arc<Mutex<...>>` 字段，职责过重
- 退出清理在 `ExitRequested` 中创建新 tokio Runtime，机制脆弱

### 2.2 commands/ — IPC 命令层

**职责**: 提供前端调用的所有 RPC 命令接口

| 子模块 | 职责 |
|--------|------|
| `search.rs` | 日志搜索、缓存命中统计、MessagePack 二进制传输 |
| `async_search.rs` | 异步搜索（取消令牌支持） |
| `import.rs` | 文件夹导入、工作区创建、CAS 存储 |
| `workspace.rs` | 工作区 CRUD、状态查询 |
| `query.rs` | 结构化查询执行 |
| `watch.rs` | 文件监听启停 |
| `config.rs` | 配置读写（6 组配置） |
| `export.rs` | 搜索结果导出 |
| `virtual_tree.rs` | 虚拟文件树（按 hash 读取） |
| 其他 7 个 | 缓存/性能/验证/状态同步/错误上报/遗留检测 |

**关键问题**:
- 部分命令返回 `Result<T, String>`，部分返回 `Result<T, CommandError>`，不统一
- `import.rs` 使用 `#[allow(non_snake_case)]` 保持 camelCase，与 CLAUDE.md 规范矛盾

### 2.3 search_engine/ — 搜索引擎核心

**职责**: 基于 Tantivy 的全文搜索 + Aho-Corasick 多模式匹配 + 磁盘结果缓存

| 核心类型 | 说明 |
|---------|------|
| `SearchEngineManager` | 封装 Tantivy Index/Reader/Writer |
| `LogSchema` | 索引 Schema（content/timestamp/level/file_path/line_number） |
| `BooleanQueryProcessor` | 布尔查询解析 |
| `HighlightingEngine` | 搜索结果高亮 |
| `VirtualSearchManager` | 服务端虚拟分页 |
| `DiskResultStore` | 磁盘搜索结果缓存 |
| `StreamingIndexBuilder` | 流式索引构建（大数据集） |

**Schema 设计评价**: content 使用 `en_stem + WithFreqsAndPositions` 支持高亮；level/file_path 使用 `raw` 分词器避免不必要分词；timestamp/line_number 使用快速字段支持范围查询。设计合理。

### 2.4 services/ — 业务逻辑层

**职责**: 核心业务服务，提供模式匹配、查询执行、文件监听、事件总线等

| 核心类型 | 说明 |
|---------|------|
| `QueryExecutor` | 查询执行协调器（Validator → Planner → Matcher 三层） |
| `RegexEngine` / `AhoCorasickEngine` | Aho-Corasick + 标准 Regex 双引擎 |
| `QueryPlannerAdapter` | 查询计划器 |
| `EventBus` (services/) | 服务层事件总线 |
| `AppServices` / `AppServicesBuilder` | 依赖注入容器（**未被使用**） |
| `FileWatcher` | 文件系统监听 |

**关键 Trait** (`traits.rs`): `QueryValidation`, `QueryPlanning`, `ContentStorage`, `MetadataStorage`, `QueryExecutor`

### 2.5 storage/ — 存储层

**职责**: CAS 内容寻址存储 + SQLite 元数据管理

| 核心类型 | 说明 |
|---------|------|
| `ContentAddressableStorage` | SHA-256 内容寻址存储（基于 Git 对象模型） |
| `MetadataStore` | SQLite 元数据管理（sqlx 异步 + FTS5） |
| `StorageCoordinator` | Saga 补偿事务协调器 |
| `GarbageCollector` | 存储垃圾回收 |
| `CacheMonitor` | 缓存健康监控 |

### 2.6 archive/ — 压缩包处理

**职责**: ZIP/TAR/GZ/RAR/7Z 格式的递归解压，包含 Actor 系统、流式管线、断点续传等

42 个 `.rs` 文件，是最大的模块。包含：
- Actor 系统（Coordinator/Supervisor/Progress）
- 流式管线（BufferPool + Pipeline）
- 断点续传（CheckpointManager）
- 审计日志（AuditLogger）
- 安全检测（SecurityDetector + PathValidator）
- 10+ 个属性测试文件

### 2.7 其他后端模块

| 模块 | 职责 | 核心类型 |
|------|------|---------|
| `task_manager/` | Actor Model 异步任务管理 | TaskManager, ActorMessage(7变体) |
| `monitoring/` | 性能指标 + OpenTelemetry | MetricsCollector, AdvancedMetricsCollector |
| `domain/` | DDD 领域模型（**未真正接入**） | LogEntry(值对象版), DomainEventBus |
| `application/` | CQRS + 插件系统（**模拟实现**） | CommandHandler, PluginManager |
| `events/` | 全局事件系统 | EventBus(全局单例), AppEvent(18变体) |
| `state_sync/` | Tauri 事件状态同步 | StateSync |
| `utils/` | 通用工具 | CacheManager, CancellationManager, 路径验证 |
| `infrastructure/` | 配置管理 | AppConfig |

---

## 3. 前端模块架构 (React)

### 3.1 stores/ — Zustand 全局状态

| Store | 职责 | 中间件栈 |
|-------|------|---------|
| `appStore` | 全局 UI 状态、Toast、活跃工作区 | devtools + subscribeWithSelector + immer |
| `workspaceStore` | 工作区 CRUD | devtools + immer |
| `keywordStore` | 关键词组 CRUD + toggle | devtools + immer |
| `taskStore` | 任务管理 + 空值安全工具 | devtools + immer |

`AppStoreProvider.tsx` (287 行): 初始化配置加载、Tauri 事件桥接、EventBus 订阅。

### 3.2 services/ — API 封装层

| 文件 | 说明 |
|------|------|
| `api.ts` | 统一 API 类（class 单例 + Zod 验证），封装全部 Tauri 命令 |
| `nullSafeApi.ts` | 带超时和空值保护的 IPC 调用包装 |
| `queryApi.ts` | 结构化查询执行 API |
| `SearchQueryBuilder.ts` | Fluent API 查询构建器 |
| `errors.ts` | 结构化错误处理体系（ApiError + ErrorCode） |
| `fileApi.ts` / `queryStorage.ts` | 文件读取 / 查询 localStorage 持久化 |

### 3.3 hooks/ — 自定义 Hooks (17个)

| 类别 | Hooks |
|------|-------|
| 工作区 | `useWorkspaceOperations`(废弃), `useWorkspaceImport`, `useWorkspaceManagement`, `useWorkspaceSelection`, `useWorkspaceWatch`, `useWorkspaceList` |
| 搜索 | `useInfiniteSearch`, `useSearchListeners` |
| 配置 | `useConfig`, `useConfigManager` |
| 任务/关键词 | `useTaskManager`, `useKeywordManager` |
| 服务端数据 | `useServerQueries`, `usePerformanceQueries` |
| UI 工具 | `useToast`, `useToastManager`, `useFormValidation`, `useKeyboardShortcuts`, `useResourceManager` |

### 3.4 components/ — UI 组件

| 目录 | 组件 | 说明 |
|------|------|------|
| `ui/` | Button, Input, Card, FormField, NavItem | 基础组件库（多变体支持） |
| `modals/` | KeywordModal, FilterPalette, FileFilterSettings | 模态框 |
| `renderers/` | HybridLogRenderer | 日志高亮渲染（多关键词颜色 + 智能截断 + React.memo） |
| `search/` | KeywordStatsPanel | 搜索统计面板 |
| `charts/` | MetricsTimeSeriesChart, TimeRangeSelector | 性能图表 |
| 根级 | ErrorBoundary, EventManager, VirtualFileTree | 全局组件 |

### 3.5 pages/ — 页面组件

| 页面 | 复杂度 | 特点 |
|------|--------|------|
| `SearchPage/` | 高 | 页面内聚架构（components/ + hooks/ + types/ 子目录） |
| `PerformancePage` | 高 | 时间序列图表 + 自动刷新 |
| `SettingsPage` | 高 | 多标签配置（提取策略/缓存/搜索/任务管理器） |
| `WorkspacesPage` | 中 | 工作区网格 + CRUD |
| `KeywordsPage` | 低 | 关键词组列表 |
| `TasksPage` | 低 | 任务列表 + 进度 |

### 3.6 其他前端模块

| 模块 | 说明 |
|------|------|
| `events/EventBus.ts` | 前端事件总线（Zod 验证 + 幂等性 + 版本号去重 + LRU 缓存） |
| `types/` | common.ts（re-export + 混合类型）、search.ts、ui.ts、api-responses.ts（Zod Schema） |
| `constants/` | colors.ts（5 种颜色样式映射）、search.ts（搜索配置常量） |
| `i18n/` | i18next（en.json / zh.json），但大量组件硬编码中文 |
| `utils/` | logger、classNames（clsx + tailwind-merge）、CircularBuffer（遗留） |

---

## 4. 架构问题清单

### 4.1 后端问题（8 个）

| # | 严重度 | 问题 | 影响 |
|---|--------|------|------|
| B1 | **高** | 三套 EventBus 并存（events/ + services/ + domain/） | 维护混乱、命名冲突 |
| B2 | **高** | DDD 分层未落地（application/domain 使用模拟数据） | 死代码、误导开发者 |
| B3 | **高** | AppState 膨胀（15+ 字段）与 4 个辅助 State 边界模糊 | 职责不清 |
| B4 | **中** | 两个 LogEntry 类型（domain 值对象 vs models 原始 String） | 类型割裂 |
| B5 | **中** | 监控系统冗余（手写 Counter/Histogram vs prometheus crate） | 两套系统未整合 |
| B6 | **中** | 服务容器 AppServices 未被 main.rs 使用 | 死代码 |
| B7 | **中** | archive 模块 42 个文件，过度工程化 | 复杂度过高 |
| B8 | **低** | 退出清理创建新 tokio Runtime，机制脆弱 | 潜在 panic 风险 |

### 4.2 前端问题（14 个）

| # | 严重度 | 问题 | 影响 |
|---|--------|------|------|
| F1 | **高** | KeywordGroup 类型三处定义不一致（comment vs description） | 运行时 undefined |
| F2 | **高** | LogEntry Schema id 字段类型冲突（string vs number） | 验证失败 |
| F3 | **高** | AppStoreProvider 287 行初始化逻辑过于复杂 | 一处失败全局阻塞 |
| F4 | **中** | 双 API 层并存（api.ts vs nullSafeApi/queryApi） | 维护成本翻倍 |
| F5 | **中** | Toast 管理三套方案（appStore / useToast / useToastManager） | 风格不一致 |
| F6 | **中** | useWorkspaceOperations 废弃但 App.tsx 仍在使用 | 不必要的 5 Hook 实例化 |
| F7 | **中** | SearchPage 接收不必要的 props drilling | 与 Zustand 理念冲突 |
| F8 | **中** | 国际化不完整（SettingsPage 硬编码中文） | 中英文切换失效 |
| F9 | **低** | providers/QueryProvider.tsx 未被使用 | 遗留代码 |
| F10 | **低** | utils/CircularBuffer.ts 遗留代码 | 磁盘直写后废弃 |
| F11 | **低** | constants/colors.ts 使用 any 类型 | 类型安全丧失 |
| F12 | **低** | HybridLogRenderer 使用 indexOf 非 regex | 名称误导 |
| F13 | **低** | useWorkspaceList.refreshWorkspaces 是空操作 | 误导调用方 |
| F14 | **低** | useConfig 未使用 React Query | 与项目风格不一致 |

---

## 5. 官方文档验证的优化方向

### 5.1 Tauri 2.0 最佳实践

| 建议 | 官方文档依据 | 当前状态 | 优化方向 |
|------|------------|---------|---------|
| 锁策略区分 | Tokio 官方：跨 await 用 tokio::sync::RwLock | 全部用 parking_lot | AppState 跨 await 字段改用 tokio::sync::RwLock |
| 状态领域分离 | Tauri 官方推荐按领域拆分 State | 已拆分 5 个 State | 合规，但边界需澄清 |
| 退出清理 | Tauri on_window_event + 提前标记 | 创建新 Runtime | 改用 on_window_event 渐进关闭 |

### 5.2 TanStack Query 最佳实践

| 建议 | 当前状态 | 优化方向 |
|------|---------|---------|
| 添加 `placeholderData: keepPreviousData` | 搜索切换时结果闪烁 | 消除闪烁 |
| 差异化 staleTime/gcTime | 统一 5 分钟 | 按数据类型设置不同策略 |
| 预加载相邻页面 | 未实现 | 范围加载时预加载下一页 |

### 5.3 Zustand 最佳实践

| 建议 | 当前状态 | 优化方向 |
|------|---------|---------|
| keywordStore 添加 persist | 未持久化 | 保存用户自定义关键词配置 |
| 中间件顺序 | devtools(subscribeWithSelector(immer())) | 如需 persist: devtools(persist(subscribeWithSelector(immer()))) |
| 细粒度订阅 | 已配置 subscribeWithSelector | 确保消费组件使用选择器 |

### 5.4 Tantivy 最佳实践

| 建议 | 当前状态 | 优化方向 |
|------|---------|---------|
| 批量导入后 wait_merging_threads | 未调用 | 导入完成后等待 segment 合并 |
| 定期 garbage_collect_files | 未实现 | 控制磁盘空间 |
| 考虑 raw_content 子字段 | 仅 en_stem | 改善错误码/UUID 精确搜索 |

### 5.5 TanStack Virtual 最佳实践

| 建议 | 当前状态 | 优化方向 |
|------|---------|---------|
| 使用 rangeChanged 回调 | useEffect 依赖 | 简化范围加载逻辑 |
| overscan 调优 | 20（合理折中） | 保持当前值 |
| estimateSize 策略 | 固定 48px | 日志行高一致时无需改动 |

---

## 6. 开源项目参考与借鉴

### 6.1 最具借鉴价值的三个项目

| 项目 | 核心借鉴点 | 优先级 |
|------|-----------|--------|
| **klogg** (C++, 3k+ stars) | 行偏移索引 — O(1) 定位任意行，按需加载行内容 | 高 |
| **lnav** (C++, 7k+ stars) | 可插拔日志格式系统 — 用户自定义格式描述 | 中 |
| **angle-grinder** (Rust, 3k+ stars) | 管线式查询 — 搜索→过滤→聚合→统计 | 中 |

### 6.2 技术选型验证

本项目技术选型与业界最佳实践**高度一致**：

| 选型 | 业界验证 | 结论 |
|------|---------|------|
| Tauri 2.0 | Electron 替代最佳方案 | 正确 |
| Rust 后端 | 性能关键逻辑行业标准 | 正确 |
| Aho-Corasick | ripgrep 同款库，生产验证 | 正确 |
| Tantivy | Quickwit/ParadeDB 生产使用 | 正确 |
| Zustand | Tauri 社区推荐状态管理 | 正确 |
| @tanstack/react-virtual | 现代虚拟滚动首选 | 正确 |
| CAS 存储 | Git/Docker 同类方案 | 正确 |

---

## 7. 优化计划与实施路线图

### 阶段一：清理与统一（1-2 天）

| 任务 | 类型 | 影响 |
|------|------|------|
| 统一 EventBus：合并 events/ + services/event_bus.rs + domain/events.rs 为单一事件系统 | 删除死代码 | 消除 B1 |
| 删除 DDD 死代码：移除未接入的 domain/ 和 application/ 模块（或标记为 experimental） | 删除死代码 | 消除 B2, B4, B6 |
| 统一错误返回类型：所有 commands 统一使用 `CommandResult<T>` | 代码规范 | 消除 B2部分 |
| 删除前端遗留代码：CircularBuffer.ts, QueryProvider.tsx, 废弃的 useWorkspaceOperations | 删除死代码 | 消除 F9, F10, F6 |
| 统一 Toast 入口：选择 appStore.addToast 为唯一入口 | 统一模式 | 消除 F5 |

### 阶段二：类型与 API 统一（2-3 天）

| 任务 | 类型 | 影响 |
|------|------|------|
| 统一 KeywordGroup 类型：选择 `stores/types.ts` 为权威定义，其余 re-export | 类型安全 | 消除 F1 |
| 统一 LogEntry Schema：确认后端返回类型，修复 `id` 字段 string/number 不一致 | 类型安全 | 消除 F2 |
| 合并双 API 层：将 nullSafeApi/queryApi 的超时和安全逻辑整合到 api.ts | 统一模式 | 消除 F4 |
| 拆分 types/common.ts：按领域重新组织（FilterOptions → search, Metrics → performance） | 模块化 | 改善可维护性 |
| 统一命令参数命名：全部改用 snake_case，移除 `#[allow(non_snake_case)]` | 规范一致 | 消除 B2部分 |

### 阶段三：架构优化（3-5 天）

| 任务 | 类型 | 影响 |
|------|------|------|
| 拆分 AppState：将 15+ 字段按职责重组，明确与 WorkspaceState/SearchState 等的边界 | 架构优化 | 消除 B3 |
| 拆分 AppStoreProvider：按初始化职责拆分为多个独立 Provider | 架构优化 | 消除 F3 |
| 消除 Props Drilling：SearchPage 直接从 Store 读取数据 | 状态管理 | 消除 F7 |
| 改善退出清理：使用 on_window_event + 提前标记替代创建新 Runtime | 稳定性 | 消除 B8 |
| 统一监控系统：移除手写 Counter/Histogram，统一使用 prometheus crate（或反之） | 代码精简 | 消除 B5 |

### 阶段四：性能与体验提升（3-5 天）

| 任务 | 类型 | 收益 |
|------|------|------|
| 添加 keepPreviousData：useInfiniteSearch 使用 placeholderData | 用户体验 | 消除搜索闪烁 |
| 差异化缓存策略：搜索/工作区/指标使用不同 staleTime/gcTime | 性能 | 优化缓存命中率 |
| keywordStore persist：添加 Zustand persist 中间件 | 用户体验 | 保存用户配置 |
| Tantivy 导入优化：导入完成后 wait_merging_threads + garbage_collect_files | 性能 | 提升搜索速度 |
| AppState 锁策略：跨 await 字段改用 tokio::sync::RwLock | 稳定性 | 避免运行时阻塞 |

### 阶段五：功能增强（远期）

| 任务 | 来源 | 收益 |
|------|------|------|
| 行偏移索引 | klogg | 大文件打开速度提升 |
| 可插拔日志格式 | lnav | 支持自定义日志格式 |
| 管线式查询 | angle-grinder | 搜索→分析能力升级 |
| 国际化补全 | 内部需求 | 中英文完整切换 |
| Tantivy raw_content 子字段 | 官方文档 | UUID/错误码精确搜索 |

---

## 附录

### A. 后端模块依赖关系图

```
                          main.rs
                            │
          ┌─────────────────┼──────────────────┐
          │                 │                  │
      commands/          models/           error.rs
      (16 子模块)     (AppState/DTOs)    (AppError)
          │
    ┌─────┼──────┬─────────┼───────────┐
    │     │      │         │           │
services/ search_engine/ storage/   archive/
(QueryExec  (Tantivy)    (CAS+      (ZIP/TAR/
 EventBus)               SQLite)    GZ/RAR/7Z)
    │     │      │         │
    └─────┼──────┴─────────┘
          │
    ┌─────┼──────────────┐
    │     │              │
  domain/ events/    monitoring/
  (DDD)  (EventBus)  (Metrics)
          │
    application/ ──→ domain/
    infrastructure/ ──→ models/config
    utils/ ←── 被所有模块引用
    task_manager/ ←── 被 commands/ 引用
```

### B. 前端组件依赖关系图

```
App.tsx
  ├── stores/appStore.ts (useAppStore)
  ├── hooks/useWorkspaceOperations.ts (废弃，组合5个子Hook)
  ├── hooks/useKeywordManager.ts
  ├── stores/AppStoreProvider.tsx
  │   ├── events/EventBus.ts
  │   ├── hooks/useConfigManager.ts
  │   └── stores/* (全部4个store)
  └── components/ui/NavItem.tsx

SearchPage/
  ├── hooks/useInfiniteSearch.ts (React Query)
  ├── hooks/useSearchListeners.ts (Tauri Events)
  ├── hooks/useSearchState.ts (useReducer)
  ├── components/ (ActiveKeywords, SearchControls, SearchFilters)
  ├── renderers/HybridLogRenderer.tsx
  └── services/SearchQueryBuilder.ts

PerformancePage → hooks/usePerformanceQueries.ts (React Query)
WorkspacesPage → hooks/useWorkspaceOperations.ts (废弃Hook)
```

### C. 参考项目链接

- [lnav](https://github.com/tstack/lnav) - 终端日志导航器
- [angle-grinder](https://github.com/rcoh/angle-grinder) - Rust 管线式日志处理
- [klogg](https://github.com/variar/klogg) - Qt 桌面日志查看器
- [Tantivy](https://github.com/quickwit-oss/tantivy) - 全文搜索引擎库
- [Quickwit](https://github.com/quickwit-oss/quickwit) - 分布式搜索引擎
- [ParadeDB](https://github.com/paradedb/paradedb) - PostgreSQL 搜索扩展
- [Tauri 2.0 架构](https://v2.tauri.app/concept/architecture/)
- [TanStack Virtual](https://tanstack.com/virtual)
- [Zustand](https://github.com/pmndrs/zustand)
