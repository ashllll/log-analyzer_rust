---
phase: 15-stats-backend-ffi
plan: 01
subsystem: FFI
tags: [flutter, rust, statistics, log-level]
dependency_graph:
  requires:
    - log-analyzer/src-tauri/src/ffi/commands_bridge.rs
    - log-analyzer/src-tauri/src/ffi/bridge.rs
    - log-analyzer_flutter/lib/shared/services/bridge_service.dart
  provides:
    - log-analyzer_flutter/lib/shared/providers/log_level_stats_provider.dart
  affects:
    - BridgeService.getLogLevelStats()
tech-stack:
  added:
    - LogLevelStats Flutter model
    - LogLevelStatsNotifier Riverpod provider
    - LogLevelStatsOutput Rust FFI type
  patterns:
    - Riverpod 3.0 AsyncNotifier with auto-refresh
    - FFI bridge pattern (Rust -> Dart)
    - 5-second auto-refresh timer
key-files:
  created:
    - log-analyzer_flutter/lib/shared/providers/log_level_stats_provider.dart
  modified:
    - log-analyzer/src-tauri/src/ffi/types.rs
    - log-analyzer/src-tauri/src/ffi/commands_bridge.rs
    - log-analyzer/src-tauri/src/ffi/bridge.rs
    - log-analyzer_flutter/lib/shared/services/bridge_service.dart
decisions:
  - Used Map<String, dynamic> return type in BridgeService to avoid direct FFI type dependency
  - Implemented 5-second auto-refresh per STATS-03 requirement
  - Created local LogLevelStats model to decouple from FFI generated types
---

# Phase 15 Plan 01: 日志级别统计后端 FFI 接口 Summary

## One-Liner

实现日志级别统计后端 FFI 接口，支持 Flutter 通过 FFI 获取每个日志级别的记录数量。

## Completed Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 添加 Rust FFI 命令 get_log_level_stats | 84757a0 | commands_bridge.rs, types.rs, bridge.rs |
| 2 | 在 bridge.rs 中导出新命令 | 84757a0 | bridge.rs |
| 3 | 在 BridgeService 中添加调用方法 | f682284 | bridge_service.dart |
| 4 | 创建 LogLevelStats Riverpod Provider | 18a06fe | log_level_stats_provider.dart |

## Implementation Details

### Rust FFI Layer
- Added `LogLevelStatsOutput` type to `ffi/types.rs`
- Added `ffi_get_log_level_stats` function in `commands_bridge.rs`
- Exports `get_log_level_stats` via `bridge.rs` using flutter_rust_bridge

### Flutter BridgeService
- Added `getLogLevelStats(String workspaceId)` method
- Returns `Map<String, dynamic>?` to decouple from generated FFI types

### Flutter Provider
- Created `LogLevelStats` data model
- Created `LogLevelStatsNotifier` with:
  - Initial load via `Future.microtask`
  - 5-second auto-refresh via `Timer.periodic`
  - `refresh()` method for manual refresh
  - `stopAutoRefresh()` / `startAutoRefresh()` for timer control

## Verification

- [x] Rust compiles: `cargo check --manifest-path log-analyzer/src-tauri/Cargo.toml`
- [x] Flutter provider analyzes: `flutter analyze lib/shared/providers/log_level_stats_provider.dart`

## Success Criteria

- [x] Flutter 可以调用 BridgeService.getLogLevelStats(workspaceId) 获取统计
- [x] 返回每个日志级别 (FATAL, ERROR, WARN, INFO, DEBUG, TRACE, UNKNOWN) 的数量
- [x] LogLevelStatsProvider 支持 refresh() 方法手动刷新
- [x] LogLevelStatsProvider 支持 5 秒自动刷新 (STATS-03)

## Deviations

None - plan executed exactly as written.

## Notes

- FFI code generation requires `flutter_rust_bridge_codegen` tool to regenerate Dart bindings
- The BridgeService returns Map to avoid direct dependency on generated FFI types
- Provider uses family pattern (workspaceId parameter) for multi-workspace support
