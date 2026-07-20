# 文档索引

本目录同时是 [VitePress 文档站](./index.md)的内容源，只保留会随代码持续维护的说明。原则：

- 保留会随着代码一起持续维护的说明
- 删除一次性分析、修复计划和重复指南
- 文档内容以当前代码**真实行为**为准

---

## 建议阅读顺序

1. [快速开始](./guide/getting-started.md) — 安装、导入与第一次搜索
2. [功能概览](./guide/features.md) — 产品能力与明确边界
3. [架构总览](./architecture/overview.md) — 分层、workspace crates 与关键设计约束
4. [环境与启动](./development/setup.md) — 开发环境、应用与文档站命令
5. [CI 与 GitHub 工作流](./operations/ci.md) — 校验、发布与 Pages 部署

---

## 文档目录

### 项目与流程

| 文档 | 说明 |
|------|------|
| [CONTRIB.md](./CONTRIB.md) | 开发环境（含平台前置依赖）、构建命令、提交流程、CI 说明 |
| [RUNBOOK.md](./RUNBOOK.md) | 构建运行、故障排查、常见问题、回滚建议、发布前核对 |
| [guide/](./guide/getting-started.md) | 用户快速开始、工作区、搜索、过滤、关键词和监听 |
| [development/](./development/setup.md) | 环境、项目结构与测试质量指南 |
| [operations/](./operations/ci.md) | CI、发布与故障排查 |
### 架构

| 文档 | 说明 |
|------|------|
| [architecture/CAS_ARCHITECTURE.md](./architecture/CAS_ARCHITECTURE.md) | CAS 对象存储设计、SQLite 元数据表结构、导入/搜索数据流、GC 机制 |
| [architecture/overview.md](./architecture/overview.md) | Clean Architecture 分层与 workspace crates |
| [architecture/search.md](./architecture/search.md) | 查询规划、批处理、过滤与结果会话 |
| [architecture/import.md](./architecture/import.md) | 导入流水线、归档安全与内容入库 |
| [architecture/ipc.md](./architecture/ipc.md) | Tauri commands、events 与前端状态投影 |
### 仓库级文档

| 文档 | 说明 |
|------|------|
| [CHANGELOG.md](https://github.com/ashllll/log-analyzer_rust/blob/main/CHANGELOG.md) | 历史版本变更记录 |
| [RELEASE_PROCESS.md](https://github.com/ashllll/log-analyzer_rust/blob/main/RELEASE_PROCESS.md) | 发布步骤、版本策略、产物说明 |

---

## 核心架构速查

### 模块分层

```text
React 前端（pages / hooks / stores / services）
  ↓ Tauri invoke / emit / listen
后端命令层（commands/）
  ↓ 协调调用
应用层（application use cases）
  ↓ domain traits
基础设施适配器（infrastructure）
  ↓
Workspace Crates（la-core / la-storage / la-search / la-archive）
```

### 主搜索链路

```text
SearchPage → api.searchLogs
  → commands/search/mod.rs: search_logs
    → resolve_search_query（解析 + 验证）
    → WorkspaceService::search
      → SearchUseCase::execute（blocking pool）
      → CasLogFileRepository（metadata + CAS）
      → QueryEngineLogSearcher + CompiledSearchFilters
      → SearchSessionManager / DiskResultStore（分页）
```

### 导入链路

```text
WorkspacesPage → api.importFolder
  → commands/import.rs: import_folder
    → ImportPipeline（任务生命周期 + 失败清理）
    → la-archive（递归安全提取）
    → WorkspaceServiceImpl::import_file
    → la-storage（CAS + MetadataStore）
    → 完整性校验 + Tantivy segment merge
```

---

## 已移除的文档类型

以下类型不再保留在 `docs/` 目录中：

- 一次性架构分析报告
- 代码审查报告与修复计划
- AI 工具专用说明文档
- 与当前 CI 不匹配的本地环境模拟文档
- 与主 README 重复的用户快速参考页

如需恢复某类文档，先确认其有持续维护价值，并遵循"文档以真实行为为准"原则。
