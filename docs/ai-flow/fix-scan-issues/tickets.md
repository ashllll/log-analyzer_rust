# Tickets — fix-scan-issues

## Ticket T1: B1 事件 emit 错误日志
- **ID:** T1
- **标题:** event_publisher emit 错误添加 warn 日志
- **用户可见结果:** 前端事件发送失败时后端日志可追踪
- **阻塞:** 无
- **验收标准:** event_publisher.rs 中所有 `let _ = emit(...)` 替换为 `if let Err(e) = emit(...)` + `tracing::warn!`
- **验证:** `cargo test -p la-core` + grep 确认无 `let _ = self.app_handle.emit`
- **状态:** ready

## Ticket T2: A1 cleanup 函数提取
- **ID:** T2
- **标题:** 提取 CleanupUseCase 到 application 层
- **用户可见结果:** 无行为变化，纯重构
- **阻塞:** 无
- **验收标准:**
  1. 新建 `application/cleanup.rs`，包含 `CleanupUseCase` 及 5 个步骤方法
  2. `commands/workspace.rs` 的 `cleanup_workspace_resources` 委托给 `CleanupUseCase::execute`
  3. 原函数体缩减为 <20 行
- **验证:** `cargo test --workspace` 全部通过
- **状态:** ready

## Ticket T3: B2 消除嵌套 match
- **ID:** T3
- **标题:** document_to_log_entry_impl 使用宏消除 6 层嵌套
- **用户可见结果:** 无行为变化，纯重构
- **阻塞:** 无
- **验收标准:**
  1. 定义 `extract_field!` 宏
  2. 6 个字段提取使用宏，函数体缩减约 50%
  3. 行为不变：字段缺失仍返回 `None`
- **验证:** `cargo test -p la-search`
- **状态:** ready

## Ticket T4: B3 热路径分配优化
- **ID:** T4
- **标题:** searcher.rs match_content 减少每行堆分配
- **用户可见结果:** 搜索性能提升（百万级日志）
- **阻塞:** 无
- **验收标准:**
  1. `virtual_path` 缓存为 `Arc<str>`，`file` 和 `real_path` 共享
  2. `HashSet` 中间收集改为排序+dedup `Vec`
  3. 行为不变：输出 LogEntry 完全一致
- **验证:** `cargo test --workspace` + 现有搜索集成测试
- **状态:** ready

## Ticket T5: A3 Mutex 统一
- **ID:** T5
- **标题:** 测试代码 std::sync::Mutex 替换为 parking_lot::Mutex
- **用户可见结果:** 无
- **阻塞:** 无
- **验收标准:** `application/mod.rs` 中 `std::sync::Mutex` 全部替换为 `parking_lot::Mutex`，lock() 调用去掉 `.unwrap()`（parking_lot 无 poison）
- **验证:** `cargo test --workspace`
- **状态:** ready

## Ticket T6: B4 glob 转义快速路径
- **ID:** T6
- **标题:** escape_sqlite_glob_literal 添加 Cow 快速路径
- **用户可见结果:** 无行为变化
- **阻塞:** 无
- **验收标准:**
  1. 返回类型改为 `Cow<'_, str>`
  2. 无需转义时返回 `Cow::Borrowed`
  3. 调用点适配 `Cow` 类型
- **验证:** `cargo test --workspace`
- **状态:** ready

## Ticket T7: B5 url_decode 优化
- **ID:** T7
- **标题:** url_decode 减少循环内分配
- **用户可见结果:** 无行为变化
- **阻塞:** 无
- **验收标准:**
  1. 移除 `format!("{h1}{h2}")`，改为位运算直接构造字节
  2. 无 % 字符的快速路径返回 `Cow::Borrowed`
- **验证:** `cargo test --workspace` + 现有 url_decode 测试
- **状态:** ready

## Ticket T8: A2 God Files 拆分
- **ID:** T8
- **标题:** 3 个 God Files 拆分为子模块目录
- **用户可见结果:** 无行为变化，纯重构
- **阻塞:** T1, T2, T3（先完成小修复再拆分，避免冲突）
- **验收标准:**
  1. `query_planner.rs` → `query_planner/{mod.rs, plan.rs, engine_cache.rs, helpers.rs, tests.rs}`
  2. `search.rs` → `search/{mod.rs, helpers.rs, tests.rs}`
  3. `regex_engine.rs` → `regex_engine/{mod.rs, aho_corasick.rs, standard.rs, memchr.rs, fancy.rs, analysis.rs, tests.rs}`
  4. 所有 `pub` API 通过 `pub use` re-export
  5. 外部调用者无需修改 import
- **验证:** `cargo check --workspace` + `cargo test --workspace`
- **状态:** blocked (depends on T1, T2, T3)

## Ticket T9: A4 类型统一
- **ID:** T9
- **标题:** ExtractionPolicy 统一到 la-core re-export
- **用户可见结果:** 无
- **阻塞:** 无
- **验收标准:**
  1. 将 la-archive 的 7 字段 ExtractionPolicy 合并到 la-core
  2. la-archive 改为 `pub use la_core::ExtractionPolicy`
  3. 消除重复定义
- **验证:** `cargo test --workspace`
- **状态:** ready

## Ticket T10: C3 消除重复编译
- **ID:** T10
- **标题:** CompiledSearchFilters 同一搜索只编译一次
- **用户可见结果:** 搜索性能微提升
- **阻塞:** T8（search.rs 拆分后再修改）
- **验收标准:**
  1. `LogSearcher::match_content` 签名接受预编译 filters
  2. `SearchUseCase` 传递编译结果给 searcher
  3. searcher 不再内部调用 `CompiledSearchFilters::compile`
- **验证:** `cargo test --workspace`
- **状态:** blocked (depends on T8)

---

## Dependency Graph

```
T1 ──┐
T2 ──┼──→ T8 ──→ T10
T3 ──┘
T4 (independent)
T5 (independent)
T6 (independent)
T7 (independent)
T9 (independent)
```

## Execution Order

1. **Phase A** (parallel): T1, T2, T3, T4, T5, T6, T7, T9
2. **Phase B**: T8 (God Files 拆分)
3. **Phase C**: T10 (消除重复编译)
