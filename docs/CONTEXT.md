# Log Analyzer — 项目上下文

> 领域术语表 + 架构速查 · 无实现细节

## 项目定位

Log Analyzer 是一款 **Tauri 桌面应用**，用于大规模日志文件的导入、搜索、分析和实时监控。Rust 后端负责性能敏感操作（导入、搜索、文件监听），React/TypeScript 前端提供交互界面。

## 技术栈

| 层 | 技术 | 角色 |
|----|------|------|
| 桌面框架 | Tauri 2.x | IPC 通信、窗口管理 |
| 后端 | Rust 1.88 | 日志处理引擎 |
| 前端 | React 19 + TypeScript 5.8 | UI |
| UI 体系 | Tailwind CSS 3 + CSS 变量双主题 | Apple 风格亮/暗主题、Material 毛玻璃 |
| 状态管理 | Zustand 5 + React Query 5 | 前端状态 / 服务端缓存 |
| IPC | Tauri Events (非 WebSocket) | 前后端事件推送 |

## 领域术语

| 术语 | 定义 |
|------|------|
| **Workspace** | 工作区，一个日志文件或文件夹的导入产物。包含 CAS 存储、元数据索引、搜索会话。 |
| **CAS** | Content Addressable Storage，按 SHA-256 哈希寻址的文件内容存储。去重 + 不可变。 |
| **MetadataStore** | 元数据索引，存储文件的虚拟路径、时间戳、行数等元信息。SQLite 实现。 |
| **TaskManager** | 异步任务调度器（Actor 模型，基于 tokio mpsc），管理导入、搜索等长任务的创建-更新-完成-取消生命周期。 |
| **TaskHandle** | 任务句柄，唯一标识一个异步任务。 |
| **SearchUseCase** | 搜索用例，编排搜索流程：构建计划 → 并行搜索 → 结果分页。 |
| **SearchBatch** | 搜索批处理器，按 BATCH_SIZE=256 聚合匹配结果，批量写入 DiskResultStore。 |
| **SearchSessionManager** | 搜索会话管理器（全局单例），管理每个搜索会话的 CancellationToken 与生命周期。 |
| **WorkspaceService** | 工作区服务组合 trait（SearchService + ImportService + WatchService），每个工作区持有独立的 CAS、MetadataStore、QueryEngine。导入完成时预创建，非懒加载。 |
| **MatchPlan** | 匹配计划 trait（`la-core/domain/search.rs`），消除 Any 向下转型与 side-map，编译期类型安全。 |
| **运行时提取策略（`la_core::ExtractionPolicy`）** | 单次归档解压实际执行所用的七字段限制与调优参数，由 `la-archive` 直接 re-export。 |
| **持久化提取策略（`la_core::models::ExtractionPolicy`）** | TOML 配置文档的嵌套模型，包含 extraction/security/paths/performance/audit/handlers；不是运行时七字段策略。 |
| **QueryPlanner** | 查询规划器，将用户查询字符串解析为执行计划（关键词/正则/逻辑组合），Moka Cache 两层缓存。 |
| **DiskResultStore** | 磁盘搜索结果缓存（NDJSON + 二进制偏移索引），持久化搜索结果到磁盘以支持分页获取。 |
| **EventBus** | 前端事件总线，前端（Zustand store 间通信，非 Tauri Events）。 |
| **TauriEventPublisher** | 后端事件发布器，通过 Tauri Events 向前端推送搜索进度/结果。 |
| **AppState** | 全局应用状态容器，由 4 个 typed registries 组成：`WorkspaceRegistry`（工作区服务表）、`SearchRegistry`（DiskResultStore + SearchSessionManager + Rayon ThreadPool）、`TaskRegistry`（TaskManager）、`SyncRegistry`（StateSync）。每个 registry 内部管理自己的锁（P8+P11 重构）。 |

## 架构分层 (Clean Architecture)

```
┌─────────────────────────────┐
│   commands/   (Tauri 命令)   │  ← 参数校验 + 委托给 use case
├─────────────────────────────┤
│   application/ (Use Cases)   │  ← 编排层：搜索/导入/监听/工作区
├─────────────────────────────┤
│   domain/      (契约定义)    │  ← 抽象层：Events/Storage/运行时策略
├─────────────────────────────┤
│   infrastructure/ (Adapters) │  ← 实现层：Storage/Events/TaskScheduler
│   services/    (引擎)        │  ← 查询引擎/正则引擎/文件监视器
├─────────────────────────────┤
│   models/      (状态模型)    │  ← AppState/配置
│   utils/       (工具)        │  ← 编码/验证/缓存/重试
├─────────────────────────────┤
│   crates/                    │
│   ├── la-core    (核心 trait)│
│   ├── la-storage (CAS/SQLite)│
│   ├── la-search  (查询引擎)  │
│   └── la-archive (归档解压)  │
└─────────────────────────────┘
```

## 关键设计决策

1. **Tauri Events 替代 WebSocket** — 桌面应用无需网络通信，Tauri 内置 IPC 更高效
2. **CAS 存储** — 日志文件内容去重，节省磁盘空间
3. **spawn_blocking 隔离** — 搜索/导入在独立线程池运行，不阻塞主事件循环
4. **UseCase 依赖倒置** — 被 application 用例消费的外部能力通过 domain trait 抽象；归档执行由 `la-archive` 自身公开 API 和运行时策略负责，不保留无调用者的中间适配器
5. **分页搜索结果** — 搜索结果按页缓存到磁盘（DiskResultStore），避免内存溢出
6. **ReDoS 防护** — 查询验证器检测正则表达式的指数级回溯模式
7. **Typed Registries（P11）** — AppState 折叠为 4 个注册表，锁粒度细化到每个 registry 内部
8. **MatchPlan trait（P7）** — 匹配计划对象化，消除 Any 向下转型与 side-map，编译期类型安全
9. **预创建 WorkspaceService（P3）** — 工作区导入完成时即创建服务实例，搜索时纯查找无懒加载

## Rust 包结构

```
log-analyzer/src-tauri/
├── src/
│   ├── main.rs          # Tauri 应用入口 + 命令注册
│   ├── application/     # UseCase + 服务组合（search/search_batch/search_session/
│   │                    #   watch/workspace_service/config/export/virtual_tree）
│   ├── commands/        # Tauri 命令参数校验 + 业务委托
│   ├── infrastructure/  # Adapter 实现（searcher/result_store/log_file_repo/event_publisher）
│   ├── services/        # 引擎（query_planner/regex_engine/search_filters/file_watcher）
│   ├── models/          # AppState（4 个 typed registries）
│   ├── task_manager/    # Actor 模型任务生命周期管理
│   ├── utils/           # 编码/验证/路径/重试/workspace_guard
│   └── state_sync/      # 前后端状态同步模型
└── crates/
    ├── la-core/         # 核心 trait + 领域模型 + 错误类型（含 MatchPlan）
    ├── la-storage/      # CAS 存储 + MetadataStore (SQLite)
    ├── la-search/       # 查询引擎（Aho-Corasick/Regex/Memchr）+ DiskResultStore
    └── la-archive/      # 归档解压（ZIP/TAR/GZ/7Z/RAR）
```

> P6/P7 重构：`ImportUseCase`/`WatchUseCase`/`query_executor.rs`/`search_executor.rs`/`async_resource_manager.rs` 已删除。导入与监听逻辑内联到 `WorkspaceServiceImpl`，搜索循环折叠回 `SearchUseCase`，批量逻辑提取到 `SearchBatch`。

## 前端目录结构

```
log-analyzer/src/
├── hooks/          # React Hooks (useBackendSync, useInfiniteSearch, useSearchOrchestrator, etc.)
├── services/       # API 客户端 + 错误处理 + 查询构造器
├── stores/         # Zustand stores (app/workspace/task/keyword)
├── pages/          # 页面组件 (Workspaces/Search/Keywords/Tasks/Settings)
├── components/
│   ├── ui/         # 设计系统原语 (Button/Input/Card/DialogSurface/PopoverSurface/Skeleton...)
│   ├── search/     # 搜索业务组件 (SearchControls/SearchResults/LogRow/LogDetailPanel...)
│   ├── renderers/  # 日志渲染器 (HybridLogRenderer — 高亮 + 智能截断)
│   └── modals/     # 模态框 (FileFilterSettings/FilterPalette/KeywordModal)
├── theme/          # 主题系统 (AppearanceProvider/AppearanceControl — 亮/暗/跟随系统三态)
├── config/         # 前端配置 (toastConfig 等)
├── events/         # 前端 EventBus
├── i18n/           # 国际化 (en/zh)
├── schemas/        # Zod 验证 schema
└── utils/          # 日志/搜索模式/错误处理
```

> 前端设计系统详见 [架构 / 前端设计系统](./architecture/frontend.md)：CSS 变量双主题、Apple Material 毛玻璃、Tailwind 集成、无障碍策略。

## CI/CD

- **GitHub Actions**: `.github/workflows/ci.yml` (Rust + Frontend + IPC 检查)
- **GitLab CI**: `.gitlab-ci.yml` (完整流水线，含安全扫描)
- **本地验证**: `scripts/check_ipc_consistency.cjs` + `cargo fmt/clippy/test`
