# Roadmap: Flutter 日志分析桌面应用

## Milestones

- [x] **v1.0 MVP** - Phases 1-6 (已交付 2026-03-01)
- [ ] **v1.1 高级搜索与虚拟文件系统** - Phases 7-11 (进行中)
- [ ] **v2.0 [待定]** - Phases 12+ (计划中)

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>v1.0 MVP (Phases 1-6) - SHIPPED 2026-03-01</summary>

### Phase 1: 架构基础设施
**Goal**: Flutter 应用具备与 Rust 后端通信的基础设施，包括项目结构、共享服务、错误处理框架
**Plans**: 4 plans

Plans:
- [x] 01-01 — FFI 桥接服务重构（纯 FFI 模式）
- [x] 01-02 — 错误处理框架（错误码 + ErrorView）
- [x] 01-03 — 启动流程与路由（Splash Screen + go_router）
- [x] 01-04 — Riverpod Provider 基础配置验证

### Phase 2: 工作区与文件导入
**Goal**: 用户可以管理工作区、导入文件和压缩包
**Plans**: 3 plans

Plans:
- [x] 02-01 — 工作区增强（键盘导航、最近优先排序）
- [x] 02-02 — 文件导入（拖放支持、导入进度显示）
- [x] 02-03 — 压缩包导入（ZIP/TAR/GZ/RAR/7Z 支持）

### Phase 3: 搜索功能与结果展示
**Goal**: 用户可以搜索日志并查看结果，具备关键词高亮和多条件筛选能力
**Plans**: 2 plans

Plans:
- [x] 03-01 — 搜索增强（进度条+日期选择器+快捷键）
- [x] 03-02 — 结果展示（详情面板+关键词高亮）

### Phase 4: 压缩包浏览
**Goal**: 用户可以浏览压缩包内的文件、预览文本文件内容、在压缩包内搜索
**Plans**: 2 plans

Plans:
- [x] 04-01 — 后端实现（ArchiveHandler 扩展 + Tauri 命令）
- [x] 04-02 — 前端实现（树形视图 + 预览面板 + 搜索）

### Phase 5: 实时监控
**Goal**: 用户可以启用文件监控，文件变化时自动更新索引
**Plans**: 2 plans

Plans:
- [x] 05-01 — 核心实现（MonitoringState + MonitoringProvider）
- [x] 05-02 — UI实现（工具栏按钮 + 状态面板）

### Phase 6: 完成与优化
**Goal**: 完善用户体验，提供设置功能，应用可以正常启动
**Plans**: 2 plans

Plans:
- [x] 06-01 — 设置基础设施（SettingsService + ThemeProvider + 页面重构）
- [x] 06-02 — 启动恢复 + UX优化（Splash工作区恢复 + 空状态组件）

</details>

### v1.1 高级搜索与虚拟文件系统 (Phases 7-11)

**Milestone Goal:** 实现高级搜索功能（正则表达式、多关键词组合、搜索历史）和虚拟文件系统（文件树、目录导航）

---

- [ ] **Phase 7: 后端 API 集成** - 扩展 API 服务，支持搜索历史和虚拟文件树
- [ ] **Phase 8: 状态管理** - 实现 SearchHistoryProvider 和 VirtualFileTreeProvider
- [ ] **Phase 9: 高级搜索 UI** - 正则表达式、多关键词组合、搜索历史界面
- [ ] **Phase 10: 虚拟文件系统 UI** - 文件树导航、目录展开折叠、文件预览
- [ ] **Phase 11: 集成与优化** - 功能联动、键盘导航、性能优化

---

## Phase Details

### Phase 7: 后端 API 集成
**Goal**: Flutter 应用能够通过 FFI 调用 Rust 后端的搜索历史和虚拟文件树 API
**Depends on**: Nothing (first phase of v1.1)
**Success Criteria** (what must be TRUE):
  1. ApiService 扩展了搜索历史相关方法 (add, get, delete, clear)
  2. ApiService 扩展了虚拟文件树获取方法
  3. 正则表达式搜索功能可在 Flutter 端调用后端
  4. 多关键词组合搜索 (AND/OR/NOT) 可在后端执行
**Plans**: 4 plans

Plans:
- [x] 07-01-PLAN.md — 搜索历史 API 集成（添加、获取、删除、清空） ✓ 2026-03-04
- [x] 07-02-PLAN.md — 虚拟文件树 API 集成（获取树结构） ✓ 2026-03-04
- [ ] 07-03-PLAN.md — 正则表达式搜索 API 集成
- [ ] 07-04-PLAN.md — 多关键词组合搜索 API 集成

### Phase 8: 状态管理
**Goal**: 使用 Riverpod 管理搜索历史和虚拟文件树的状态
**Depends on**: Phase 7
**Success Criteria** (what must be TRUE):
  1. SearchHistoryProvider 可以增删改查搜索历史
  2. VirtualFileTreeProvider 可以获取和刷新文件树
  3. 历史记录支持 LRU 限制（最多100条）
  4. 虚拟文件树支持懒加载
**Plans**: TBD

Plans:
- [ ] 08-01: SearchHistoryProvider 实现（CRUD + LRU 限制）
- [ ] 08-02: VirtualFileTreeProvider 实现（懒加载支持）

### Phase 9: 高级搜索 UI
**Goal**: 用户可以使用正则表达式搜索、多关键词组合、查看搜索历史
**Depends on**: Phase 8
**Requirements**: ASEARCH-01, ASEARCH-02, ASEARCH-03, ASEARCH-04, ASEARCH-05, ASEARCH-06, HIST-01, HIST-02, HIST-03, HIST-04, HIST-05
**Success Criteria** (what must be TRUE):
  1. 用户可以切换到正则表达式搜索模式
  2. 正则表达式搜索时显示语法有效/无效反馈
  3. 用户可以输入多个关键词并选择 AND 组合
  4. 用户可以输入多个关键词并选择 OR 组合
  5. 用户可以输入多个关键词并选择 NOT 组合
  6. 用户可以查看组合后的搜索条件预览
  7. 搜索自动保存到历史记录
  8. 用户可以在下拉列表中查看历史搜索记录
  9. 用户可以点击历史记录快速填充搜索框
  10. 用户可以删除单条历史记录
  11. 用户可以清空所有搜索历史
**Plans**: TBD

Plans:
- [ ] 09-01: SearchInputBar 组件（正则模式切换、语法反馈）
- [ ] 09-02: 关键词组合 UI（AND/OR/NOT 选择器、条件预览）
- [ ] 09-03: SearchHistoryPanel 组件（历史列表、点击填充）
- [ ] 09-04: 历史管理（删除单条、清空全部）

### Phase 10: 虚拟文件系统 UI
**Goal**: 用户可以浏览虚拟文件树、展开目录、预览文件内容
**Depends on**: Phase 8
**Requirements**: VFS-01, VFS-02, VFS-03, VFS-04
**Success Criteria** (what must be TRUE):
  1. 用户可以查看工作区的虚拟文件树结构
  2. 目录节点可以展开/折叠
  3. 用户可以点击文件预览内容
  4. 文件树显示文件/目录图标区分
**Plans**: TBD

Plans:
- [ ] 10-01: VirtualFileTreeView 组件（文件树展示、图标区分）
- [ ] 10-02: 目录展开/折叠功能
- [ ] 10-03: 文件预览面板

### Phase 11: 集成与优化
**Goal**: 高级搜索与虚拟文件系统联动，提供流畅的用户体验
**Depends on**: Phase 9, Phase 10
**Success Criteria** (what must be TRUE):
  1. 搜索结果可以关联到虚拟文件树中的文件
  2. 支持键盘导航（上下箭头 + 回车）
  3. 文件树支持手动刷新
  4. 整体性能优化（懒加载、虚拟滚动）
**Plans**: TBD

Plans:
- [ ] 11-01: 搜索结果与文件树关联
- [ ] 11-02: 键盘导航支持
- [ ] 11-03: 性能优化（懒加载、虚拟滚动）

---

## Progress

**Execution Order:**
Phases execute in numeric order: 7 → 8 → 9 → 10 → 11

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 7. 后端 API 集成 | 2/4 | In Progress | 07-01, 07-02 |
| 8. 状态管理 | 0/2 | Not started | - |
| 9. 高级搜索 UI | 0/4 | Not started | - |
| 10. 虚拟文件系统 UI | 0/3 | Not started | - |
| 11. 集成与优化 | 0/3 | Not started | - |

---

## Coverage

### v1.1 Requirement Mapping

| Requirement | Phase | Status |
|-------------|-------|--------|
| ASEARCH-01 | Phase 9 | Pending |
| ASEARCH-02 | Phase 9 | Pending |
| ASEARCH-03 | Phase 9 | Pending |
| ASEARCH-04 | Phase 9 | Pending |
| ASEARCH-05 | Phase 9 | Pending |
| ASEARCH-06 | Phase 9 | Pending |
| HIST-01 | Phase 9 | Pending |
| HIST-02 | Phase 9 | Pending |
| HIST-03 | Phase 9 | Pending |
| HIST-04 | Phase 9 | Pending |
| HIST-05 | Phase 9 | Pending |
| VFS-01 | Phase 10 | Pending |
| VFS-02 | Phase 10 | Pending |
| VFS-03 | Phase 10 | Pending |
| VFS-04 | Phase 10 | Pending |

**Coverage:**
- v1.1 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0 ✓

---

*Roadmap created: 2026-03-04*
*Ready for planning: yes*
