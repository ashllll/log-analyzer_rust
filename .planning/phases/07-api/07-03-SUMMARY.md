---
phase: 07-api
plan: 03
subsystem: ffi
tags:
  - ffi
  - regex
  - search
  - flutter-bridge
requires:
  - 07-01 (Search History FFI)
  - 07-02 (Virtual File Tree FFI)
provides:
  - FFI regex validation (validate_regex)
  - FFI regex search (search_regex)
affects:
  - Flutter search UI
  - Advanced search features
tech-stack:
  added:
    - regex crate (existing)
  patterns:
    - FFI sync functions with #[frb(sync)]
    - Result unwrapping with context messages
key-files:
  created: []
  modified:
    - log-analyzer/src-tauri/src/ffi/types.rs
    - log-analyzer/src-tauri/src/ffi/commands_bridge.rs
    - log-analyzer/src-tauri/src/ffi/bridge.rs
    - log-analyzer_flutter/lib/shared/services/bridge_service.dart
decisions:
  - Reuse existing SearchResultEntry type for regex search results
  - Follow existing FFI patterns with sync functions
  - Support case-sensitive and case-insensitive regex modes
metrics:
  duration: 8 min
  tasks: 4
  files: 4
  completed_at: "2026-03-04T15:30:00.000Z"
---

# Phase 7 Plan 3: Regex Search FFI Bridge Summary

扩展 Flutter 与 Rust 后端的 FFI 桥接，实现正则表达式搜索功能。

## 一句话描述

实现了 `validate_regex` 和 `search_regex` 两个 FFI 函数，让 Flutter 应用能够通过 FFI 调用后端的正则表达式验证和搜索 API。

## 任务完成情况

| Task | 名称                    | 状态 | Commit  |
| ---- | ----------------------- | ---- | ------- |
| 1    | 添加正则搜索 FFI 类型定义 | Done | 3c8c463 |
| 2    | 实现正则搜索 FFI 适配层   | Done | 65b68d6 |
| 3    | 添加正则搜索 FFI 导出函数 | Done | 91910c2 |
| 4    | 添加 Flutter 桥接服务方法 | Done | a9824c1 |

## 关键变更

### 1. FFI 类型定义 (`ffi/types.rs`)

新增 `RegexValidationResult` 类型用于正则验证结果:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegexValidationResult {
    pub valid: bool,
    pub error_message: Option<String>,
}
```

复用已有的 `SearchResultEntry` 类型用于搜索结果。

### 2. FFI 适配函数 (`ffi/commands_bridge.rs`)

新增两个 FFI 适配函数:

- `ffi_validate_regex(pattern: String) -> RegexValidationResult`
  - 验证正则表达式语法是否有效
  - 返回验证结果和可能的错误信息

- `ffi_search_regex(pattern, workspace_id, max_results, case_sensitive) -> Result<Vec<SearchResultEntry>, String>`
  - 在工作区中搜索匹配正则表达式的行
  - 支持大小写敏感/不敏感模式
  - 返回包含行号、内容、匹配位置的结果列表

### 3. FFI 导出函数 (`ffi/bridge.rs`)

新增两个 FFI 导出函数:

```rust
#[frb(sync)]
pub fn validate_regex(pattern: String) -> RegexValidationResult

#[frb(sync)]
pub fn search_regex(
    pattern: String,
    workspace_id: Option<String>,
    max_results: i32,
    case_sensitive: bool,
) -> Vec<SearchResultEntry>
```

### 4. Flutter 桥接服务 (`bridge_service.dart`)

新增两个桥接方法:

```dart
Future<ffi.RegexValidationResult> validateRegex(String pattern)

Future<List<ffi.SearchResultEntry>> searchRegex({
  required String pattern,
  String? workspaceId,
  int maxResults = 10000,
  bool caseSensitive = false,
})
```

## 技术要点

1. **正则表达式处理**: 使用 Rust `regex` crate 进行正则编译和匹配
2. **大小写不敏感**: 通过 `(?i)` 前缀实现
3. **性能优化**: 在搜索前先验证正则表达式有效性
4. **错误处理**: 遵循现有 FFI 模式，使用 `unwrap_result` 处理错误

## 偏离计划情况

无偏离 - 计划完全按预期执行。

## 后续步骤

1. **运行 FRB 代码生成**: 需要运行 `flutter_rust_bridge` 代码生成以创建 Dart 端的类型定义
2. **集成测试**: 在 Flutter UI 中集成正则搜索功能
3. **性能测试**: 测试大量文件的正则搜索性能

## 验证命令

```bash
# Rust 编译验证
cd log-analyzer/src-tauri && cargo check --lib

# Flutter 代码生成后验证
cd log-analyzer_flutter && flutter analyze lib/shared/services/bridge_service.dart
```

## Commits

- `3c8c463`: feat(07-03): add RegexValidationResult FFI type definition
- `65b68d6`: feat(07-03): implement regex search FFI adapter functions
- `91910c2`: feat(07-03): add regex search FFI export functions
- `a9824c1`: feat(07-03): add Flutter regex search bridge service methods
