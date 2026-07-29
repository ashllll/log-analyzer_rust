# CONTEXT.md — 领域词汇表

> 本文件记录项目的领域语言（ubiquitous language）。架构评审与深化时命名模块，
> 应优先使用本表已有的词；引入新概念时同步登记。

## 搜索执行

| 术语 | 含义 |
|---|---|
| **段（Segment）** | 搜索编排器把候选文件列表按序切分的工作单元：连续小文件（< 64KB，`LARGE_FILE_STREAM_THRESHOLD_BYTES`）累积为一个段；单个大文件（≥ 64KB）自成一个段。结果按段序落库，与文件列表顺序一致。 |
| **编排器（Orchestrator）** | `SearchUseCase::run_blocking` 深化后的角色：`build_plan` → 分段 → 按段类型派发策略 → 收尾 flush。持有全局唯一的 `SearchBatch`。 |
| **ParallelSmallFileStrategy** | 处理小文件段的具体结构体（application 层，无 trait——见 ADR-0001）。Rayon 并行搜索段内文件，逐文件调共享 consume。 |
| **StreamingLargeFileStrategy** | 处理大文件段的具体结构体（application 层，无 trait——见 ADR-0001）。按 512 行 chunk 流式读取（`SEARCH_LINE_CHUNK_SIZE`），每 chunk 调共享 consume，收到停止信号立即中断。 |
| **SearchBatch** | 全局唯一的批量累加器（已有模块，`application/search_batch.rs`）。决定 Continue / Flush / Truncate；`max_results` 全局截断依赖其全局唯一性。 |
| **共享 consume** | `consume_search_entries` / `flush_search_batch` 深化后的定位：两个策略共用的批量→落库→进度 helper，批量逻辑单点保留。 |

## 存储

| 术语 | 含义 |
|---|---|
| **CAS** | Content Addressable Storage，按 SHA-256 哈希存储日志内容，跨工作区去重。 |
| **工作区（Workspace）** | 一组导入日志文件的容器；数据落在 `{app_data_dir}/workspaces/{workspace_id}/`（CAS + `metadata.db`）。 |
| **内容身份（Content Identity）** | CAS 对象的 SHA-256；相同字节只存一份，可被多个元数据路径复用。 |
| **路径身份（Path Identity）** | `metadata.db` 中文件或归档的唯一 `virtual_path`；同一路径重新导入时原地更新内容引用。 |
