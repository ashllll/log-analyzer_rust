# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language

All responses, code comments, and documentation should be in **Chinese (中文)** by default. Use English only for technical terms or quoting original text.

**铁律**: Never apply "simple" fixes without first reading the relevant code and confirming the approach. When unsure, search the codebase first, then plan and implement.

***

> **项目**: log-analyzer_rust — 高性能桌面日志分析工具
> **技术栈**: Tauri 2.0 + Rust (edition 2021) + React 19 + TypeScript 5

***

## 常用命令

所有命令从 `log-analyzer/` 目录执行（Rust 命令从 `log-analyzer/src-tauri/` 执行）：

```bash
# 开发（Tauri + Vite，固定端口 3000）
npm run tauri dev

# 单独启动前端开发服务器（端口 3000）
npm run dev

# TypeScript 类型检查
npm run type-check

# ESLint
npm run lint / npm run lint:fix

# 前端测试（Jest + jsdom，覆盖率阈值 ~16%）
npm test

# 构建生产版本
npm run tauri build

# Rust 测试（在 src-tauri/ 下执行）
cargo test --workspace --all-features          # 全部测试（含 workspace crates）
cargo test --all-features pattern_matcher      # 单模块测试
cargo test --all-features test_name            # 单个测试
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt -- --check

# 推送前验证（Husky pre-push hook 自动执行）
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
├── commands/        # Tauri IPC 命令层 (16 个子模块)
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
| **la-core** | `src-tauri/crates/la-core/` | 共享错误类型 (`AppError`, `CommandError`)、数据模型、trait、工具函数 |
| **la-storage** | `src-tauri/crates/la-storage/` | CAS 存储 (`cas.rs`)、SQLite 元数据 (`metadata_store.rs`)、Saga 事务 (`coordinator.rs`)、GC |
| **la-search** | `src-tauri/crates/la-search/` | Tantivy 索引、搜索管理器、`ReaderPool`、并发搜索、高亮引擎、流式构建器 |
| **la-archive** | `src-tauri/crates/la-archive/` | 压缩包处理 (ZIP/TAR/GZ/RAR/7Z)、解压安全检测、递归深度限制 |

**注意**：主 crate `src-tauri` 通过 path 依赖引入 workspace crates。修改 crate 源码后，从 `src-tauri/` 运行 `cargo test --workspace` 可测试全部 crate。

### 前端模块 (React: `log-analyzer/src/`)

```
src/
├── pages/           # 页面组件 (SearchPage 内聚: components/ + hooks/ + types/ + utils/)
├── components/      # UI 组件 (ui/ 基础组件, modals/, renderers/, search/, charts/)
├── stores/          # Zustand Store (appStore, workspaceStore, keywordStore, taskStore)
│                   # + 初始化 Hooks (useConfigInitializer, useTauriEventListeners 等)
├── hooks/           # 业务 Hooks (useInfiniteSearch, useSearchListeners 等)
├── services/        # API 层: api.ts (统一入口, Zod 验证, 超时控制)
├── types/           # TypeScript 类型 + Zod Schema
├── i18n/            # i18next (zh.json / en.json)
└── events/          # EventBus (Zod 验证 + 幂等性 + 版本号去重)
```

### 关键架构决策

- **事件系统**：所有命令直接使用 `app_handle.emit()` 发送到前端，无中间 EventBus 层
- **锁策略**：AppState 使用 `parking_lot::Mutex`，采用 "lock → clone → unlock → await" 模式，不跨 `.await` 持锁
- **搜索结果**：通过 `DiskResultStore` 写入磁盘临时文件，前端按需分页读取（避免大量数据驻留内存）
- **Tantivy 导入**：批量导入后调用 `commit_and_wait_merge()` 等待 segment 合并完成
- **RAR 支持**：通过 `rar-support` feature 控制（默认启用），可通过 `check_rar_support` 命令查询
- **HMR 已禁用**：Vite 配置 `hmr: false`，避免 WebSocket 权限问题

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

### 必须使用业内成熟方案

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
- `.clippy.toml` 配置了 `unwrap-used = "warn"`, `expect-used = "warn"` — 优先使用 `?` 传播错误
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

*最后更新: 2026-04-10*
