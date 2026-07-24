# State Ledger — fix-scan-issues

## Baseline
- **Git HEAD:** `c23a66b4cf9a8da4876f93816d8b5381de406d01`
- **Branch:** (current working branch, uncommitted changes exist)
- **Date:** 2026-07-21

## Current Phase
Phase 1: Grilling

## Scope
User chose: **全部 P0-P2（10+ 个问题）**

## Issues to Fix

### P0 🔴
| ID | Issue | File |
|---|---|---|
| B1 | 事件 emit 错误静默丢弃 (11处 `let _ =`) | `infrastructure/event_publisher.rs` |
| A1 | `cleanup_workspace_resources` 复杂度 43，297 行 | `commands/workspace.rs:252-548` |

### P1 🟡
| ID | Issue | File |
|---|---|---|
| B2 | `document_to_log_entry_impl` 6 层嵌套 match | `la-search/src/manager.rs:598-697` |
| B3 | 搜索热路径每行 8+ 次堆分配 | `infrastructure/searcher.rs:84-98` |

### P2 🟢
| ID | Issue | File |
|---|---|---|
| A2 | God Files 拆分 (>20KB) | `query_planner.rs`, `search.rs`, `regex_engine.rs` 等 6 个文件 |
| A3 | Mutex 实现不一致 | 测试代码用 `std::sync::Mutex`，生产用 `parking_lot` |
| B4 | `escape_sqlite_glob_literal` 循环内分配 | `utils/validation/helpers.rs` |
| B5 | `url_decode` 循环内分配 | `utils/validation/helpers.rs` |
| A4 | 重复结构体定义 | `la-archive` vs `la-core` 的 `ExtractionPolicy` |
| C3 | `CompiledSearchFilters::compile` 重复编译 | `application/search.rs` + `infrastructure/searcher.rs` |

## Completed Gates
(none yet)

## Assumptions
- 保留已有未提交变更，不 git stash
- P2 God Files 拆分如果过于侵入，可以降级为 TODO 注释

## Blockers
(none)

## Residual Risks
- P2 A2 God Files 拆分范围大，可能引入回归
- 全部修复的 blast radius 较大
