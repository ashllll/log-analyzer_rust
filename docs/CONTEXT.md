# Log Analyzer — 项目上下文

> 领域术语表 + 架构速查 · 无实现细节

## 项目定位

Log Analyzer 是一款 **Tauri 桌面应用**，用于大规模日志文件的导入、搜索、分析和实时监控。Rust 后端负责性能敏感操作（导入、搜索、文件监听），React/TypeScript 前端提供交互界面。

## 技术栈

| 层 | 技术 | 角色 |
|----|------|------|
| 桌面框架 | Tauri 2.x | IPC 通信、窗口管理 |
| 后端 | Rust 1.85 | 日志处理引擎 |
| 前端 | React + TypeScript | UI |
| 状态管理 | Zustand + React Query | 前端状态 / 服务端缓存 |
| IPC | Tauri Events (非 WebSocket) | 前后端事件推送 |

## 领域术语

| 术语 | 定义 |
|------|------|
| **Workspace** | 工作区，一个日志文件或文件夹的导入产物。包含 CAS 存储、元数据索引、搜索会话。 |
| **CAS** | Content Addressable Storage，按 SHA-256 哈希寻址的文件内容存储。去重 + 不可变。 |
| **MetadataStore** | 元数据索引，存储文件的虚拟路径、时间戳、行数等元信息。SQLite 实现。 |
| **TaskManager** | 异步任务调度器，管理导入、搜索等长任务的创建-更新-完成-取消生命周期。 |
| **TaskHandle** | 任务句柄，唯一标识一个异步任务。 |
| **SearchUseCase** | 搜索用例，编排搜索流程：构建计划 → 并行搜索 → 结果分页。 |
| **ImportUseCase** | 导入用例，编排导入流程：解压归档 → 文件哈希 → CAS 存储 → 元数据索引。 |
| **WatchUseCase** | 文件监听用例，监控工作区目录变化并增量索引新日志行。 |
| **ArchiveExtractor** | 归档解压器 trait，支持 ZIP/TAR/GZ/7Z/RAR 格式。 |
| **QueryPlanner** | 查询规划器，将用户查询字符串解析为执行计划（关键词/正则/逻辑组合）。 |
| **DiskResultStore** | 磁盘搜索结果缓存，持久化搜索结果到磁盘以支持分页获取。 |
| **EventBus** | 前端事件总线，前端（Zustand store 间通信，非 Tauri Events）。 |
| **TauriEventPublisher** | 后端事件发布器，通过 Tauri Events 向前端推送搜索进度/结果。 |
| **AppState** | 全局应用状态容器，持有 TaskManager、CAS、MetadataStore 等共享资源。 |

## 架构分层 (Clean Architecture)

```
┌─────────────────────────────┐
│   interfaces/  (Tauri 命令)  │  ← 入口层：#[tauri::command]
│   commands/   (业务委托)     │  ← 参数校验 + 委托给 use case
├─────────────────────────────┤
│   application/ (Use Cases)   │  ← 编排层：搜索/导入/监听/工作区
├─────────────────────────────┤
│   domain/      (Trait 定义)  │  ← 抽象层：Events/Storage/Extractor
├─────────────────────────────┤
│   infrastructure/ (Adapters) │  ← 实现层：ArchiveExtractor/TaskScheduler
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
4. **Domain Trait 模式** — 所有外部依赖通过 trait 抽象，UseCase 无具体实现依赖
5. **分页搜索结果** — 搜索结果按页缓存到磁盘（DiskResultStore），避免内存溢出
6. **ReDoS 防护** — 查询验证器检测正则表达式的指数级回溯模式

## Rust 包结构

```
log-analyzer/src-tauri/
├── src/
│   ├── main.rs          # Tauri 应用入口 + 命令注册
│   ├── lib.rs           # 库根
│   ├── application/     # UseCase（search/import/watch/workspace/config）
│   ├── commands/        # Tauri 命令参数校验 + 业务委托
│   ├── interfaces/      # #[tauri::command] 定义层
│   ├── infrastructure/  # Adapter 实现（archive_extractor/task_scheduler）
│   ├── services/        # 引擎（query_planner/query_executor/regex_engine/file_watcher）
│   ├── models/          # AppState
│   ├── utils/           # 编码/验证/缓存/取消
│   └── state_sync/      # 状态同步属性测试
└── crates/
    ├── la-core/         # 核心 trait + 领域模型 + 错误类型
    ├── la-storage/      # CAS 存储 + MetadataStore (SQLite)
    ├── la-search/       # 查询引擎（Aho-Corasick/Regex/Memchr）
    └── la-archive/      # 归档解压（ZIP/TAR/GZ/7Z/RAR）
```

## 前端目录结构

```
log-analyzer/src/
├── hooks/          # React Hooks (useBackendSync, useInfiniteSearch, etc.)
├── services/       # API 客户端 + 错误处理 + 查询构造器
├── stores/         # Zustand stores (app/workspace/task/keyword)
├── pages/          # 页面组件 (SearchPage)
├── components/     # 共享组件 (renderers, modals)
├── events/         # 前端 EventBus
├── schemas/        # Zod 验证 schema
└── utils/          # 日志/搜索模式/错误处理
```

## CI/CD

- **GitHub Actions**: `.github/workflows/ci.yml` (Rust + Frontend + IPC 检查)
- **GitLab CI**: `.gitlab-ci.yml` (完整流水线，含安全扫描)
- **本地验证**: `scripts/check_ipc_consistency.cjs` + `cargo fmt/clippy/test`
