# Roadmap: Flutter 日志分析桌面应用

## Milestones

- [x] **v1.0 MVP** - Phases 1-6 (已交付 2026-03-01)
- [x] **v1.1 高级搜索与虚拟文件系统** - Phases 7-8 (已交付 2026-03-05)
- [x] **v1.2 UI 完善** - Phases 9-11 (已交付 2026-03-07)
- [ ] **v1.3 功能扩展** - Phases 12-17 (计划中)
- [ ] **v2.0 [待定]** - Phases 18+ (计划中)

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

<details>
<summary>v1.1 高级搜索与虚拟文件系统 (Phases 7-8) - SHIPPED 2026-03-05</summary>

### Phase 7: 后端 API 集成
**Goal**: Flutter 应用能够通过 FFI 调用 Rust 后端的搜索历史和虚拟文件树 API
**Depends on**: Phase 6
**Requirements**: (v1.1 FFI 桥接需求)
**Success Criteria** (what must be TRUE):
  1. ApiService 扩展了搜索历史相关方法 (add, get, delete, clear)
  2. ApiService 扩展了虚拟文件树获取方法
  3. 正则表达式搜索功能可在 Flutter 端调用后端
  4. 多关键词组合搜索 (AND/OR/NOT) 可在后端执行
**Plans**: 4 plans

Plans:
- [x] 07-01 — 搜索历史 API 集成
- [x] 07-02 — 虚拟文件树 API 集成
- [x] 07-03 — 正则表达式搜索 API 集成
- [x] 07-04 — 多关键词组合搜索 API 集成

### Phase 8: 状态管理
**Goal**: 使用 Riverpod 3.0 AsyncNotifier 管理搜索历史和虚拟文件树的状态，支持参数化工作区、乐观更新、懒加载
**Depends on**: Phase 7
**Requirements**: (v1.1 状态管理需求)
**Success Criteria** (what must be TRUE):
  1. SearchHistoryProvider 可以增删改查搜索历史（CRUD）
  2. SearchHistoryProvider 支持乐观更新和错误回滚
  3. VirtualFileTreeProvider 可以获取文件树根节点
  4. VirtualFileTreeProvider 支持懒加载子节点
  5. 切换工作区时状态自动刷新
**Plans**: 2 plans

Plans:
- [x] 08-01 — SearchHistoryProvider 实现
- [x] 08-02 — VirtualFileTreeProvider 实现

</details>

---

## v1.2 UI 完善 (Phases 9-11)

- [x] **Phase 9: 高级搜索 UI** - 正则表达式、多关键词组合、搜索历史界面 ✅ 2026-03-06
- [x] **Phase 10: 虚拟文件系统 UI** - 文件树导航、目录展开折叠、文件预览
 (completed 2026-03-06)
- [x] **Phase 11: 集成与优化** - 端到端测试、性能优化、用户体验改进
 (completed 2026-03-07)

---

## Phase Details

### Phase 9: 高级搜索 UI
**Goal**: 用户可以使用正则表达式搜索、多关键词组合搜索、并通过搜索历史快速访问过往查询
**Depends on**: Phase 8 (状态管理已就绪)
**Requirements**: ASEARCH-01, ASEARCH-02, ASEARCH-03, ASEARCH-04, ASEARCH-05, ASEARCH-06, HIST-01, HIST-02, HIST-03, HIST-04, HIST-05
**Success Criteria** (what must be TRUE):
  1. 用户可以在搜索栏切换到正则表达式搜索模式
  2. 正则表达式搜索时，输入框下方实时显示语法有效/无效反馈
  3. 用户可以输入多个关键词并选择 AND 逻辑组合
  4. 用户可以输入多个关键词并选择 OR 逻辑组合
  5. 用户可以输入多个关键词并选择 NOT 逻辑组合
  6. 用户可以在执行搜索前查看组合后的完整搜索条件预览
  7. 每次搜索执行后自动保存到搜索历史记录
  8. 用户可以在搜索栏下拉列表中查看历史搜索记录
  9. 用户可以点击历史记录条目快速填充搜索框
  10. 用户可以删除单条历史记录
  11. 用户可以一键清空所有搜索历史
**Plans**: 4 plans

Plans:
- [x] 09-01: SearchInputBar 增强 - 正则模式切换、语法验证反馈 (ASEARCH-01, ASEARCH-02)
- [x] 09-02: 关键词组合 UI - AND/OR/NOT 选择器、条件预览显示 (ASEARCH-03~06)
- [x] 09-03: SearchHistoryDropdown - 历史列表展示、点击快速填充 (HIST-01~03)
- [x] 09-04: 历史管理功能 - 删除单条、清空全部确认 (HIST-04~05)
- [x] 09-05: Gap Closure - 组合搜索组件集成修复

### Phase 10: 虚拟文件系统 UI
**Goal**: 用户可以浏览工作区的虚拟文件树、展开/折叠目录、预览文件内容
**Depends on**: Phase 8 (VirtualFileTreeProvider 已就绪)
**Requirements**: VFS-01, VFS-02, VFS-03, VFS-04
**Success Criteria** (what must be TRUE):
  1. 用户可以在侧边栏查看工作区的虚拟文件树结构
  2. 目录节点显示展开/折叠箭头，点击可切换状态
  3. 用户可以点击文件节点在预览面板中查看文件内容
  4. 文件树使用不同图标区分文件和目录类型
**Plans**: 3 plans

Plans:
- [x] 10-01: VirtualFileTreeView - 树形组件、文件/目录图标区分 (VFS-01, VFS-04)
- [ ] 10-02: 目录展开折叠 - TreeController 集成、懒加载子节点 (VFS-02)
- [ ] 10-03: 文件预览面板 - 内容展示、状态处理 (VFS-03)

### Phase 11: 集成与优化
**Goal**: 确保所有功能端到端可用，性能达标，用户体验流畅
**Depends on**: Phase 9, Phase 10
**Requirements**: INT-01, INT-02, INT-03, INT-04
**Success Criteria** (what must be TRUE):
  1. 每个核心功能（高级搜索、搜索历史、文件树）有端到端测试覆盖
  2. 搜索响应时间 <200ms，文件树首次加载 <500ms
  3. 所有加载状态统一显示（Shimmer/Skeleton），错误处理一致（ErrorView）
  4. 代码审查完成，技术文档更新到最新状态
**Plans**: TBD

Plans:
- [x] 11-01: 端到端测试 - Widget Test 覆盖核心功能
- [x] 11-02: 性能优化 - 搜索响应、文件树懒加载、虚拟滚动
- [x] 11-03: UX 完善 - 加载状态、错误处理、无障碍支持
- [x] 11-04: 文档更新 - 代码审查、技术文档

---

## v1.3 功能扩展 (Phases 12-17)

- [x] **Phase 12:** 多工作区标签页基础设施 (Tabs)
 (completed 2026-03-07)
- [x] **Phase 13:** 自定义过滤器后端 FFI 接口 (Filters)
 (completed 2026-03-08)
- [x] **Phase 14:** 自定义过滤器 UI (Filters)
 (planned 2026-03-08)
- [x] **Phase 15:** 日志级别统计后端 FFI 接口 (Stats) (completed 2026-03-08)
- [x] **Phase 16:** 日志级别统计 UI 面板 (Stats) (completed 2026-03-08)
- [ ] **Phase 17:** 集成与优化 (Integration)

---

## Phase Details

### Phase 12: 多工作区标签页基础设施
**Goal**: 用户可以打开、切换、关闭多个工作区标签页，状态隔离且持久化
**Depends on**: Phase 11
**Requirements**: TAB-01, TAB-02, TAB-03, TAB-04, TAB-05, TAB-06
**Success Criteria** (what must be TRUE):
  1. 用户可以打开新标签页并选择工作区
  2. 用户可以通过点击标签或快捷键切换标签页
  3. 用户可以关闭不需要的标签页
  4. 用户可以拖拽调整标签页顺序
  5. 每个标签页维护独立状态，切换时自动保存/恢复
  6. 标签页列表在会话间持久化
**Plans**: TBD

### Phase 13: 自定义过滤器后端 FFI 接口
**Goal**: Flutter 应用能够通过 FFI 调用 Rust 后端的过滤器 CRUD 接口
**Depends on**: Phase 12
**Requirements**: FILTER-01, FILTER-02, FILTER-03, FILTER-05
**Success Criteria** (what must be TRUE):
  1. 可以创建新过滤器（名称 + 条件组合）
  2. 可以编辑现有过滤器
  3. 可以删除过滤器
  4. 过滤器在工作区级别持久化存储
**Plans**: 1 plan

Plans:
- [x] 13-01 — 过滤器 FFI 接口实现 (FILTER-01, FILTER-02, FILTER-03, FILTER-05)

### Phase 14: 自定义过滤器 UI
**Goal**: 用户可以通过侧边栏和对话框管理过滤器，并在搜索时快速应用
**Depends on**: Phase 13
**Requirements**: FILTER-04
**Success Criteria** (what must be TRUE):
  1. 侧边栏显示过滤器列表
  2. 过滤器创建/编辑对话框
  3. 搜索栏显示过滤器快捷按钮
  4. 点击过滤器自动填充搜索条件
**Plans**: 2 plans

Plans:
- [x] 14-01 — 过滤器 UI 组件实现 (FILTER-04)
- [x] 14-02 — Gap 修复：侧边栏创建功能 + FilterPalette 复用 (FILTER-04)

### Phase 15: 日志级别统计后端 FFI 接口
**Goal**: Flutter 应用能够通过 FFI 调用 Rust 后端获取日志级别统计
**Depends on**: Phase 12
**Requirements**: STATS-01, STATS-03
**Success Criteria** (what must be TRUE):
  1. 可以获取每个日志级别的记录数量
  2. 索引更新时统计数据自动刷新
**Plans**: 1 plan

Plans:
- [ ] 15-01 — 日志级别统计 FFI 接口 (STATS-01, STATS-03)

### Phase 16: 日志级别统计 UI 面板
**Goal**: 用户可以查看日志级别的数量、分布图表，并按级别快速过滤
**Depends on**: Phase 15
**Requirements**: STATS-02, STATS-04, STATS-05
**Success Criteria** (what must be TRUE):
  1. 统计面板显示每个级别的计数
  2. 显示级别分布饼图/条形图
  3. 点击级别可快速筛选对应日志
  4. 数据显示实时更新
**Plans**: 2 plans

Plans:
- [x] 16-01 — LogLevelStatsPanel 组件实现 (STATS-02, STATS-04)
- [x] 16-02 — 搜索页面集成 (STATS-04)

### Phase 17: 集成与优化
**Goal**: 确保所有新功能端到端可用，性能达标
**Depends on**: Phase 14, Phase 16
**Requirements**: T-04, NF-01, NF-02, NF-03
**Success Criteria** (what must be TRUE):
  1. 所有功能端到端测试覆盖
  2. 标签页切换 <100ms，统计加载 <500ms
  3. 内存占用符合预期 (<50MB/标签页)
  4. 与现有功能无冲突
**Plans**: 3 plans

Plans:
- [x] 17-01 — 端到端测试覆盖 (NF-03) [COMPLETED 2026-03-08]
- [x] 17-02 — 性能与内存测试 (T-04, NF-01, NF-02) [COMPLETED 2026-03-08]
- [ ] 17-03 — 兼容性检查与集成验证 (NF-03)

---

## Progress

**Execution Order:**
Phases execute in numeric order: 12 → 13 → 14 → 15 → 16 → 17

| Phase | Plans Complete | Status | Target |
|-------|----------------|--------|--------|
| 12. 多工作区标签页 | 0/ | Complete    | 2026-03-07 |
| 13. 过滤器 FFI | 1/1 | Complete    | 2026-03-08 |
| 14. 过滤器 UI | 2/2 | Complete    | 2026-03-08 |
| 15. 统计 FFI | 1/1 | Complete    | 2026-03-08 |
| 16. 统计 UI | 2/2 | Complete    | 2026-03-08 |
| 17. 集成优化 | 2/3 | In Progress | 2026-03-08 |

---

## Coverage

### v1.3 Requirement Mapping

| Requirement | Phase | Status |
|-------------|-------|--------|
| TAB-01 | Phase 12 | Pending |
| TAB-02 | Phase 12 | Pending |
| TAB-03 | Phase 12 | Pending |
| TAB-04 | Phase 12 | Pending |
| TAB-05 | Phase 12 | Pending |
| TAB-06 | Phase 12 | Pending |
| FILTER-01 | Phase 13 | Complete |
| FILTER-02 | Phase 13 | Complete |
| FILTER-03 | Phase 13 | Complete |
| FILTER-04 | Phase 14 | Complete |
| FILTER-05 | Phase 13 | Complete |
| STATS-01 | Phase 15 | Pending |
| STATS-02 | Phase 16 | Pending |
| STATS-03 | Phase 15 | Pending |
| STATS-04 | Phase 16 | Complete |
| STATS-05 | Phase 16 | Pending |

**Coverage:**
- v1.3 requirements: 16 total
- Mapped to phases: 16
- Unmapped: 0 ✓

---

*Roadmap created: 2026-03-05*
*Last updated: 2026-03-08 (Phase 14 planned)*
*Ready for execution: Phase 14*
