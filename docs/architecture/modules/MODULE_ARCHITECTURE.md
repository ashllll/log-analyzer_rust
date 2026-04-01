# 模块架构

本文档只保留当前代码结构中最重要的模块边界与调用关系。

## 总体分层

```text
React UI
→ api.ts / hooks
→ Tauri commands
→ services / storage / search_engine / archive
→ workspace crates
```

## 前端模块

### `src/pages/`

主要页面：

- `SearchPage.tsx`
- `WorkspacesPage.tsx`
- `KeywordsPage.tsx`
- `TasksPage.tsx`
- `PerformancePage.tsx`
- `SettingsPage.tsx`

其中 `SearchPage` 是搜索主入口。

### `src/services/`

职责：

- Tauri IPC 调用封装
- 查询构建器
- 前后端交互适配

关键文件：

- `api.ts`
- `SearchQueryBuilder.ts`

### `src/hooks/`

职责：

- 分页搜索
- 事件订阅
- 服务端查询封装

关键文件：

- `useInfiniteSearch.ts`

### `src/stores/`

职责：

- Zustand 状态管理

## 后端主 crate 模块

### `src/commands/`

职责：

- 暴露 Tauri IPC 命令
- 协调工作区、搜索、导入、导出和监听流程

关键文件：

- `search.rs`
- `workspace.rs`
- `import.rs`
- `watch.rs`
- `performance.rs`

### `src/services/`

职责：

- 查询验证、计划与执行
- 正则与多模式匹配
- 文件监听与日志元数据解析

关键文件：

- `query_validator.rs`
- `query_planner.rs`
- `query_executor.rs`
- `regex_engine.rs`
- `file_watcher.rs`

说明：

- 这里是真实搜索匹配逻辑的核心

### `src/search_engine/`

职责：

- 磁盘结果分页
- 搜索会话管理
- 查询优化和若干高级搜索基础设施

关键文件：

- `disk_result_store.rs`
- `virtual_search_manager.rs`
- `advanced_features.rs`

说明：

- 这里包含一些高级能力和基础设施
- 但当前主搜索命中逻辑仍由 `commands/search.rs + services/*` 驱动

### `src/storage/`

职责：

- CAS 对象存储
- 元数据存储
- 一致性与回收

关键文件：

- `cas.rs`
- `metadata_store.rs`
- `coordinator.rs`
- `gc.rs`

### `src/archive/`

职责：

- 归档处理入口与兼容封装

说明：

- 主要压缩包实现已下沉到 workspace crate `la-archive`

### `src/monitoring/`

职责：

- 性能指标采集与历史指标查询
- 指标聚合与统计摘要

关键文件：

- `metrics.rs`

### `src/task_manager/`

职责：

- 后台任务调度与生命周期管理
- Actor 模式的任务执行
- 任务消息传递与错误恢复

关键文件：

- `actor.rs`
- `messages.rs`
- `types.rs`

### `src/state_sync/`

职责：

- 前后端实时状态同步
- 工作区状态变更通知
- 事件历史记录与回放

关键文件：

- `models.rs`

## Workspace crates

### `la-core`

职责：

- 公共错误、模型、trait、工具类型

### `la-storage`

职责：

- CAS 与元数据存储核心实现

### `la-search`

职责：

- 搜索结果存储、高级搜索数据结构、查询优化器等

### `la-archive`

职责：

- ZIP / TAR / GZ / RAR / 7Z 等归档处理核心逻辑

## 真实搜索链路

当前主搜索链路：

```text
SearchPage
→ api.searchLogs()
→ commands/search.rs: search_logs
→ MetadataStore::get_all_files()
→ CAS 读内容
→ QueryExecutor / RegexEngine 匹配
→ DiskResultStore 写盘
→ fetch_search_page 分页返回
```

这个链路比“README 上看起来像 Tantivy 主检索”更接近当前真实行为，因此所有架构和性能文档都应以此为准。

## 阅读建议

若要理解项目，按以下顺序读代码更有效：

1. `src/pages/SearchPage.tsx`
2. `src/services/api.ts`
3. `src-tauri/src/commands/search.rs`
4. `src-tauri/src/services/query_*.rs`
5. `src-tauri/src/storage/*`
6. `src-tauri/crates/*`
