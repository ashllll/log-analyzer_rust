# Requirements — fix-scan-issues

## Problem
代码库扫描发现 10 个架构和代码质量问题，影响可维护性、性能和正确性。

## Desired Outcome
修复全部 P0-P2 问题，提升代码质量和搜索性能。

## Actors
- 开发者（维护代码库）
- 用户（使用搜索功能，间接受益于性能提升和事件可靠性）

## Functional Requirements

### FR-1: 事件 emit 错误日志（B1）
`event_publisher.rs` 中 11 处 `let _ = self.app_handle.emit(...)` 必须至少记录 `warn!` 级别日志。

### FR-2: cleanup 函数拆分（A1）
`commands/workspace.rs:252-548` 的 `cleanup_workspace_resources` 拆分为 `application/` 层的 `CleanupUseCase`，包含 5 个独立步骤方法。

### FR-3: 消除深层嵌套（B2）
`la-search/src/manager.rs:598-697` 的 `document_to_log_entry_impl` 使用 `Option::and_then` 链或辅助宏消除 6 层嵌套。

### FR-4: 热路径分配优化（B3）
`infrastructure/searcher.rs:84-98` 的 match 路径减少堆分配：避免 HashSet 中间收集，复用 Vec 容量。

### FR-5: God Files 拆分（A2）
将 6 个 >20KB 文件拆分为子模块：
- `query_planner.rs` (47KB) → `query_planner/` 子模块
- `search.rs` (38KB) → `search/` 子模块（分离测试）
- `regex_engine.rs` (31KB) → `regex_engine/` 子模块
- 其余 3 个文件按需拆分

### FR-6: Mutex 统一（A3）
测试代码中的 `std::sync::Mutex` 替换为 `parking_lot::Mutex`。

### FR-7: glob 转义快速路径（B4）
`escape_sqlite_glob_literal` 添加快速路径：无需转义时返回 `Cow::Borrowed`。

### FR-8: url_decode 优化（B5）
`url_decode` 减少循环内 String 重新分配。

### FR-9: 类型统一（A4）
`la-archive` 的公开 API 类型改为 re-export 自 `la-core`，消除重复定义。

### FR-10: 消除重复编译（C3）
`CompiledSearchFilters::compile` 确保同一次搜索只编译一次，通过参数传递编译结果。

## Non-Functional Requirements
- 所有现有测试必须继续通过
- 不改变公开 API（Tauri IPC 命令签名不变）
- 每个修复独立可验证

## Constraints
- 保留已有未提交变更
- Rust 1.88 工具链
- 不引入新依赖（`parking_lot` 已在依赖中）

## Acceptance Criteria
- AC1: `cargo check --workspace` 通过
- AC2: `cargo test --workspace` 通过
- AC3: `cargo clippy --all-features --all-targets -- -D warnings` 通过
- AC4: 每个 issue 的修复可在代码中确认

## Assumptions
- God Files 拆分保持 module 公开 API 不变
- 拆分后的子模块使用 `pub mod` + `pub use` 保持兼容

## Out of Scope
- C1: query_planner 架构重设计（仅拆分文件）
- C2: 线程池复用确认（需要生产环境验证）
- 前端代码变更
