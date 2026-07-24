# Specification — fix-scan-issues

## Current-State Evidence

### B1: event_publisher.rs — 11 处 `let _ = emit()`
- 文件：`infrastructure/event_publisher.rs`
- 11 个 trait 方法全部静默丢弃 `app_handle.emit()` 的 `Result`
- 同文件的 `emit_workspace_event_with_retry`（第 94-136 行）已有正确的 `tracing::warn!` 模式可参考

### A1: workspace.rs cleanup — 297 行
- 文件：`commands/workspace.rs:252-548`
- 5 个步骤：停止 watcher → 跳过缓存 → 关闭 DB → 删除索引文件 → 删除目录
- 使用 `Vec<String>` 收集错误，最终判断是否 critical
- 签名：`async fn cleanup_workspace_resources(workspace_id: &str, state: &AppState, app: &AppHandle)`

### B2: manager.rs 嵌套 — 6 层 match
- 文件：`la-search/src/manager.rs:598-697`
- 6 个字段（content/timestamp/level/file_path/real_path/line_number）重复相同的 2 层嵌套 pattern
- 5 个字段映射到 `Arc<str>`，1 个映射到 `usize`

### B3: searcher.rs 热路径 — 8+ 次分配/行
- 文件：`infrastructure/searcher.rs:84-98`
- 每行匹配：`clone` → `HashSet` → `Vec` → 6x `Arc<str>`/`String` → `line.to_string()`
- `virtual_path.to_string()` 对 `file` 和 `real_path` 重复调用
- `tags: vec![]` 始终为空

### A2: God Files
- `query_planner.rs` (47KB, ~1466 行)：QueryPlanner struct + ExecutionPlan + PlanTerm + CompiledEngine + 34 个测试 + 11 个自由函数
- `search.rs` (38KB)：SearchUseCase + 6 个测试桩 struct + 4 个测试 + 5 个辅助函数
- `regex_engine.rs` (31KB, ~980 行)：RegexEngine enum + 4 个引擎实现 (AhoCorasick/Standard/Memchr/Fancy) + MatchResult + 22 个测试
- `workspace.rs` (27KB)：7 个 Tauri command + cleanup
- `validation/helpers.rs` (26KB)
- `task_manager/mod.rs` (25KB)

### A3: Mutex 不一致
- `application/mod.rs:75,123` 使用 `std::sync::Mutex`
- 生产代码全部使用 `parking_lot::Mutex`

### B4: escape_sqlite_glob_literal
- 文件：`services/search_filters.rs:40-52`
- 始终创建新 `String` 并逐字符复制，即使无需转义

### B5: url_decode
- 文件：`utils/validation/helpers.rs:111-178`
- 使用 `Vec<u8>` 收集字节，`format!("{h1}{h2}")` 每次 %XX 创建临时 String
- `char::encode_utf8` 使用栈上 buffer，相对合理

### A4: ExtractionPolicy 重复
- `la-archive/src/extraction_engine.rs:186` — 7 字段版本（含 buffer_size, dir_batch_size 等）
- `la-core/src/domain/extract.rs:35` — 3 字段版本（仅 max_depth, max_file_size, max_total_size）
- `la-core/src/models/extraction_policy.rs:46` — 配置文件加载版本（嵌套子结构）

### C3: CompiledSearchFilters 重复编译
- `application/search.rs:84` — 编译一次
- `infrastructure/searcher.rs:55` — 再编译一次
- 同一次搜索中 filters 被编译两次

---

## Proposed Design

### B1 Fix: emit 错误日志
将所有 `let _ = self.app_handle.emit(...)` 改为 match 模式，失败时 `tracing::warn!`。
参考同文件 `emit_workspace_event_with_retry` 的日志模式。

```rust
// Before:
let _ = self.app_handle.emit("search-start", payload);
// After:
if let Err(e) = self.app_handle.emit("search-start", payload) {
    tracing::warn!(error = %e, "Failed to emit search-start event");
}
```

### A1 Fix: 提取 CleanupUseCase
1. 在 `application/` 下创建 `cleanup.rs`
2. 定义 `CleanupUseCase` struct，包含 5 个步骤方法
3. `commands/workspace.rs` 的 `cleanup_workspace_resources` 委托给 `CleanupUseCase`
4. 保持相同的错误收集模式

```
application/cleanup.rs:
  pub struct CleanupUseCase
    pub async fn execute(workspace_id, state, app) -> Result
    async fn stop_watcher(workspace_id, state)
    async fn close_databases(workspace_id, state)
    async fn delete_legacy_indices(workspace_id, app)
    async fn delete_workspace_directory(workspace_id, state, app)
```

### B2 Fix: 消除嵌套
使用辅助宏 `extract_field!` 消除重复：

```rust
macro_rules! extract_field {
    ($doc:expr, $schema:expr, $field:ident, $method:ident) => {
        match $doc.get_first($schema.$field) {
            Some(v) => match v.$method() {
                Some(val) => val,
                None => {
                    tracing::warn!(field = stringify!($field), "Document has wrong-typed field, skipping");
                    return None;
                }
            },
            None => {
                tracing::warn!(field = stringify!($field), "Document missing required field, skipping");
                return None;
            }
        }
    };
}
```

### B3 Fix: 热路径优化
1. `virtual_path` 的 `to_string()` 结果缓存为 `Arc<str>`，`file` 和 `real_path` 共享
2. `HashSet` 中间收集改为排序+去重 `Vec`（避免 HashSet 开销）
3. `line` 参数改为 `&str` 避免 `to_string()`（已经是 `&str`，确认调用链）
4. `tags: vec![]` 改为使用常量空 Vec 或 `Vec::new()`（已是零分配，确认）

### A2 Fix: God Files 拆分

**query_planner.rs → query_planner/ 目录：**
- `mod.rs`：re-export + QueryPlanner struct + 核心方法 (new, build, validate_query)
- `plan.rs`：ExecutionPlan + PlanTerm + impl blocks
- `engine_cache.rs`：CompiledEngine + engine_cost_rank + engine_is_match + engine_find_all
- `helpers.rs`：自由函数 (validate_term, contains_backreference, should_use_word_boundary 等)
- `tests.rs`：全部 #[cfg(test)] 测试

**search.rs → search/ 目录：**
- `mod.rs`：re-export + SearchUseCase struct + 核心方法
- `helpers.rs`：emit_progress, emit_error 辅助函数
- `tests.rs`：全部测试桩和测试

**regex_engine.rs → regex_engine/ 目录：**
- `mod.rs`：re-export + RegexEngine enum + 公开 API
- `aho_corasick.rs`：AhoCorasickEngine + AhoCorasickMatches
- `standard.rs`：StandardEngine + StandardMatches
- `memchr.rs`：MemchrEngine + MemchrMatches
- `fancy.rs`：FancyEngine + FancyMatches
- `analysis.rs`：is_simple_keyword, regex_complexity_score, needs_lookaround, check_redos_risk 等
- `tests.rs`：全部测试

### A3 Fix: Mutex 统一
`application/mod.rs:75,123` 将 `std::sync::Mutex` 改为 `parking_lot::Mutex`。
API 差异：`parking_lot::Mutex::lock()` 返回 `MutexGuard` 不返回 `Result`（无 poison），直接调用即可。

### B4 Fix: glob 转义快速路径
```rust
fn escape_sqlite_glob_literal(value: &str) -> Cow<'_, str> {
    if !value.contains(|c| matches!(c, '*' | '?' | '[' | ']')) {
        return Cow::Borrowed(value);
    }
    let mut e = String::with_capacity(value.len() + 4);
    // ... existing loop
    Cow::Owned(e)
}
```

### B5 Fix: url_decode 优化
1. 移除 `format!("{h1}{h2}")`，改为直接字节拼接 `((h1 as u8) << 4) | (h2 as u8)`
2. 对无 % 字符的快速路径返回 `Cow::Borrowed`

### A4 Fix: 类型统一
`la-archive/src/extraction_engine.rs` 的 `ExtractionPolicy` 改为从 `la-core` re-export。
需要注意 `la-archive` 版本有 7 个字段而 `la-core` 版本只有 3 个——需要先将完整 7 字段定义合并到 `la-core`。

### C3 Fix: 消除重复编译
`SearchUseCase` 在 `execute` 中编译 filters 后，将编译结果传递给 `searcher.match_content`，
而不是让 searcher 内部再次编译。修改 `LogSearcher` trait 的 `match_content` 签名接受预编译的 filters。

---

## Affected Modules

| Issue | 修改文件 | Crate |
|---|---|---|
| B1 | `infrastructure/event_publisher.rs` | src-tauri |
| A1 | `commands/workspace.rs`, `application/cleanup.rs` (新建) | src-tauri |
| B2 | `crates/la-search/src/manager.rs` | la-search |
| B3 | `infrastructure/searcher.rs` | src-tauri |
| A2 | `services/query_planner.rs` → `services/query_planner/` | src-tauri |
| A2 | `application/search.rs` → `application/search/` | src-tauri |
| A2 | `services/regex_engine.rs` → `services/regex_engine/` | src-tauri |
| A3 | `application/mod.rs` | src-tauri |
| B4 | `services/search_filters.rs` | src-tauri |
| B5 | `utils/validation/helpers.rs` | src-tauri |
| A4 | `crates/la-archive/src/extraction_engine.rs`, `la-core/src/domain/extract.rs` | la-archive, la-core |
| C3 | `domain/mod.rs` (trait), `application/search.rs`, `infrastructure/searcher.rs` | la-core, src-tauri |

## Error and Edge-Case Behavior
- B1：emit 失败仅记录日志，不中断业务流程（与现有行为一致）
- A1：cleanup 错误收集逻辑保持不变，仅改变代码组织
- B2：字段缺失仍返回 `None`（跳过整条文档），行为不变
- B3：优化后的结果必须与原始结果完全一致
- A2：模块拆分后所有 `pub` API 通过 `pub use` re-export，外部调用者无需修改
- C3：trait 签名变更需要同步更新所有实现者

## Compatibility
- 不改变 Tauri IPC 命令签名
- 不改变前端事件格式
- `LogSearcher` trait 签名变更是内部 API，不影响前端

## Testing Strategy
- 每个修复后运行 `cargo test --workspace`
- A2 拆分后确认所有现有测试通过
- C3 需要确认 `match_content` 的 trait 实现者全部更新
