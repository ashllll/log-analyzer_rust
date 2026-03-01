---
phase: 02-workspace-import
plan: 02
subsystem: flutter-frontend
tags:
  - frontend
  - import
  - drag-drop
  - ui-components
dependency_graph:
  requires:
    - FILE-01
    - FILE-07
  provides:
    - import_progress_provider
    - drop_zone
    - import_progress_dialog
  affects:
    - workspaces_page
tech-stack:
  added:
    - desktop_drop: ^0.4.0
    - Riverpod 3.0 (existing)
  patterns:
    - StateNotifier pattern for import progress
    - DropTarget wrapper component
    - Modal progress dialog
key-files:
  created:
    - log-analyzer_flutter/lib/shared/providers/import_progress_provider.dart
    - log-analyzer_flutter/lib/shared/widgets/drop_zone.dart
    - log-analyzer_flutter/lib/shared/widgets/import_progress_dialog.dart
  modified:
    - log-analyzer_flutter/pubspec.yaml
    - log-analyzer_flutter/lib/features/workspace/presentation/workspaces_page.dart
    - log-analyzer_flutter/test/import/import_progress_provider_test.dart
    - log-analyzer_flutter/test/widgets/import_progress_dialog_test.dart
decisions:
  - Used double (0.0-1.0) for progressPercent for better precision
  - Riverpod 3.x overrideWithValue for test overrides
  - desktop_drop for cross-platform drag-drop support
---

# Phase 02 Plan 02: 文件夹导入功能 (拖放 + 进度显示)

## 摘要

实现了文件夹导入功能，包括拖放支持和导入进度显示。用户可以通过拖放文件夹到窗口或点击导入按钮来导入文件夹到工作区。

## 完成的任务

| Task | Name | Commit | Status |
|------|------|--------|--------|
| 1 | 添加 desktop_drop 依赖 | 61ae32b | Done |
| 2 | 创建导入进度 Provider | df66040 | Done |
| 3 | 创建拖放区域组件 | 5dcc48a | Done |
| 4 | 创建导入进度对话框 | 2405c85 | Done |
| 5 | 集成拖放到工作区页面 | 3e66e2b | Done |

## 功能详情

### 1. desktop_drop 依赖
- 添加了 `desktop_drop: ^0.4.0` 到 pubspec.yaml
- 支持桌面应用拖放文件功能

### 2. ImportProgressProvider
- `ImportProgressState` 数据类：跟踪总文件数、已处理文件、当前文件、进度百分比、错误列表、状态
- `ImportStatus` 枚举：idle, importing, paused, completed, cancelled, failed
- `ImportProgressNotifier` 方法：
  - `startImport(taskId, totalFiles)` - 开始导入
  - `updateProgress(...)` - 更新进度
  - `pauseImport()` - 暂停
  - `resumeImport()` - 继续
  - `cancelImport()` - 取消
  - `completeImport()` - 完成
  - `failImport(error)` - 失败

### 3. DropZoneWidget
- 使用 desktop_drop 实现 DropTarget
- 支持拖入/拖出/完成事件
- 视觉反馈：高亮边框和背景色变化
- 文件过滤：支持扩展名过滤或仅文件夹
- 回调 `onFilesDropped` 传递文件路径列表

### 4. ImportProgressDialog
- 圆形进度指示器显示进度
- 已处理/总文件数
- 当前处理的文件名
- 处理速度 (文件/秒)
- 预估剩余时间
- 取消按钮
- 暂停/继续按钮
- 错误列表显示
- 完成状态显示摘要

### 5. Workspace 页面集成
- AppBar 添加导入按钮
- 拖放区域包裹工作区列表
- 拖放后弹出工作区选择对话框
- 导入流程：选择工作区 -> 显示进度 -> 更新状态

## 测试结果

- `import_progress_provider_test.dart`: 4 个测试全部通过
- `import_progress_dialog_test.dart`: 3 个测试全部通过

## 偏差

无 - 计划按预期执行。

## Self-Check

- [x] desktop_drop 已添加到 pubspec.yaml
- [x] ImportProgressProvider 已创建并通过测试
- [x] DropZoneWidget 已创建
- [x] ImportProgressDialog 已创建并通过测试
- [x] 工作区页面已集成拖放功能
- [x] 所有任务已提交
