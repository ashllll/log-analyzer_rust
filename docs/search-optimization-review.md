# 搜索性能与边界条件二次审核说明

## 审核范围

本次审核只针对当前工作空间里真实承载搜索的业务代码路径：

- `log-analyzer/src-tauri/src/commands/search.rs`
- `log-analyzer/src-tauri/src/services/file_watcher.rs`
- `log-analyzer/src-tauri/src/services/query_planner.rs`
- `log-analyzer/src-tauri/src/services/query_executor.rs`

核对结果：

- 主搜索链路实际走的是 `search_logs` / `search_logs_paged`。
- 查询执行实际由 `QueryExecutor` / `QueryPlanner` / `RegexEngine` 完成逐行匹配。
- 时间、级别、文件路径过滤实际在 `commands/search.rs` 中执行。
- 主流程此前没有把“多关键词 OR 查询”的多模式匹配优势真正用到 `search_logs` 入口。
- README 中提到的 Tantivy / 高级索引能力当前并不是主搜索链路的实际执行入口，本次没有按“预留能力”误改主逻辑。

## 已确认的真实问题

### 1. 时间范围过滤与前端输入格式不兼容

真实现状：

- 前端过滤器使用 `datetime-local`，默认提交 `YYYY-MM-DDTHH:MM`。
- 日志时间戳提取器 `TimestampParser` 支持多种“无时区”格式。
- 搜索过滤逻辑却使用 `chrono::DateTime::parse_from_rfc3339` 比较开始/结束时间和命中行时间戳。

业务后果：

- 前端时间过滤值大概率解析失败，时间范围过滤形同失效。
- 日志中无法被 RFC3339 解析的时间戳会被静默放行，导致用户选择时间范围后仍看到范围外或无时间戳结果。

### 2. `filePattern` 名称与实际行为不一致

真实现状：

- 前端文案和测试将其当作 “pattern” 使用。
- 后端实现只是 `contains` 子串匹配。

业务后果：

- `*.log`、`logs/*.log` 这类典型模式不会生效。
- 文件过滤无法在读文件前提前裁剪候选文件，浪费 I/O 和匹配成本。

### 3. 命中结果 ID 存在重复风险

真实现状：

- 结果 ID 依赖 `global_offset + line_index`。
- 分页搜索分批扫描时 offset 不是全局唯一递增值。

业务后果：

- 前端使用 `entry.id` 建图和选中状态时，存在重复覆盖风险。

### 4. 时间 / 级别过滤仍然按“逐行重匹配后再过滤”执行

真实现状：

- 主搜索链路虽然已经支持文件级和行级过滤，但有时间范围或级别过滤时，仍然会对整份文件的每一行执行 `match_with_details`。
- 也就是说，过滤条件并没有形成真正的“先裁剪，再执行全文匹配”。

业务后果：

- 大文件场景下，即使用户已经明确选择了某个时间窗口或单一日志级别，搜索仍然会为大量注定被过滤掉的行付出匹配成本。
- 这与架构文档中的 “Segment Pruner before RecordMatcher” 不一致。

### 5. `query.split('|')` 主路径没有利用多模式 OR 匹配

真实现状：

- `search_logs` 会把用户输入按 `|` 切成多个 `SearchTerm`，然后交给 `QueryExecutor` 逐个 term 判断。
- `RegexEngine` 本身支持 Aho-Corasick 多模式匹配，但主路径此前只在“单个 term 内部包含 `|`”时才可能使用。

业务后果：

- 常见的 `error|warning|timeout` 这类 OR 查询，在真实入口上会退化为“每行多次单词匹配”，没有利用现成的多模式引擎做快速预检。

### 6. `SearchEngineManager` 生命周期不闭环

真实现状：

- 运行时代码中，文件监听增量索引、导入后 segment merge、工作区时间范围查询都依赖 `state.search_engine_managers`。
- 但导入主路径此前只初始化了 CAS 和 `MetadataStore`，没有真实创建并注册 `SearchEngineManager`。
- 同时，索引写入侧把 `LogEntry.timestamp` 直接按整数解析，而主业务日志时间戳大多是 `2024-01-15 10:30:45` 这类字符串。

业务后果：

- 导入后的历史日志没有被建立 Tantivy 索引。
- 文件监听新增日志可能命中 “manager 不存在” 分支，根本不会写入索引。
- 工作区时间范围查询可能返回空值或大量 `0` 时间戳，无法反映真实日志时间线。

## 已落地的改进

### 1. 新增过滤编译层

在 `commands/search.rs` 中增加 `CompiledSearchFilters`：

- 日志级别过滤预归一化为小写 `HashSet`
- 时间过滤在搜索启动前一次性编译
- 文件模式在搜索启动前一次性编译

收益：

- 避免每条命中结果重复解析过滤条件
- 错误输入能在搜索开始前直接返回验证错误

### 2. 统一时间解析入口

在 `file_watcher.rs` 中为 `TimestampParser` 增加 `parse_naive_datetime`：

- 复用现有日志时间格式
- 额外支持前端 `datetime-local` 的 `YYYY-MM-DDTHH:MM`
- 支持 RFC3339 带时区输入

收益：

- 搜索过滤和日志时间抽取使用同一套解析策略
- 消除“日志能抽取时间、过滤却不能比较”的链路不一致

### 3. 时间范围过滤改为严格语义

当用户启用时间范围过滤时：

- 命中行时间戳无法解析 -> 直接排除
- 开始时间晚于结束时间 -> 直接返回验证错误

这样更符合真实业务语义：用户已经明确要求按时间范围过滤，就不应该混入无法定位时间的日志。

### 4. `filePattern` 支持通配模式

实现规则：

- 含 `*` / `?` 时按 wildcard 编译为正则
- 不含通配符时仍保持原有子串匹配语义

收益：

- `logs/*.log`、`service-?.txt` 这类模式可直接使用
- 老的 `error.log` 子串输入不受影响

### 5. 文件级早筛与行级早筛

优化位置：

- 文件读入前先按 `filePattern` 过滤候选文件
- 行命中后、构建 `LogEntry` 前先做时间/级别过滤

收益：

- 减少无意义文件读取
- 减少被过滤结果的 `LogEntry` 分配和关键词统计成本

### 6. 结果 ID 改为结果集内顺序唯一

调整后：

- 命中结果在最终纳入结果集时再分配 `entry.id`
- `search_logs` 与 `search_logs_paged` 都保证当前搜索会话内唯一

收益：

- 前端 `loadedEntriesMap`、选中态与虚拟滚动映射更稳定

### 7. 增加文件内分段摘要与预裁剪

实现位置：

- `commands/search.rs`

实现方式：

- 仅当用户启用了时间范围或级别过滤时，主链路切换为“分段扫描”。
- 每 `256` 行构建一个轻量分段摘要，记录：
  - 分段内出现过的日志级别位图
  - 分段内可解析时间戳的最小值 / 最大值
- 若某个分段与当前时间范围、级别过滤不可能相交，则直接跳过，不再对该分段逐行执行 `match_with_details`。

收益：

- 把过滤真正前移到全文匹配之前。
- 对时间窗口很窄、级别过滤很严的大文件搜索，能够显著减少无效匹配调用次数。
- 保持现有返回结构、分页方式、事件流和 `LogEntry` 结构不变，属于可直接落地的主链路优化。

### 8. 为 OR 多关键词查询增加快速预检引擎

实现位置：

- `query_planner.rs`
- `query_executor.rs`

实现方式：

- 当查询满足以下条件时：
  - 全局操作符为 `OR`
  - 至少两个启用 term
  - term 都是非正则
  - 大小写敏感配置一致
- 额外构建一个共享的 `fast_or_engine`，优先使用 Aho-Corasick 对整行做一次多模式预检。
- 只有预检命中后，才回到现有逐 term 详情收集逻辑，保证高亮和命中词统计语义不变。

收益：

- `search_logs` 真实入口的 `error|warning|timeout` 这类查询终于能利用项目内已存在的多模式引擎能力。
- 先做一次线性多模式判定，再做详情提取，避免每个未命中行都反复执行多次单词匹配。

### 9. 补齐 `SearchEngineManager` 初始化与首次建索引

实现位置：

- `commands/import.rs`
- `crates/la-search/src/manager.rs`
- `commands/workspace.rs`

实现方式：

- 在 `import_folder` 中创建工作区 CAS / `MetadataStore` 后，立即初始化并注册 `SearchEngineManager`。
- 导入成功后，从 CAS + `MetadataStore` 重新遍历已导入文件，批量解析日志行并回填 Tantivy 索引。
- 建索引期间按文件批次定期 `commit`，导入结束后继续沿用现有 `commit_and_wait_merge`。
- 删除工作区时，补充移除运行态的 `workspace_dirs` / `cas_instances` / `metadata_stores` / `search_engine_managers`，保证生命周期对称。

收益：

- 导入后的历史日志首次就能拥有完整索引，而不是只靠后续 watch 增量补丁。
- `append_to_workspace_index`、`get_workspace_time_range`、导入后 merge 这些既有调用终于有了真实运行态依赖。
- 工作区删除后不会再保留悬空的搜索引擎资源引用。

### 10. 统一索引侧时间戳解析

实现方式：

- `SearchEngineManager::add_document` 现在支持：
  - Unix 秒/毫秒时间戳
  - RFC3339
  - 项目当前常见日志时间格式
- 只有真正无法解析时，才回退到 `0` 作为占位。

收益：

- 时间范围 fast field 不再被大面积写成 `0`。
- `get_time_range()` 返回的最小/最大时间更接近真实业务日志分布。

## 边界条件清单

本次已覆盖并补测的边界：

- 空白时间过滤值：视为未设置
- `datetime-local` 输入：可解析
- 开始时间晚于结束时间：直接报错
- 命中行无有效时间戳但启用时间过滤：排除
- 文件模式为 `*.log`：可匹配
- 文件模式为普通字符串：保持 `contains` 兼容
- 级别过滤大小写差异：统一按小写比较
- 搜索结果 ID 重复：消除
- 时间过滤存在但整段日志都没有可解析时间：整段直接排除
- 分段中同时存在范围内和范围外时间：只保留范围内行，不扩大匹配
- 级别过滤存在但整段只含其他级别：整段直接排除
- 多个简单 OR 关键词：共享快速预检与原有详情提取结果保持一致
- `SearchEngineManager` 未初始化：导入后已补齐注册
- 历史导入日志无初始索引：导入完成后补建
- 日志时间戳为常见字符串格式：索引侧可解析为 Unix 时间

本次刻意未改变的语义：

- 简单查询主入口仍然是 `query.split('|')`
- 未把前端结构化查询直接接入 `search_logs`
- 未把 README 中预留的 Tantivy 路径强行接入现有主搜索
- 未把仓库里 `FilterEngine` / `TimePartitionedIndex` 直接切为主搜索执行器，因为当前真实结果详情、分页缓存、事件通知仍围绕 `commands/search.rs` 组织

这是为了避免把未投入真实业务主链路的能力误当成当前行为。

## 验证结果

已执行：

- `cargo fmt`
- `cargo test -q manager -- --nocapture`
- `cargo test -q segment_pruning -- --nocapture`
- `cargo test -q fast_or_engine -- --nocapture`
- `cargo test -q search -- --nocapture`
- `cargo test -q`

结果：

- Rust 全量测试通过

额外修复：

- `monitoring` 模块中两个文档测试原本会导致 `cargo test -q` 失败，本次顺手修正为正确的 doctest 标记，保证仓库全量测试为绿色。

## 官方文档依据

本次实现对照的官方文档方向：

- Chrono `NaiveDateTime::parse_from_str`
  - 用于支持项目当前以“无时区本地时间”为主的日志时间格式
  - 参考: <https://docs.rs/chrono/latest/chrono/struct.NaiveDateTime.html>
- Regex `regex::escape`
  - 用于把 wildcard 模式安全转换为正则
  - 参考: <https://docs.rs/regex/latest/regex/fn.escape.html>
- Aho-Corasick builder 文档
  - 用于确认 `ascii_case_insensitive` 和多模式构建能力，支撑 OR 快速预检
  - 参考: <https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasickBuilder.html>
- Regex `Regex`
  - 用于保持现有 `is_match` / `find_iter` 详情提取语义不变
  - 参考: <https://docs.rs/regex/latest/regex/struct.Regex.html>
- Tantivy `ReloadPolicy`
  - 用于确认 `OnCommitWithDelay` 与显式 `reader.reload()` 的可见性语义
  - 参考: <https://docs.rs/tantivy/latest/tantivy/enum.ReloadPolicy.html>
- Tantivy `IndexWriter::commit`
  - 用于确认提交后再 reload reader 的生命周期做法
  - 参考: <https://docs.rs/tantivy/latest/tantivy/struct.IndexWriter.html>

## 说明

你提供的 ChatGPT share 链接在本地校验时返回 `share_not_found`，因此本次审核没有把该链接内容作为事实依据使用，而是完全以仓库内可执行代码、测试结果和官方文档为准。
