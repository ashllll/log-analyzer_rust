# Milestones
## v1.1 高级搜索与虚拟文件系统 (Shipped: 2026-03-04)

**Phases completed:** 1/5 phases (Phase 7 only), 4/19 plans, 14 tasks
**Git Range:** c2a8119 → 927e80d

**Key accomplishments:**
1. **搜索历史 FFI 桥接** — Flutter 可调用后端搜索历史 CRUD 操作 (add/get/delete/clear)
2. **虚拟文件树 FFI 桥接** — 懒加载文件树访问，支持 CAS 内容寻址存储
3. **正则表达式搜索 FFI** — 模式验证和高效正则搜索，支持大小写敏感/不敏感模式
4. **多关键词组合搜索 FFI** — AND/OR/NOT 结构化查询，Aho-Corasick 算法 O(n+m) 复杂度

### Known Gaps (未完成的需求)

**高级搜索 (ASEARCH-01 ~ ASEARCH-06):**
- [ ] 正则表达式搜索模式切换 UI (Phase 9)
- [ ] 正则表达式语法反馈 UI (Phase 9)
- [ ] 多关键词 AND/OR/NOT 组合 UI (Phase 9)
- [ ] 搜索条件预览 (Phase 9)

**搜索历史 (HIST-01 ~ HIST-05):**
- [ ] 搜索自动保存 (Phase 9)
- [ ] 历史搜索记录列表 (Phase 9)
- [ ] 历史记录快速填充 (Phase 9)
- [ ] 历史管理功能 (Phase 9)

**虚拟文件树 (VFS-01 ~ VFS-04):**
- [ ] 虚拟文件树 UI (Phase 10)
- [ ] 目录展开/折叠 (Phase 10)
- [ ] 文件预览面板 (Phase 10)

**未开始阶段:**
- Phase 8: 状态管理 (0/2 plans)
- Phase 9: 高级搜索 UI (0/4 plans)
- Phase 10: 虚拟文件系统 UI (0/3 plans)
- Phase 11: 集成与优化 (0/3 plans)

---


## v1.0 MVP — Flutter 日志分析应用基础

**Shipped:** 2026-03-01
**Phases:** 1-2 (Phase 1, Phase 2)
**Plans:** 7 completed
**Git Range:** 12ccd32 → b5f2e4a

### Key Accomplishments

1. **FFI 桥接服务重构** — 从 HTTP API 迁移到纯 FFI 模式，使用 flutter_rust_bridge
2. **错误处理框架** — ErrorCodes 分类 + AppException + ErrorView 组件
3. **启动流程** — Splash Screen (含 FFI 初始化) + go_router 路由配置
4. **工作区管理增强** — 键盘导航 + 最近优先排序 + 状态轮询
5. **文件夹导入** — 拖放支持 + ImportProgressDialog 进度显示
6. **压缩包导入** — ZIP/TAR/GZ/RAR/7Z 全格式支持

### Known Gaps

- 压缩包预览功能：后端 list_archive 命令未实现，预览对话框显示空列表
- 选择性解压：后端 import_archive_files 未支持

### Stats

| Metric | Value |
|--------|-------|
| Phases | 2 |
| Plans | 7 |
| Commits | 14+ |
| Timeline | 2026-02-28 → 2026-03-01 |

---

*Last milestone: v1.0 (2026-03-01)*
