# 文档索引

本目录只保留当前项目仍需长期维护的核心文档。原则：

- 保留会随着代码一起持续维护的说明
- 删除一次性分析、修复计划和重复指南
- 文档内容以当前代码**真实行为**为准

---

## 建议阅读顺序

1. [项目总览](../README.md) — 技术栈、主要能力、仓库结构、核心搜索链路
2. [贡献指南](./CONTRIB.md) — 开发环境搭建、提交流程、测试约定
3. [IPC API 概览](./architecture/API.md) — 40+ Tauri 命令与事件接口说明
4. [模块架构详解](./architecture/modules/MODULE_ARCHITECTURE.md) — 各层模块职责、数据结构、调用关系
5. [CAS 存储架构](./architecture/CAS_ARCHITECTURE.md) — 内容寻址存储、SQLite 元数据、导入/搜索数据流

---

## 文档目录

### 项目与流程

| 文档 | 说明 |
|------|------|
| [CONTRIB.md](./CONTRIB.md) | 开发环境（含平台前置依赖）、构建命令、提交流程、CI 说明 |
| [RUNBOOK.md](./RUNBOOK.md) | 构建运行、故障排查、常见问题、回滚建议、发布前核对 |
| [search-optimization-review.md](./search-optimization-review.md) | 搜索性能边界条件审核、已落地优化项、验证结果 |

### 架构

| 文档 | 说明 |
|------|------|
| [architecture/API.md](./architecture/API.md) | 所有 IPC 命令与后端推送事件的详细接口说明 |
| [architecture/CAS_ARCHITECTURE.md](./architecture/CAS_ARCHITECTURE.md) | CAS 对象存储设计、SQLite 元数据表结构、导入/搜索数据流、GC 机制 |
| [architecture/modules/MODULE_ARCHITECTURE.md](./architecture/modules/MODULE_ARCHITECTURE.md) | 各模块（la-core / la-storage / la-search / la-archive / commands / services 等）的详细设计 |
| [architecture/modules/README.md](./architecture/modules/README.md) | 模块架构文档入口 |

### 仓库级文档

| 文档 | 说明 |
|------|------|
| [../CHANGELOG.md](../CHANGELOG.md) | 历史版本变更记录 |
| [../RELEASE_PROCESS.md](../RELEASE_PROCESS.md) | 发布步骤、版本策略、产物说明 |

---

## 核心架构速查

### 模块分层

```text
React 前端（pages / hooks / stores / services）
  ↓ Tauri invoke / emit / listen
后端命令层（commands/）
  ↓ 协调调用
业务服务层（services/ / storage/ / search_engine/ / archive/）
  ↓ 依赖
Workspace Crates（la-core / la-storage / la-search / la-archive）
```

### 主搜索链路

```text
SearchPage → api.searchLogs
  → commands/search.rs: search_logs
    → MetadataStore::get_all_files（文件列表）
    → CAS::retrieve（读取内容）
    → QueryExecutor（逐行匹配，Aho-Corasick 多模式预检）
    → DiskResultStore::write_results（写盘分页）
  → fetch_search_page（分页拉取）
```

### 导入链路

```text
WorkspacesPage → api.importFolder
  → commands/import.rs: import_folder
    → FileTypeFilter（过滤非日志文件）
    → la-archive: ExtractionOrchestrator（递归解压）
    → la-storage: StorageCoordinator（CAS + 元数据）
    → SearchEngineManager（建 Tantivy 索引）
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
