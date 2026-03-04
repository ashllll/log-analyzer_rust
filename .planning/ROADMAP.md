# Roadmap: Flutter 日志分析桌面应用

## Overview

使用 Flutter 全新实现的桌面日志分析应用，通过 FFI 与现有 Rust 后端通信。路线图从架构基础设施开始，逐步构建工作区管理、文件导入、核心搜索功能、压缩包管理和实时监控能力，最终完成用户体验优化。

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: 架构基础设施** - Flutter 项目结构、共享服务、错误处理框架
- [ ] **Phase 2: 工作区与文件导入** - 工作区管理、文件夹/压缩包导入
- [ ] **Phase 3: 搜索功能与结果展示** - 全文搜索、关键词高亮、多条件筛选
<<<<<<< HEAD
- [ ] **Phase 4: 压缩包浏览** - 压缩包内文件浏览、预览、内嵌搜索
- [ ] **Phase 5: 实时监控** - 文件监控、自动索引更新
=======
- [x] **Phase 4: 压缩包浏览** - 压缩包内文件浏览、预览、内嵌搜索
- [x] **Phase 5: 实时监控** - 文件监控、自动索引更新
>>>>>>> gsd/phase-06-completion
- [ ] **Phase 6: 完成与优化** - 任务进度、设置、用户体验完善

## Phase Details

### Phase 1: 架构基础设施
**Goal**: Flutter 应用具备与 Rust 后端通信的基础设施，包括项目结构、共享服务、错误处理框架
**Depends on**: Nothing (first phase)
**Success Criteria** (what must be TRUE):
  1. Flutter 桌面应用可以正常启动并显示主窗口
  2. ApiService 能够与 Rust 后端 FFI 通信
  3. BridgeService 能够通过 FFI 与 Rust 后端通信
  4. 错误处理框架能够显示用户友好的错误信息
  5. Riverpod Provider 基础配置完成并正常工作
**Plans**: 4 plans

Plans:
- [x] 01-01-PLAN.md — FFI 桥接服务重构（纯 FFI 模式）
- [x] 01-02-PLAN.md — 错误处理框架（错误码 + ErrorView）
- [x] 01-03-PLAN.md — 启动流程与路由（Splash Screen + go_router）
- [x] 01-04-PLAN.md — Riverpod Provider 基础配置验证

### Phase 2: 工作区与文件导入
**Goal**: 用户可以管理工作区、导入文件和压缩包
**Depends on**: Phase 1
**Requirements**: WORK-01, WORK-02, WORK-03, WORK-04, FILE-01, FILE-02, FILE-03, FILE-04, FILE-05, FILE-06, FILE-07
**Success Criteria** (what must be TRUE):
  1. 用户可以创建新的工作区
  2. 用户可以打开已有工作区
  3. 用户可以删除工作区
  4. 用户可以查看工作区状态 (文件数、索引状态)
  5. 用户可以导入文件夹
  6. 用户可以导入 ZIP/TAR/GZ/RAR/7Z 压缩包
  7. 文件导入时显示进度
**Plans**: 3 plans

Plans:
- [x] 02-01-PLAN.md — 工作区增强（键盘导航、最近优先排序）
- [x] 02-02-PLAN.md — 文件导入（拖放支持、导入进度显示）
- [x] 02-03-PLAN.md — 压缩包导入（ZIP/TAR/GZ/RAR/7Z 支持）

### Phase 3: 搜索功能与结果展示
**Goal**: 用户可以搜索日志并查看结果，具备关键词高亮和多条件筛选能力
**Depends on**: Phase 2
**Requirements**: SEARCH-01, SEARCH-02, SEARCH-03, SEARCH-04, SEARCH-05, SEARCH-06, UI-01, UI-02, UI-03
**Success Criteria** (what must be TRUE):
  1. 用户可以输入关键词进行全文搜索
  2. 搜索结果中高亮显示匹配的关键词
  3. 用户可以按日期范围筛选搜索结果
<<<<<<< HEAD
  4. 用户可以按日志级别筛选 (ERROR, WARN, INFO, DEBUG)
  5. 用户可以按文件类型筛选
=======
  4. 用户可以按日志级别筛选 (ERROR, WARN, INFO, DEBUG) — 用户选择不实现
  5. 用户可以按文件类型筛选 — 用户选择不实现
>>>>>>> gsd/phase-06-completion
  6. 搜索响应时间 <200ms
  7. 用户可以看到搜索结果列表
  8. 用户可以查看单条日志详情
  9. 用户可以查看任务进度
<<<<<<< HEAD
**Plans**: TBD
=======
**Plans**: 4 plans

Plans:
- [x] 03-01-PLAN.md — 搜索增强（进度条+日期选择器+快捷键） ✓
- [x] 03-02-PLAN.md — 结果展示（详情面板+关键词高亮） ✓
>>>>>>> gsd/phase-06-completion

### Phase 4: 压缩包浏览
**Goal**: 用户可以浏览压缩包内的文件、预览文本文件内容、在压缩包内搜索
**Depends on**: Phase 3
**Requirements**: ARCH-01, ARCH-02, ARCH-03
**Success Criteria** (what must be TRUE):
  1. 用户可以浏览压缩包内的文件列表
  2. 用户可以预览压缩包内的文本文件内容
  3. 用户可以在压缩包内搜索关键词
<<<<<<< HEAD
**Plans**: TBD
=======
**Plans**: 2 plans

Plans:
- [x] 04-01-PLAN.md — 后端实现（ArchiveHandler 扩展 + Tauri 命令）
- [x] 04-02-PLAN.md — 前端实现（树形视图 + 预览面板 + 搜索）
>>>>>>> gsd/phase-06-completion

### Phase 5: 实时监控
**Goal**: 用户可以启用文件监控，文件变化时自动更新索引
**Depends on**: Phase 4
**Requirements**: MON-01, MON-02, MON-03
**Success Criteria** (what must be TRUE):
  1. 用户可以启用文件监控
  2. 文件变化时自动更新索引
  3. 用户可以查看监控状态
<<<<<<< HEAD
**Plans**: TBD
=======
**Plans**: 2 plans

Plans:
- [x] 05-01-PLAN.md — 核心实现（MonitoringState + MonitoringProvider）
- [x] 05-02-PLAN.md — UI实现（工具栏按钮 + 状态面板）
>>>>>>> gsd/phase-06-completion

### Phase 6: 完成与优化
**Goal**: 完善用户体验，提供设置功能，应用可以正常启动
**Depends on**: Phase 5
**Requirements**: UI-04
**Success Criteria** (what must be TRUE):
  1. 应用程序可以正常启动
  2. 用户可以访问设置/配置界面
  3. 所有核心功能可用且稳定
<<<<<<< HEAD
**Plans**: TBD
=======
**Plans**: 2 plans

Plans:
- [x] 06-01-PLAN.md — 设置基础设施（SettingsService + ThemeProvider + 页面重构）
- [x] 06-02-PLAN.md — 启动恢复 + UX优化（Splash工作区恢复 + 空状态组件）
>>>>>>> gsd/phase-06-completion

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. 架构基础设施 | 4/4 | Completed | 2026-02-28 |
| 2. 工作区与文件导入 | 3/3 | Completed | 2026-03-01 |
<<<<<<< HEAD
| 3. 搜索功能与结果展示 | 0/N | Not started | - |
| 4. 压缩包浏览 | 0/N | Not started | - |
| 5. 实时监控 | 0/N | Not started | - |
| 6. 完成与优化 | 0/N | Not started | - |
=======
| 3. 搜索功能与结果展示 | 2/4 | Completed | - |
| 4. 压缩包浏览 | 2/2 | Completed | 2026-03-02 |
| 5. 实时监控 | 2/2 | Completed | 2026-03-03 |
| 6. 完成与优化 | 2/2 | Completed | 2026-03-03 |
>>>>>>> gsd/phase-06-completion

## Coverage

### Requirement Mapping

| Requirement | Phase | Status |
|-------------|-------|--------|
<<<<<<< HEAD
| WORK-01 | Phase 2 | Pending |
| WORK-02 | Phase 2 | Pending |
| WORK-03 | Phase 2 | Pending |
| WORK-04 | Phase 2 | Pending |
| FILE-01 | Phase 2 | Pending |
| FILE-02 | Phase 2 | Pending |
| FILE-03 | Phase 2 | Pending |
| FILE-04 | Phase 2 | Pending |
| FILE-05 | Phase 2 | Pending |
| FILE-06 | Phase 2 | Pending |
| FILE-07 | Phase 2 | Pending |
| SEARCH-01 | Phase 3 | Pending |
| SEARCH-02 | Phase 3 | Pending |
| SEARCH-03 | Phase 3 | Pending |
| SEARCH-04 | Phase 3 | Pending |
| SEARCH-05 | Phase 3 | Pending |
| SEARCH-06 | Phase 3 | Pending |
| ARCH-01 | Phase 4 | Pending |
| ARCH-02 | Phase 4 | Pending |
| ARCH-03 | Phase 4 | Pending |
| MON-01 | Phase 5 | Pending |
| MON-02 | Phase 5 | Pending |
| MON-03 | Phase 5 | Pending |
| UI-01 | Phase 3 | Pending |
| UI-02 | Phase 3 | Pending |
| UI-03 | Phase 3 | Pending |
=======
| WORK-01 | Phase 2 | Done |
| WORK-02 | Phase 2 | Done |
| WORK-03 | Phase 2 | Done |
| WORK-04 | Phase 2 | Done |
| FILE-01 | Phase 2 | Done |
| FILE-02 | Phase 2 | Done |
| FILE-03 | Phase 2 | Done |
| FILE-04 | Phase 2 | Done |
| FILE-05 | Phase 2 | Done |
| FILE-06 | Phase 2 | Done |
| FILE-07 | Phase 2 | Done |
| SEARCH-01 | Phase 3 | Done |
| SEARCH-02 | Phase 3 | Done |
| SEARCH-03 | Phase 3 | Done |
| SEARCH-04 | Phase 3 | Deferred |
| SEARCH-05 | Phase 3 | Deferred |
| SEARCH-06 | Phase 3 | Done |
| ARCH-01 | Phase 4 | Done |
| ARCH-02 | Phase 4 | Done |
| ARCH-03 | Phase 4 | Done |
| MON-01 | Phase 5 | Done |
| MON-02 | Phase 5 | Done |
| MON-03 | Phase 5 | Done |
| UI-01 | Phase 3 | Done |
| UI-02 | Phase 3 | Done |
| UI-03 | Phase 3 | Done |
>>>>>>> gsd/phase-06-completion
| UI-04 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 25 total
- Mapped to phases: 25
- Unmapped: 0 ✓

---

*Roadmap created: 2026-02-28*
*Ready for planning: yes*
