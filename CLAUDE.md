# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 语言设置

**重要**: 本项目使用中文作为主要交流语言。请：

- 所有回答默认使用中文
- 代码注释使用中文
- 文档内容使用中文
- 仅在引用英文原文或技术术语时使用英文
- 任何情况下都不允许使用简单方案实施修复，不确定的问题先阅读代码或者搜索后再规划与实施修改

***

> **项目**: log-analyzer_rust - 高性能桌面日志分析工具
> **技术栈**: Tauri 2.0 + Rust 1.70+ + React 19 + TypeScript 5.8.3

***

## 常用命令

```bash
# 开发（Tauri + Vite，固定端口 3000）
# 注意：如果 3000 端口被占用，启动会直接失败（不会漂移）
npm run tauri dev

# 单独启动前端开发服务器（端口 3000）
npm run dev

# TypeScript 类型检查
npm run type-check

# ESLint
npm run lint / npm run lint:fix

# 前端测试
npm test

# 构建生产版本
npm run tauri build

# Rust 测试（在 log-analyzer/src-tauri/ 下执行）
cargo test --workspace --all-features          # 全部测试（含 workspace crates）
cargo test --all-features pattern_matcher      # 单模块/模式测试
cargo test --all-features test_name            # 单个测试
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt -- --check

# 推送前验证（Git pre-push hook 自动执行）
# 注意：默认不自动运行 Rust 测试（脚本末尾会提示是否运行）
npm run validate:ci
```

***

## 核心架构

### 技术选型

| 层 | 技术 | 职责 |
|---|------|------|
| 前端 UI | React 19 + Tailwind CSS | 组件渲染 |
| 状态管理 | Zustand (UI) + React Query (服务端缓存) | 全局/缓存状态 |
| 虚拟滚动 | @tanstack/react-virtual | 大量日志行渲染 |
| IPC 通信 | Tauri invoke/emit | 前后端桥接 |
| 后端框架 | Tauri 2.0 + tokio | 命令处理、异步运行时 |
| 搜索引擎 | Aho-Corasick + Tantivy 0.22 | 多模式匹配 + 全文搜索 |
| 存储 | CAS (SHA-256) + SQLite (sqlx) | 内容寻址 + 元数据 |

### 后端模块 (Rust: `log-analyzer/src-tauri/src/`)

```
src/
├── commands/        # Tauri IPC 命令层 (16+ 个子模块)
│                   # 所有命令返回 CommandResult<T>，使用 CommandError 错误类型
├── search_engine/   # Tantivy 全文搜索 + Aho-Corasick 多模式匹配
│                   # 核心类型: SearchEngineManager, VirtualSearchManager, DiskResultStore
├── services/        # 业务逻辑: QueryExecutor (Validator→Planner→Executor 三层)
│                   # RegexEngine, FileWatcher, ReportCollector
├── storage/         # CAS 内容寻址存储 + SQLite 元数据 (MetadataStore)
│                   # StorageCoordinator (Saga 事务协调)
├── archive/         # ZIP/TAR/GZ/RAR/7Z 递归解压 (主要逻辑在 la-archive crate)
├── task_manager/    # Actor 模型异步任务管理 (有界 mpsc 通道 + 版本号幂等)
├── models/          # 数据模型 + AppState (parking_lot::Mutex)
├── state_sync/      # Tauri 事件状态同步
├── monitoring/      # 性能指标收集 (MetricsCollector)
└── utils/           # CacheManager, CancellationManager, 路径验证, 编码支持
```

### Cargo Workspace 结构

后端采用 **Workspace 架构**，核心业务拆分到 4 个 crate：

| Crate | 路径 | 职责 |
|-------|------|------|
| **la-core** | `crates/la-core/` | 共享错误类型 (`AppError`, `CommandError`)、数据模型、trait、工具函数 |
| **la-storage** | `crates/la-storage/` | CAS 存储 (`cas.rs`)、SQLite 元数据 (`metadata_store.rs`)、Saga 事务 (`coordinator.rs`)、GC |
| **la-search** | `crates/la-search/` | Tantivy 索引、搜索管理器、`ReaderPool`、并发搜索、高亮引擎、流式构建器 |
| **la-archive** | `crates/la-archive/` | 压缩包处理 (ZIP/TAR/GZ/RAR/7Z)、解压安全检测、递归深度限制 |

**注意**：主 crate `src-tauri` 通过 path 依赖引入 workspace crates。修改 crate 源码后，从 `src-tauri/` 运行 `cargo test --workspace` 可测试全部 crate。

**关键架构决策**:
- 事件系统：所有命令直接使用 `app_handle.emit()` 发送到前端，无中间 EventBus 层
- 锁策略：AppState 使用 `parking_lot::Mutex`，采用 "lock → clone → unlock → await" 模式，不跨 `.await` 持锁
- 搜索结果：通过 `DiskResultStore` 写入磁盘临时文件，前端按需分页读取（避免大量数据驻留内存）
- Tantivy 导入：批量导入后调用 `commit_and_wait_merge()` 等待 segment 合并完成

### 前端模块 (React: `log-analyzer/src/`)

```
src/
├── pages/           # 页面组件 (SearchPage 内聚: components/ + hooks/ + types/)
├── components/      # UI 组件 (ui/ 基础组件, modals/, renderers/, search/, charts/)
├── stores/          # Zustand Store (appStore, workspaceStore, keywordStore, taskStore)
│                   # + 初始化 Hooks (useConfigInitializer, useTauriEventListeners 等)
├── hooks/           # 业务 Hooks (useInfiniteSearch, useSearchListeners 等)
├── services/        # API 层: api.ts (统一入口, Zod 验证, 超时控制)
├── types/           # TypeScript 类型 + Zod Schema
├── i18n/            # i18next (zh.json / en.json)
└── events/          # EventBus (Zod 验证 + 幂等性 + 版本号去重)
```

**状态管理层级**:
- **Zustand**: UI 状态 (toasts, 工作区选择, 关键词组) — keywordStore 带 persist 中间件
- **React Query**: 服务端缓存 — 搜索结果 (staleTime 5min), 性能指标 (10s)
- **useReducer**: 搜索执行状态 (SearchPage 内部)
- **组件 state**: 局部 UI 状态

**事件通信模式**:
- 前端→后端: `invoke('command_name', params)` (Tauri IPC)
- 后端→前端: `app_handle.emit("event-name", data)` → 前端 `listen("event-name")`
- 前端内部: EventBus 单例 (Zod 验证 + 幂等去重)

***

## 前后端集成规范

> **关键**: Rust字段名 = JSON字段名 = TypeScript字段名 (统一使用 snake_case)

```rust
// Rust
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String,        // snake_case
}
```
```typescript
// TypeScript
interface TaskInfo {
  task_id: string;              // 与 Rust 完全一致
}
```

**CAS UNIQUE 约束**: 使用 `INSERT OR IGNORE + SELECT` 模式处理并发

***

## 编码规范

### 必须使用业内成熟方案（铁律）

| 需求 | 推荐方案 | 禁止方案 |
|------|---------|---------|
| 超时控制 | AbortController | 手写 setTimeout + flag |
| 状态管理 | Zustand / React Query | 自造 useState 管理 |
| 多模式匹配 | Aho-Corasick 算法库 | 逐行正则表达式 |
| 异步重试 | tokio-retry | 手写 loop + sleep |
| 表单验证 | Zod / Validator derive | 手写正则校验 |
| 全文搜索 | Tantivy | 手写倒排索引 |
| 错误处理 | thiserror / miette | String / Box\<dyn Error\> |

### Rust
- `cargo fmt`, `cargo clippy -- -D warnings`
- `?` 传播错误，生产代码 100% 消除 panic
- `parking_lot` 高性能锁，`DashMap` 并发哈希

### TypeScript/React
- 函数式组件 + Hooks
- Tailwind + `clsx` + `tailwind-merge`
- i18next 国际化，所有 UI 文案走字典
- 严格 TypeScript，避免 `any`

***

## 代码质量检查

推送前自动执行 (Husky pre-push hook):

| 检查项 | 命令 |
|--------|------|
| ESLint | `npm run lint` |
| TypeScript 类型 | `npm run type-check` |
| 前端测试 | `npm test` |
| 前端构建 | `npm run build` |
| Rust 格式 | `cargo fmt -- --check` |
| Rust Clippy | `cargo clippy --all-features -- -D warnings` |
| Rust 测试 | `cargo test --all-features --lib --bins` |

***

## 故障排查

### 搜索无结果
1. 检查工作区状态是否为 `READY`
2. 后端日志确认索引已加载
3. `SELECT COUNT(*) FROM files;`

### 任务一直"处理中"
- EventBus 版本号重复，幂等性跳过更新
- 确保任务事件版本号单调递增

### IPC 字段名不一致
- Rust `task_id` vs 前端 `taskId` → 统一为 `task_id`
- Option/null: Rust `None` → JSON `null`

***

## 性能基准

| 指标 | 数值 |
|------|------|
| 搜索吞吐量 | 10,000+ 次/秒 |
| 单关键词搜索 | <10ms |
| Tantivy 全文搜索 | <200ms |
| 空闲内存 | <100MB |
| CAS 去重节省空间 | 30%+ |

***

## 已知问题 (Known Issues)

> **2026-03-31 大规模修复完成**：此前代码审查发现的 36 个问题（4 CRITICAL + 10 HIGH + 12 MEDIUM + 10 LOW）已在当日集中修复。主要修复内容包括：
> - `cache_key` 哈希逻辑补全（避免缓存污染）
> - 无界通道替换为有界通道 `tokio::sync::mpsc::channel(1000)`（消除 OOM 风险）
> - 孤儿文件清理竞态条件修复（数据库事务保证原子性）
> - `ReaderPool.acquire()` 方法实现（支持并发搜索 timeout 获取）
> - 以及 10 个 MEDIUM、10 个 LOW 优先级的清理与优化
>
> 当前暂无已确认的 CRITICAL/HIGH 级别未修复问题。如需查看具体修复细节，请查阅 2026-03-31 附近的 git 历史。

### 当前注意事项

| 类别 | 说明 |
|------|------|
| **测试功能开关** | `Cargo.toml` 定义了 `test` feature（默认不含），用于测试时启用 mock 功能 |
| **压缩包嵌套深度** | `la-archive` 已实现递归深度限制，默认最深 7 层 |
| **SQLite 版本** | 使用 `sqlx 0.8` + SQLite FTS5，与旧文档中的 `sqlx 0.7` 不同 |

***

*最后更新: 2026-03-31*
