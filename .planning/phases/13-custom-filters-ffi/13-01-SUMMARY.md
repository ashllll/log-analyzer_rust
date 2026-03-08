---
phase: 13-custom-filters-ffi
plan: "01"
subsystem: FFI/Bridge
tags: [ffi, flutter, rust, filters, bridge]
dependency_graph:
  requires:
    - FILTER-01
    - FILTER-02
    - FILTER-03
    - FILTER-05
  provides:
    - SavedFilter CRUD via FFI
  affects:
    - Phase 14: 过滤器 UI
tech_stack:
  added:
    - Rust: SavedFilterData, SavedFilterInput FFI types
    - Rust: Filter CRUD FFI commands
    - Flutter: SavedFilter, SearchTerm, TimeRange models
    - Flutter: BridgeService filter methods
    - Flutter: SavedFilters Riverpod Provider
key_files:
  created:
    - log-analyzer_flutter/lib/shared/models/saved_filter.dart
    - log-analyzer_flutter/lib/shared/providers/saved_filters_provider.dart
  modified:
    - log-analyzer/src-tauri/src/ffi/types.rs
    - log-analyzer/src-tauri/src/ffi/commands_bridge.rs
    - log-analyzer/src-tauri/src/ffi/bridge.rs
    - log-analyzer_flutter/lib/shared/services/bridge_service.dart
decisions:
  - 使用 JSON 配置文件存储过滤器（filters.json）
  - 使用 workspace_id + name 作为过滤器唯一键
  - FFI 类型使用 snake_case 命名，与 Rust 保持一致
  - Flutter 模型支持与 Rust FFI 的双向转换
---

# Phase 13 Plan 01: 自定义过滤器后端 FFI 接口

## 执行摘要

实现了自定义过滤器的后端 FFI 接口，使 Flutter 应用能够通过 FFI 调用 Rust 后端的过滤器 CRUD 接口。为 Phase 14 的过滤器 UI 提供了完整的后端支持。

## 完成的任务

| 任务 | 名称 | 提交哈希 | 文件 |
|------|------|----------|------|
| 1 | 定义 Rust 后端 SavedFilter FFI 类型 | 6bc5515 | ffi/types.rs |
| 2 | 实现 Rust 后端过滤器 FFI 命令 | 3b42669 | ffi/commands_bridge.rs |
| 3 | 暴露过滤器 FFI 接口到 bridge.rs | 55f5b90 | ffi/bridge.rs |
| 4 | 创建 Flutter SavedFilter 数据模型 | ae07027 | shared/models/saved_filter.dart |
| 5 | 在 BridgeService 中添加过滤器方法 | 03f05b2 | shared/services/bridge_service.dart |
| 6 | 创建 SavedFilters Riverpod Provider | 77b9e73 | shared/providers/saved_filters_provider.dart |

## 实现详情

### Rust 后端 (FFI)

**FFI 类型 (ffi/types.rs)**
- `SavedFilterInput`: 用于创建/更新过滤器的输入结构
- `SavedFilterData`: 用于从后端获取过滤器数据的输出结构
- 字段包括: id, name, description, workspace_id, terms_json, global_operator, time_range_start/end, levels_json, file_pattern, is_default, sort_order, usage_count, created_at, last_used_at

**FFI 命令 (ffi/commands_bridge.rs)**
- `ffi_save_filter`: 保存或更新过滤器（基于 workspace_id + name 唯一键）
- `ffi_get_saved_filters`: 获取工作区的所有过滤器
- `ffi_delete_filter`: 删除指定过滤器
- `ffi_update_filter_usage`: 更新过滤器使用统计

**FFI 桥接 (ffi/bridge.rs)**
- 注册了四个同步 FFI 函数供 Flutter 调用

### Flutter 前端

**数据模型 (models/saved_filter.dart)**
- `SearchTerm`: 搜索条件模型
- `TimeRange`: 时间范围模型
- `SavedFilter`: 主过滤器模型，包含与 Rust FFI 的双向转换方法

**BridgeService (services/bridge_service.dart)**
- `saveFilter(SavedFilter)`: 保存过滤器
- `getSavedFilters(String workspaceId, {int? limit})`: 获取过滤器列表
- `deleteFilter(String filterId, String workspaceId)`: 删除过滤器
- `updateFilterUsage(String filterId, String workspaceId)`: 更新使用统计

**Riverpod Provider (providers/saved_filters_provider.dart)**
- `SavedFiltersNotifier`: AsyncNotifier 管理过滤器状态
- 支持 workspaceId 参数化
- 乐观更新模式，后端同步
- 自动回滚机制

## 验证

- [x] Rust 编译通过: `cargo check --lib`
- [x] Flutter 模型分析通过: `flutter analyze lib/shared/models/saved_filter.dart`
- [x] Riverpod 代码生成: `build_runner` 成功生成 `.g.dart` 文件

## 偏差说明

无重大偏差。计划执行与设计一致。

## 后续工作

- Phase 14: 过滤器 UI 实现（使用 SavedFilters Provider）
- 需要运行 flutter_rust_bridge 代码生成器以生成新的 FFI 类型

## 自检

- [x] 所有任务已完成并提交
- [x] 提交哈希已记录在 SUMMARY.md
- [x] Rust 编译通过
- [x] Flutter 分析通过（saved_filter.dart）

## Self-Check: PASSED
