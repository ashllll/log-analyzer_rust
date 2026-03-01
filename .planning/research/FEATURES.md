# Feature Research

**Domain:** Flutter 桌面日志分析应用
**Researched:** 2026-02-28
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (用户必备功能)

用户认为理所当然存在的功能。缺少这些功能会让产品感觉不完整。

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| 全文搜索 | 日志分析的核心功能，用户输入关键词快速定位 | HIGH | 后端 Tantivy 已实现，前端需完善 UI |
| 搜索结果列表 | 查看搜索匹配行，带行号和上下文 | MEDIUM | 虚拟滚动已实现 (SliverFixedExtentList) |
| 关键词高亮 | 帮助用户快速识别匹配内容 | LOW | 后端已支持，前端渲染 |
| 多格式压缩包支持 | 日志常以压缩包形式分发 | MEDIUM | ZIP/TAR/GZ/RAR/7Z 后端已实现 |
| 工作区管理 | 创建、打开、删除工作区 | MEDIUM | 已有基础 UI，需完善 |
| 文件导入 | 导入日志文件/文件夹到工作区 | MEDIUM | import_folder 命令已实现 |
| 搜索历史 | 保存用户过去的搜索记录 | LOW | 命令已实现，前端需集成 |
| 任务进度显示 | 导入/索引操作需要进度反馈 | MEDIUM | 已有 tasks_page，需完善 |
| 筛选器 (日期/级别/文件类型) | 缩小搜索范围 | MEDIUM | 已有 filter_palette 组件 |
| 设置/配置界面 | 管理应用行为 | LOW | 已有 settings_page |

### Differentiators (差异化功能)

让产品脱颖而出的功能。非必需但有高价值。

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| 热力图小地图 (Heatmap Minimap) | 快速浏览日志分布，一眼识别问题区域 | HIGH | 前端 heatmap_minimap 已实现，差异化明显 |
| 实时文件监控 | 日志文件变化时自动更新索引和搜索结果 | HIGH | 后端 watch 命令已实现，前端需集成事件流 |
| 异步搜索流式返回 | 大文件搜索时渐进式显示结果，不阻塞 UI | MEDIUM | async_search_logs 已实现，前端需完善 |
| 搜索统计面板 | 显示匹配数、处理速度等指标 | LOW | search_stats_panel 已实现 |
| 性能监控面板 | 展示索引、缓存、内存使用情况 | MEDIUM | performance_page 已实现 |
| 虚拟文件系统 | 像浏览本地文件一样浏览归档内文件 | HIGH | get_virtual_file_tree 已实现，需完善前端 |
| 错误报告系统 | 收集用户错误反馈用于改进 | LOW | report_frontend_error 已实现 |
| 模式/关键词管理 | 保存常用搜索模式和关键词 | LOW | keywords_page 已实现 |

### Anti-Features (避免构建的功能)

看起来很好但会产生问题的功能。

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| 移动端支持 | 用户可能在手机上查看 | 移动端日志分析需求极低，UI 适配成本高 | 专注桌面端体验 |
| 云端同步 | 便于多设备访问 | 本地日志涉及敏感数据，隐私问题 | 保持纯本地应用 |
| 用户认证系统 | 区分多用户 | 本地单用户应用不需要 | 工作区隔离足矣 |
| 插件系统 (初期) | 扩展功能灵活性 | 增加架构复杂度，初期无需 | API 稳定后再考虑 |
| 实时协作编辑 | 团队协作需求 | 本地应用场景不需要 | 导出分享功能更实际 |
| WebAssembly 支持 | 跨平台运行 | Flutter Desktop 已支持主流平台 | 保持现有方案 |

## Feature Dependencies

```
[工作区管理]
    └──requires──> [CAS 存储]
                       └──requires──> [SQLite + FTS5]

[全文搜索]
    └──requires──> [索引构建]
                       └──requires──> [文件导入]

[搜索结果列表]
    └──requires──> [全文搜索]
    └──requires──> [虚拟滚动]

[热力图小地图]
    └──requires──> [虚拟滚动]
    └──requires──> [密度数据计算]

[实时文件监控]
    └──requires──> [文件监控系统]
    └──requires──> [增量索引更新]
    └──enhances──> [全文搜索]

[虚拟文件系统]
    └──requires──> [压缩包解压]
    └──requires──> [CAS 存储]
```

### Dependency Notes

- **工作区管理依赖 CAS 存储:** 工作区使用内容寻址存储来管理文件，需要先有存储基础设施
- **全文搜索依赖索引构建:** 搜索需要先建立索引，索引依赖于文件导入完成
- **热力图小地图增强虚拟滚动:** 需要虚拟滚动的行号信息来计算密度
- **实时文件监控增强搜索:** 文件变化时自动更新索引，用户下次搜索获得最新结果

## MVP Definition

### Launch With (v1.0)

最小可行产品 - 验证核心价值所需的最少功能。

- [ ] 全文搜索 (Tantivy) — 核心价值，后端已实现
- [ ] 搜索结果列表 + 关键词高亮 — 用户直接交互界面
- [ ] 工作区管理 (创建/打开/删除) — 基本使用流程
- [ ] 文件导入 + 压缩包解压 — 数据来源
- [ ] 基础筛选器 (日期范围、级别) — 缩小搜索范围

### Add After Validation (v1.x)

核心功能验证后添加。

- [ ] 热力图小地图 — 差异化功能，提升用户体验
- [ ] 实时文件监控 — 自动化工作流
- [ ] 搜索历史 — 重复搜索便利性
- [ ] 搜索统计面板 — 搜索反馈
- [ ] 任务进度显示 — 长时间操作反馈

### Future Consideration (v2+)

产品市场匹配确立后考虑。

- [ ] 虚拟文件系统 — 高级归档浏览
- [ ] 性能监控面板 — 系统优化
- [ ] 模式/关键词管理 — 高级用户功能
- [ ] 错误报告系统 — 用户反馈收集

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| 全文搜索 | HIGH | MEDIUM | P1 |
| 搜索结果列表 + 高亮 | HIGH | LOW | P1 |
| 工作区管理 | HIGH | MEDIUM | P1 |
| 文件导入 | HIGH | MEDIUM | P1 |
| 压缩包支持 | HIGH | MEDIUM | P1 |
| 基础筛选器 | HIGH | LOW | P1 |
| 热力图小地图 | MEDIUM | HIGH | P2 |
| 实时文件监控 | MEDIUM | HIGH | P2 |
| 搜索历史 | MEDIUM | LOW | P2 |
| 任务进度 | MEDIUM | MEDIUM | P2 |
| 搜索统计面板 | LOW | LOW | P3 |
| 性能监控面板 | LOW | MEDIUM | P3 |
| 关键词管理 | LOW | LOW | P3 |
| 错误报告 | LOW | LOW | P3 |

**Priority key:**
- P1: 必须上线
- P2: 尽快添加
- P3: 未来考虑

## Competitor Feature Analysis

| Feature | Logstash/Graylog | Splunk | Chainsaw (Java) | Our Approach |
|---------|------------------|--------|-----------------|--------------|
| 全文搜索 | YES | YES | YES | 后端 Tantivy，性能优于传统方案 |
| 压缩包支持 | 部分 | 部分 | 基础 | 支持 ZIP/TAR/GZ/RAR/7Z，最全面 |
| 桌面应用 | NO (C/S) | NO (Web) | YES | Flutter 现代 UI，跨平台 |
| 实时监控 | YES | YES | 基础 | 后端已支持，前端需集成 |
| 热力图 | 部分 | YES | NO | Flutter 实现，差异化功能 |
| 本地离线 | NO | NO | YES | 纯本地，隐私优先 |
| CAS 存储 | NO | NO | NO | 独特架构，避免路径限制 |

**Our Advantages (差异化优势):**
- Flutter 现代 UI，开发效率高
- Tantivy 全文搜索，性能优异
- CAS 内容寻址存储，解决 Windows 路径限制问题
- 本地离线优先，隐私安全

## Implementation Status

### Backend (Rust) - 已完成

| 功能 | 状态 | 文件位置 |
|------|------|----------|
| Tantivy 全文搜索 | ✅ | search_engine/ |
| Aho-Corasick 多模式 | ✅ | services/pattern_matcher.rs |
| 正则表达式搜索 | ✅ | search_engine/dfa_engine.rs |
| ZIP/TAR/GZ/RAR/7Z | ✅ | archive/ |
| 文件监控 | ✅ | services/file_watcher_async.rs |
| CAS 存储 | ✅ | storage/cas.rs |
| SQLite + FTS5 | ✅ | storage/ |
| 任务管理 | ✅ | task_manager/ |
| 指标收集 | ✅ | monitoring/ |

### Frontend (Flutter) - 已实现

| 页面 | 状态 | 路径 |
|------|------|------|
| 搜索页面 | ✅ 实现中 | features/search/ |
| 工作区页面 | ✅ 实现中 | features/workspace/ |
| 任务页面 | ✅ 实现中 | features/task/ |
| 关键词页面 | ✅ 实现中 | features/keyword/ |
| 性能页面 | ✅ 实现中 | features/performance/ |
| 设置页面 | ✅ 实现中 | features/settings/ |

### Frontend (Flutter) - 需完善

| 功能 | 优先级 | 依赖 |
|------|--------|------|
| 搜索结果虚拟滚动 | P1 | 已有框架 |
| 关键词高亮渲染 | P1 | 搜索结果 |
| 筛选器 UI | P1 | API 集成 |
| 热力图小地图 | P2 | 密度数据 |
| 实时监控事件流 | P2 | WebSocket/EventBus |
| 搜索历史 UI | P2 | API 集成 |
| 任务进度显示 | P2 | 事件订阅 |
| 虚拟文件系统 | P3 | 后端 API |

## Sources

- PROJECT.md - 项目需求和约束
- ARCHITECTURE.md - 现有 Rust 后端架构
- Rust 命令代码分析 (commands/*.rs)
- Flutter 页面代码分析 (features/*/)
- CLAUDE.md - 技术栈和性能基准

---
*Feature research for: Flutter Desktop Log Analyzer*
*Researched: 2026-02-28*
