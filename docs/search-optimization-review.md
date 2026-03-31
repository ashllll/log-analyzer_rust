# 搜索性能与边界条件二次审核说明

## 审核范围

本次审核只针对当前工作空间里真实承载搜索的业务代码路径：

- `log-analyzer/src-tauri/src/commands/search.rs`
- `log-analyzer/src-tauri/src/services/file_watcher.rs`

核对结果：

- 主搜索链路实际走的是 `search_logs` / `search_logs_paged`。
- 查询执行实际由 `QueryExecutor` / `QueryPlanner` / `RegexEngine` 完成逐行匹配。
- 时间、级别、文件路径过滤实际在 `commands/search.rs` 中执行。
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

本次刻意未改变的语义：

- 简单查询主入口仍然是 `query.split('|')`
- 未把前端结构化查询直接接入 `search_logs`
- 未把 README 中预留的 Tantivy 路径强行接入现有主搜索

这是为了避免把未投入真实业务主链路的能力误当成当前行为。

## 验证结果

已执行：

- `cargo fmt`
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
  - 当前项目现有大小写不敏感多模式匹配能力仍保持不变
  - 参考: <https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasickBuilder.html>

## 说明

你提供的 ChatGPT share 链接在本地校验时返回 `share_not_found`，因此本次审核没有把该链接内容作为事实依据使用，而是完全以仓库内可执行代码、测试结果和官方文档为准。
