# SearchUseCase 策略模块深化规格

## 目标与已核对基线

本规格只落实候选 1：将 `SearchUseCase` 的文件处理深化为两个具体 application 结构体，复用现有 `SearchBatch`，不定义策略 trait，并保持现有搜索行为。它不改变 `SearchUseCase::execute`、`SearchService` 或 Tauri command 的外部 interface。

已核对当前实现（`log-analyzer/src-tauri/src/application/search.rs`）与 `docs/adr/0001-no-file-processing-strategy-trait.md`、`docs/CONTEXT.md`：

- `SearchUseCase::run_blocking` 当前同时负责构建计划、按 `FILE_CHUNK_SIZE = 10` 编排文件、区分小文件和大文件、取消检查、批量消费和最终 flush。
- 小文件路径使用 Rayon `par_iter`，随后对 `collect` 的结果按输入顺序消费；单文件读取失败会记录 warning 并跳过。
- 大文件路径调用 `read_line_chunks_sync`，每个 chunk 为 `SEARCH_LINE_CHUNK_SIZE = 512` 行；每个 chunk 检查取消，读取失败会记录 warning、跳过该文件并继续后续文件。
- 小文件阈值为 `LARGE_FILE_STREAM_THRESHOLD_BYTES = 64 * 1024`：`size < 64 KiB` 走并行路径，`size >= 64 KiB` 走流式路径。
- `SearchBatch` 由编排器唯一创建，当前 `BATCH_SIZE = 256`。共享消费逻辑依据它决定 Continue、Flush 或 Truncate；结果 ID 在消费时以 `batch.total()` 为基准分配，因此 `max_results` 跨策略生效。
- 大文件行号通过 `start_line.saturating_sub(1)` 修正；该行号偏移与结果 ID 是两个独立语义。
- ADR-0001 已接受“两个具体结构体、编排器直接调用、不定义 `FileProcessingStrategy` trait”；`docs/CONTEXT.md` 将 `SearchBatch` 定义为全局批处理器。本规格遵循这两项决策。

## 实现形状

在 application 层新增 `search_strategy.rs`，定义以下两个具体结构体。它们不实现共同 trait，也不新增 domain interface、adapter 或策略注册表。

### `ParallelSmallFileStrategy`

该结构体处理一次既有 `FILE_CHUNK_SIZE = 10` 执行批次中，直到大文件边界前收集的小文件。它将沿用当前 Rayon adapter：对小文件并行读取和匹配，再按 `par_iter().collect()` 产生的输入顺序逐文件调用共享消费函数。并行 map 在读取每个文件前检查 cancellation token；策略按顺序消费已收集的结果，消费阶段由共享批量逻辑处理全局截断。策略返回 `false` 仅表示全局截断或已确认的停止状态，其他情况返回 `true`。

### `StreamingLargeFileStrategy`

该结构体一次处理一个 `size >= 64 KiB` 的文件。它将沿用 `read_line_chunks_sync` 和 512 行 chunk；每次回调先检查 cancellation token，再匹配当前 chunk、修正 `start_line` 行号并调用共享消费函数。收到取消或全局截断后立即停止后续 chunk 并返回 `false`。读取错误继续记录 warning、跳过当前文件并返回 `true`，不改变现有错误传播策略。

## 编排与共享状态

`SearchUseCase::run_blocking` 将只保留编排职责：构建匹配计划，按文件列表顺序遍历既有的 10 文件执行批次，在每个大文件前派发并 flush 待处理的小文件，派发大文件策略，并在每个执行批次结束时 flush 剩余小文件。它将在搜索开始创建一个 `SearchBatch`，把同一个可变引用传给两种策略，响应策略返回的 `false`，最后执行一次剩余批次 flush。

策略模块使用一个私有 application 执行上下文集中携带现有 adapter 引用、匹配计划、已编译过滤器、搜索会话 ID、`max_results` 和 cancellation token。上下文不对 domain 层公开。策略不创建、替换或重置 `SearchBatch`。

共享消费函数和 flush 函数只保留一份实现，负责：以 `SearchBatch::total()` 分配结果 ID，调用 `SearchBatch::accumulate`，批量写入结果仓库，发布现有进度事件，并传播结果写入错误。`SearchBatch` 的大小、累计总数、全局截断和持久化协议保持不变；进度事件继续使用当前异步 spawn 行为。

## 不变量（必须保持）

- `SearchUseCase::execute`、`SearchService`、Tauri command 的名称、参数、返回值和生命周期行为不变。
- `FILE_CHUNK_SIZE` 保持 10；文件列表顺序保持不变。小文件不因 Rayon 并行而重排，大文件仍在其原列表位置处理。
- `size < 64 KiB` 与 `size >= 64 KiB` 的路径选择保持不变；大文件 chunk 保持 512 行。
- `SearchBatch` 全局唯一；`max_results` 不因策略切换而重置。达到上限后不再产生新结果。
- 搜索开始前取消不产生结果；大文件读取期间取消停止后续 chunk。
- 大文件行号继续按 chunk 的 `start_line` 修正，不能把行号偏移用于结果 ID。
- 小文件 CAS 读取失败和大文件 chunk 读取失败均保持“记录 warning、跳过当前文件、继续搜索”的现有行为。
- 现有 `run_blocking` 流程测试全部保留，作为行为安全网；策略聚焦测试直接实例化两个具体结构体，不通过 trait 对象测试。

## 测试范围

新增策略聚焦测试，覆盖以下可观察行为：

1. 多个小文件即使并行执行，结果仍按输入顺序消费。
2. 大文件使用 512 行 chunk，并在读取到 EOF 前按批次 flush。
3. 小文件与大文件交替时，共享批次产生连续结果 ID。
4. 64 KiB 阈值两侧选择正确路径。
5. 搜索开始前取消不产生结果；大文件读取期间取消停止后续 chunk。
6. `max_results` 跨两种策略全局生效。
7. 大文件 `start_line` 正确修正，且不影响结果 ID 语义。
8. 大文件读取失败跳过当前文件并继续。

保留当前 `SearchUseCase::run_blocking` 测试对构建计划错误、空文件集、最终 flush、批次边界、截断、取消、超时和结果写入错误的覆盖。实现验证先运行 application 搜索测试，再运行完整 Rust workspace 测试；本次不修改前端代码或行为。

## 明确的非目标

- 不改变搜索查询语法、正则分类、ReDoS 验证、过滤器语义或匹配计划。
- 不改变 `SearchBatch` 的批量大小、全局唯一性、结果 ID 分配规则或持久化协议。
- 不改变 CAS、MetadataStore、DiskResultStore、搜索事件 payload 或 Tauri IPC。
- 不新增 `FileProcessingStrategy` trait、domain trait、adapter、注册表或第三种策略。
- 不拆分现有 `search.rs` 测试桩；现有流程测试留在原位置，策略测试放在新策略模块的测试范围内。
- 不修改 ADR-0001 或 `docs/CONTEXT.md` 的核心决策与领域语义；实现必须同步修正 `docs/architecture/search.md` 中受影响的模块位置和策略说明，避免架构文档继续描述已移动的实现。
- 不修复与策略拆分无关的历史文档数字或其他架构候选。

## 实施边界

本规格锁定模块职责、调用顺序、状态所有权和行为不变量；具体 Rust 字段布局与函数签名由实现依据当前 domain trait 类型确定。若实现发现当前代码与 ADR-0001 的结果顺序、全局截断、取消、ID 或行号语义冲突，必须保留已核对的运行时行为并在代码审查中报告冲突；不得通过新增 trait 解决冲突。

## Documentation impact

- Feature / user-facing docs introduced: none
- Materially amended existing docs: `docs/architecture/search.md`
- Derived / memory docs invalidated: none
