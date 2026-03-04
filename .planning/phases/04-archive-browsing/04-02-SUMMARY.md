---
phase: 04-archive-browsing
plan: 02
subsystem: archive
tags: [archive, browsing, frontend, flutter]
dependency_graph:
  requires:
    - ARCH-01
    - ARCH-02
    - ARCH-03
  provides:
    - ArchiveBrowserPage
    - ArchiveTreeView
    - ArchivePreviewPanel
    - ArchiveSearchBar
    - ArchiveBrowserProvider
  affects:
    - Backend archive commands (04-01)
tech_stack:
  added:
    - ArchiveNode model
    - ArchiveBrowserProvider (Riverpod)
    - ArchiveTreeView widget
    - ArchivePreviewPanel widget
    - ArchiveSearchBar widget
    - ArchiveBrowserPage (Split Pane)
    - Route configuration
    - Tauri invoke methods
  patterns:
    - Riverpod state management
    - Split Pane layout
    - Real-time search filtering
    - Keyword highlighting
key_files:
  created:
    - log-analyzer_flutter/lib/features/archive_browsing/models/archive_node.dart
    - log-analyzer_flutter/lib/features/archive_browsing/providers/archive_browser_provider.dart
    - log-analyzer_flutter/lib/features/archive_browsing/presentation/pages/archive_browser_page.dart
    - log-analyzer_flutter/lib/features/archive_browsing/presentation/widgets/archive_tree_view.dart
    - log-analyzer_flutter/lib/features/archive_browsing/presentation/widgets/archive_preview_panel.dart
    - log-analyzer_flutter/lib/features/archive_browsing/presentation/widgets/archive_search_bar.dart
  modified:
    - log-analyzer_flutter/lib/shared/services/api_service.dart
    - log-analyzer_flutter/lib/shared/services/bridge_service.dart
    - log-analyzer_flutter/lib/core/router/app_router.dart
decisions:
  - "使用 Riverpod StateNotifier 模式进行状态管理"
  - "使用 Tauri invoke 方法调用后端压缩包命令"
  - "Split Pane 布局：左侧 30% 文件树，右侧 70% 预览"
  - "实时搜索模式：输入即过滤文件列表"
  - "关键词高亮使用黄色背景标记"
metrics:
  duration: ~15 minutes
  completed_date: "2026-03-02"
---

# Phase 04 Plan 02: 压缩包浏览前端实现

## 概述

实现压缩包浏览前端功能：树形视图展示文件列表、Split Pane 布局、文件预览（支持关键词高亮）、实时搜索。

## 完成的任务

| Task | Name | Commit |
|------|------|--------|
| 1 | 创建 ArchiveNode 数据模型 | abc6b32 |
| 2 | 扩展 ApiService 添加压缩包浏览方法 | abc6b32 |
| 3 | 创建 ArchiveBrowserProvider 状态管理 | abc6b32 |
| 4 | 创建 ArchiveTreeView 树形视图组件 | abc6b32 |
| 5 | 创建 ArchivePreviewPanel 预览面板组件 | abc6b32 |
| 6 | 创建 ArchiveSearchBar 搜索栏组件 | abc6b32 |
| 7 | 创建 ArchiveBrowserPage 主页面 | abc6b32 |
| 8 | 添加路由配置 | abc6b32 |

## 实现的功能

### 1. ArchiveNode 数据模型
- 支持目录和文件两种节点类型
- 包含文件名、路径、大小、展开状态等属性
- 提供 `buildTree` 方法将扁平 entries 转换为树形结构

### 2. ApiService 扩展
- `listArchiveContents` - 列出压缩包内容
- `readArchiveFile` - 读取压缩包内文件
- 新增 `ArchiveFileResult` 类型

### 3. ArchiveBrowserProvider
使用 Riverpod StateNotifier 模式：
- `archivePathProvider` - 压缩包路径
- `archiveTreeProvider` - 文件树
- `selectedFileProvider` - 当前选中文件
- `searchKeywordProvider` - 搜索关键词
- `filteredNodesProvider` - 过滤后的文件列表

### 4. ArchiveTreeView
- 递归渲染目录结构
- 支持展开/折叠
- 显示文件大小
- 根据文件类型显示不同图标

### 5. ArchivePreviewPanel
- 支持关键词高亮（黄色背景）
- 大文件截断提示
- 加载状态和错误状态显示
- 空状态友好提示

### 6. ArchiveSearchBar
- 实时搜索模式
- 输入即过滤文件列表
- 清除按钮

### 7. ArchiveBrowserPage
- Split Pane 布局（30% 文件树 + 70% 预览）
- 集成所有子组件
- 路由参数接收压缩包路径

### 8. 路由配置
- 路径：`/archive-browser?path=xxx`
- 使用 go_router 声明式路由

## 验证结果

- flutter analyze 通过（无错误）
- 所有组件已创建并正确配置

## 后续工作

- 用户体验优化
- 集成测试
