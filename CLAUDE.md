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
# 开发（Tauri + Vite HMR）
npm run tauri dev

# TypeScript 类型检查
npm run type-check

# ESLint
npm run lint / npm run lint:fix

# 前端测试
npm test

# 构建生产版本
npm run tauri build

# Rust 测试（在 log-analyzer/src-tauri/ 下执行）
cargo test --all-features          # 全部测试
cargo test pattern_matcher         # 单模块测试
cargo test --lib -- tests::test_name  # 单个测试
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt -- --check

# 推送前验证（Git pre-push hook 自动执行）
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
├── archive/         # ZIP/TAR/GZ/RAR/7Z 递归解压 (Actor 模型 + 流式管线)
├── task_manager/    # Actor 模型异步任务管理 (mpsc 通道 + 版本号幂等)
├── models/          # 数据模型 + AppState (15+ 字段, parking_lot::Mutex)
├── state_sync/      # Tauri 事件状态同步
├── monitoring/      # 监控模块 (当前为空壳)
└── utils/           # CacheManager, CancellationManager, 路径验证, 编码支持
```

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

> 基于全面代码审查确认的36个真实问题（截至2026-03-31）
> 其中 4个 CRITICAL、10个 HIGH、12个 MEDIUM、10个 LOW

### 🔴 CRITICAL (需立即修复)

| 问题 | 位置 | 描述 | 风险 |
|------|------|------|------|
| **cache_key哈希不完整** | `services/query_executor.rs:61-77` | `generate_cache_key()`只哈希了`value/is_regex/case_sensitive/enabled`，遗漏`term.id/operator/priority`和`query.filters/id` | 缓存污染，查询返回错误结果 |
| **文档与实现不一致** | `models/state.rs:21-35` vs `67-81` | 注释声明使用`tokio::sync::RwLock`实现写并发，实际代码使用`std::sync::Mutex` | 并发能力下降，单写锁阻塞所有读取 |
| **无界通道内存泄漏** | `task_manager/mod.rs:551` | 使用`mpsc::unbounded_channel()`接收任务事件，无流量控制 | 高负载下内存无限增长直至OOM |
| **孤儿文件清理竞态** | `crates/la-storage/src/coordinator.rs:190-231` | `cleanup_orphan_files()`check-then-act模式，多线程下可能误删新建文件 | 数据丢失风险 |

### 🟠 HIGH (建议尽快修复)

| 问题 | 位置 | 描述 | 风险 |
|------|------|------|------|
| **ReaderPool未使用** | `crates/la-search/src/concurrent_search.rs:75-144` | `ReaderPool`结构体创建后`_readers`字段从未被访问，无`get_reader()`方法 | 设计缺陷，资源浪费 |
| **block_on阻塞运行时** | `services/query_executor.rs:112-119` | `tokio::task::block_on()`在async上下文中同步等待，持有锁期间阻塞线程 | 线程池耗尽，性能下降 |
| **check-then-rename竞态** | `crates/la-storage/src/cas.rs:1265-1310` | `store_file_zero_copy()`先检查再重命名，非原子操作 | 并发导入时数据损坏 |
| **错误信息不一致** | `storage/mod.rs` vs `errors.rs` | 存储层错误与全局错误类型不统一，转换时信息丢失 | 调试困难 |
| **TODO未处理** | `crates/la-search/src/index.rs:156-160` | `commit_and_wait_merge()`有硬编码超时和未处理的错误分支 | 索引提交可能死锁或静默失败 |
| **文档同步滞后** | `services/CLAUDE.md` | 模糊搜索模块已删除但文档仍引用`fuzzy_matcher.rs` | 维护成本增加 |
| **unsafe代码无注释** | `crates/la-search/src/pattern_matcher.rs` | 使用`unsafe`进行字节操作但无安全注释说明 | 潜在UB风险 |
| **递归解压无深度限制** | `archive/*.rs` | 压缩包递归解压未限制嵌套深度 | 解压炸弹攻击风险 |
| **索引文件句柄泄漏** | `search_engine/manager.rs` | IndexReader未实现Drop，依赖系统回收 | 长时间运行文件句柄耗尽 |
| **SQLite连接池过小** | `storage/metadata.rs` | 连接池大小硬编码，无动态调整 | 高并发下连接等待 |

### 🟡 MEDIUM (计划修复)

| 问题 | 位置 | 描述 |
|------|------|------|
| **测试覆盖率不足** | `archive/` | 压缩包处理模块测试覆盖<50% |
| **错误处理不完善** | `commands/import.rs` | 部分错误只打印日志未返回给前端 |
| **性能监控缺失** | `monitoring/` | 监控模块为空壳，无实际指标收集 |
| **配置验证缺失** | `models/config.rs` | 用户配置无schema验证，可能导致运行时错误 |
| **国际化不完整** | `i18n/` | 部分错误信息未走i18n字典 |
| **缓存无过期策略** | `utils/cache.rs` | LRU缓存无TTL，过期数据长期驻留 |
| **日志级别不合理** | 多处 | DEBUG级别日志过多，影响性能 |
| **依赖版本滞后** | `Cargo.toml` | 部分crate使用旧版本，存在已知漏洞 |
| **前端状态不同步** | `stores/` | 部分Zustand状态未持久化，刷新丢失 |
| **类型定义重复** | `types/` vs `models/` | 前后端共享类型存在重复定义 |
| **CSS硬编码** | `components/` | 部分组件使用硬编码颜色，不支持主题切换 |
| **事件监听未清理** | `hooks/` | 部分useEffect返回的清理函数不完整 |

### 🟢 LOW (优化项)

| 问题 | 位置 | 描述 |
|------|------|------|
| **冗余代码** | `services/pattern_matcher.rs` | 部分辅助函数未使用 |
| **命名不一致** | 多处 | 部分变量使用缩写，可读性差 |
| **文档注释缺失** | `task_manager/` | 部分pub函数无doc comment |
| **魔术数字** | `search_engine/` | 缓冲区大小等参数硬编码 |
| **导入顺序混乱** | 多处 | 未按标准顺序组织use语句 |
| **单元测试分散** | `*/mod.rs` | 部分测试放在生产代码文件中 |
| **clippy警告** | 多处 | 存在allow(clippy::*)压制警告 |
| **死代码** | `utils/` | 部分工具函数已废弃但未删除 |
| **重复逻辑** | `commands/` | 部分命令有重复的验证逻辑 |
| **版本号硬编码** | `lib.rs` | 版本号未从Cargo.toml读取 |

### 修复建议与优先级

**立即修复 (本周):**
1. `cache_key` 完整性修复 → 添加所有相关字段到哈希计算
2. 替换无界通道 → 使用`tokio::sync::mpsc::channel(1000)`并处理背压
3. 竞态条件修复 → 使用原子操作或文件锁

**短期修复 (本月):**
- ReaderPool实现或移除
- block_on调用审查，考虑使用spawn_blocking
- 错误类型统一

**中期优化 (下季度):**
- 测试覆盖率提升
- 性能监控实现
- 配置验证完善

***

*最后更新: 2026-03-31 | 问题清单基于完整代码库审查*
