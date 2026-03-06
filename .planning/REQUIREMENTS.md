# Requirements: Flutter 日志分析桌面应用 v1.2

**Defined:** 2026-03-04
**Milestone:** v1.2 UI 完善 (继续 v1.1 未完成需求)
**Core Value:** 让用户能够高效地搜索、分析和监控日志文件

## v1 Requirements

### 高级搜索

- [x] **ASEARCH-01**: 用户可以切换到正则表达式搜索模式
- [x] **ASEARCH-02**: 正则表达式搜索时提供语法反馈 (有效/无效)
- [x] **ASEARCH-03**: 用户可以输入多个关键词并选择 AND 组合
- [x] **ASEARCH-04**: 用户可以输入多个关键词并选择 OR 组合
- [x] **ASEARCH-05**: 用户可以输入多个关键词并选择 NOT 组合
- [x] **ASEARCH-06**: 用户可以查看组合后的搜索条件预览

### 搜索历史

- [x] **HIST-01**: 搜索自动保存到搜索历史
- [x] **HIST-02**: 用户可以在下拉列表中查看历史搜索记录
- [x] **HIST-03**: 用户可以点击历史记录快速填充搜索框
- [x] **HIST-04**: 用户可以删除单条历史记录
- [x] **HIST-05**: 用户可以清空所有搜索历史

### 虚拟文件树

- [ ] **VFS-01**: 用户可以查看工作区的虚拟文件树结构
- [x] **VFS-02**: 目录节点可以展开/折叠
- [ ] **VFS-03**: 用户可以点击文件预览内容
- [ ] **VFS-04**: 文件树显示文件/目录图标区分

### 集成与优化

- [ ] **INT-01**: 每个核心功能有端到端测试覆盖
- [ ] **INT-02**: 搜索响应时间 <200ms，文件树加载 <500ms
- [ ] **INT-03**: 加载状态显示、错误处理统一、无障碍支持
- [ ] **INT-04**: 代码审查完成、技术文档更新

## v2 Requirements

### 高级搜索

- **ASEARCH-07**: 支持括号组合优先级
- **ASEARCH-08**: 搜索语法高亮显示

### 搜索历史

- **HIST-06**: 搜索历史按使用频率排序
- **HIST-07**: 智能搜索建议 (基于历史)

### 虚拟文件树

- **VFS-05**: 文件树搜索过滤
- **VFS-06**: 键盘导航 (上下箭头 + 回车)
- **VFS-07**: 多选文件进行批量操作

## Out of Scope

| Feature | Reason |
|---------|--------|
| 目录层级浏览 | 延期到后续里程碑 |
| 云端搜索历史同步 | 本地应用不需要，隐私问题 |
| 实时搜索结果预览 | 性能复杂，延期 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| ASEARCH-01 | Phase 9 | Complete |
| ASEARCH-02 | Phase 9 | Complete |
| ASEARCH-03 | Phase 9 | Complete |
| ASEARCH-04 | Phase 9 | Complete |
| ASEARCH-05 | Phase 9 | Complete |
| ASEARCH-06 | Phase 9 | Complete |
| HIST-01 | Phase 9 | Complete |
| HIST-02 | Phase 9 | Complete |
| HIST-03 | Phase 9 | Complete |
| HIST-04 | Phase 9 | Complete |
| HIST-05 | Phase 9 | Complete |
| VFS-01 | Phase 10 | Pending |
| VFS-02 | Phase 10 | Complete |
| VFS-03 | Phase 10 | Pending |
| VFS-04 | Phase 10 | Pending |
| INT-01 | Phase 11 | Pending |
| INT-02 | Phase 11 | Pending |
| INT-03 | Phase 11 | Pending |
| INT-04 | Phase 11 | Pending |

**Coverage:**
- v1 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0 ✓

---

*Requirements defined: 2026-03-04*
*Last updated: 2026-03-05 after v1.2 milestone started*
