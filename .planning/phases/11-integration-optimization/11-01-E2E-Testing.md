---
wave: 1
depends_on: []
autonomous: true
files_modified:
  - log-analyzer_flutter/test/
  - log-analyzer_flutter/test/...
---

# Plan 11-01: 端到端测试覆盖

## Goal
为所有核心功能创建端到端测试覆盖，确保每个关键功能路径可验证

## Requirement IDs
- INT-01: 端到端测试覆盖

## Context
- Phase 9 (高级搜索 UI) 和 Phase 10 (虚拟文件系统 UI) 已完成基础实现
- 现有测试仅覆盖部分 Provider 和 Widget
- 需要全面测试覆盖：高级搜索、搜索历史、文件树导航、文件预览

## Decisions
- 测试类型: Widget Test + 集成测试
- 数据准备: 纯 Mock 数据（不依赖真实文件）
- 覆盖范围: 核心路径测试（关键操作路径必须通过）

## Tasks

### T1: 创建测试基础设施
- [ ] 创建 `test/shared/mocks/` 目录
- [ ] 创建 MockBridgeService 模拟 FFI 通信
- [ ] 创建 MockWorkspaceProvider 模拟工作区状态
- [ ] 配置 test/ widget_test.dart 全局配置

### T2: 搜索功能测试
- [ ] 测试 SearchQueryProvider 状态管理
- [ ] 测试正则表达式搜索模式切换
- [ ] 测试 AND/OR/NOT 关键词组合
- [ ] 测试搜索结果展示

### T3: 搜索历史测试
- [ ] 测试 SearchHistoryProvider CRUD 操作
- [ ] 测试历史记录保存和读取
- [ ] 测试历史记录删除和清空

### T4: 虚拟文件树测试
- [ ] 测试 VirtualFileTreeProvider 状态
- [ ] 测试目录展开/折叠行为
- [ ] 测试文件点击预览功能

### T5: 集成测试
- [ ] 创建 search_integration_test.dart 搜索流程测试
- [ ] 创建 file_tree_integration_test.dart 文件树测试
- [ ] 创建 workflow_integration_test.dart 端到端工作流测试

## Verification
- [ ] 所有核心 Provider 有单元测试覆盖
- [ ] 所有核心 Widget 有 Widget Test 覆盖
- [ ] 集成测试覆盖关键用户路径
- [ ] 测试通过率 100%

## Must-Haves
- 每个核心功能至少有 3 个测试用例
- Mock 数据覆盖正常路径和异常路径
- 测试文档说明测试覆盖范围
