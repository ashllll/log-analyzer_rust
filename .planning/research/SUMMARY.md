# Project Research Summary

**Project:** Flutter Desktop Log Analyzer (log-analyzer_rust)
**Domain:** 高性能桌面日志分析工具 - Flutter + Rust FFI 集成应用
**Researched:** 2026-02-28
**Confidence:** HIGH

## Executive Summary

这是一个高性能桌面日志分析工具，采用 Flutter + Rust 双栈架构。Flutter 作为前端 UI 层，Rust 作为后端处理搜索引擎、压缩包解析和 CAS 存储。核心价值在于提供本地离线日志分析能力，结合 Tantivy 全文搜索引擎和 Aho-Corasick 多模式匹配算法，实现高速搜索体验。

基于研究，推荐采用 **Clean Architecture + Riverpod 3.0** 架构，优先使用 FFI (flutter_rust_bridge) 与 Rust 后端通信。主要风险在于 FFI 边界处理：字段名不一致、错误丢失、线程阻塞、状态同步和内存泄漏。后端 Tantivy 搜索、CAS 存储、压缩包处理已实现，前端需完善搜索结果渲染、筛选器 UI 和热力图小地图。

**关键建议:**
1. 架构阶段强制统一 snake_case 命名规范
2. MVP 阶段聚焦搜索、工作区、文件导入三大核心功能
3. 差异化功能 (热力图、实时监控) 放在 v1.x 阶段
4. FFI 边界错误处理必须在 Phase 2-3 完成

## Key Findings

### Recommended Stack

Flutter 桌面应用推荐技术栈已确定。**核心框架**: Flutter SDK >=3.8.0 <4.0.0 官方支持 Windows/macOS/Linux 桌面。**状态管理**: Riverpod 3.0 是 2026 年官方推荐方案，相比 BLoC 样板代码少 60%，内置编译时安全和异步支持。**FFI 桥接**: 继续使用 flutter_rust_bridge 2.x，已有项目基础，生成类型安全绑定。**HTTP 客户端**: Dio 5.4.0，拦截器、缓存、进度监控完善。**路由**: go_router 14.0.0，声明式路由，深度链接支持。**数据类**: freezed 3.2.3 + json_serializable 6.11.2，不可变数据类，withCopyWith 自动实现。

**Core technologies:**
- **Flutter SDK 3.8.0+**: 桌面 UI 框架 — 官方支持全平台桌面
- **Riverpod 3.0**: 状态管理 — 编译时安全，样板代码最少
- **flutter_rust_bridge 2.x**: FFI 桥接 — 项目已有基础，类型安全
- **Dio 5.4.0**: HTTP 客户端 — 拦截器、自动 JSON、请求取消
- **go_router 14.0.0**: 声明式路由 — 官方推荐，类型安全
- **Tantivy 0.22**: 全文搜索 — Rust 版 Lucene，性能优异

### Expected Features

**Must have (table stakes):**
- 全文搜索 (Tantivy) — 日志分析核心，后端已实现
- 搜索结果列表 + 关键词高亮 — 用户直接交互界面
- 工作区管理 (创建/打开/删除) — 基本使用流程
- 文件导入 + 压缩包解压 (ZIP/TAR/GZ/RAR/7Z) — 数据来源
- 基础筛选器 (日期范围、级别) — 缩小搜索范围
- 任务进度显示 — 长时间操作反馈

**Should have (competitive):**
- 热力图小地图 — 快速浏览日志分布，一眼识别问题区域
- 实时文件监控 — 日志变化时自动更新索引
- 异步搜索流式返回 — 大文件渐进式显示结果
- 搜索统计面板 — 显示匹配数、处理速度等指标
- 搜索历史 — 重复搜索便利性

**Defer (v2+):**
- 虚拟文件系统 — 高级归档浏览
- 性能监控面板 — 系统优化
- 模式/关键词管理 — 高级用户功能
- 错误报告系统 — 用户反馈收集

### Architecture Approach

推荐 **Clean Architecture with Riverpod** 四层架构。**Domain Layer**: Entities, Use Cases, Repository Interfaces — 无外部依赖，最先构建。**Data Layer**: Models, DataSources (FFI/HTTP), Repository Implementations — 依赖 Domain。**Presentation Layer**: Providers, Notifiers, Widgets, Screens — 依赖 Domain 抽象。**External Integrations**: Rust Backend (FFI), Local Storage。

**Major components:**
1. **Search Feature**: Domain (SearchQuery, SearchResult entities) -> Data (FFISearchDataSource) -> Presentation (SearchNotifier)
2. **Workspace Feature**: Domain (Workspace entity) -> Data (FFIWorkspaceDataSource) -> Presentation (WorkspaceNotifier)
3. **Task Feature**: Domain (Task entity) -> Data (FFITaskDataSource) -> Presentation (TaskNotifier)
4. **Shared Services**: ApiService (Dio), BridgeService (FRB wrapper), EventStreamService (Tauri events)

### Critical Pitfalls

1. **字段名不一致 (snake_case vs camelCase)** — Dart 惯例 camelCase，Rust 惯例 snake_case，FRB 默认不转换。**预防**: 整个项目强制使用 snake_case
2. **FFI 边界错误处理丢失** — Rust Result 错误在 FFI 边界丢失，Flutter 只收到 panic/null。**预防**: 使用 thiserror 定义结构化错误，Flutter 端实现 ErrorWidget
3. **主线程阻塞** — 同步 FFI 调用导致 UI 冻结。**预防**: 优先使用 async fn，避免同步函数用于耗时操作
4. **状态同步竞态条件** — Flutter 状态与 Rust 状态不一致。**预防**: 实现版本号机制，幂等事件处理
5. **FFI 边界内存泄漏** — Rust 分配的内存在 Dart 端未正确释放。**预防**: 使用 FRB opaque 类型，避免裸指针

## Implications for Roadmap

基于研究，建议以下阶段划分:

### Phase 1: 架构基础设施
**Rationale:** 基础不牢，地动山摇。FFI 边界问题必须在开发前期解决，否则后期修复成本极高。

**Delivers:**
- 项目结构搭建 (Clean Architecture folders)
- 共享服务层 (ApiService, BridgeService)
- 错误处理框架 (Failure classes)
- Riverpod Provider 基础配置

**Addresses:** 字段名规范 (P1 Pitfall), 错误处理设计 (P2 Pitfall), FFI 边界设计

**Avoids:** 字段名不一致导致的全链路返工

### Phase 2: 核心功能 MVP
**Rationale:** 验证核心价值 — 日志搜索。工作区管理是使用流程起点，文件导入是数据来源，三者形成闭环。

**Delivers:**
- 全文搜索功能 (Tantivy 集成)
- 工作区管理 (创建/打开/删除)
- 文件导入 + 压缩包解压
- 搜索结果列表 + 关键词高亮
- 基础筛选器 UI

**Uses:** flutter_rust_bridge, Riverpod 3.0, go_router

**Implements:** Search Feature (Domain -> Data -> Presentation 全链路)

### Phase 3: 完善与差异化
**Rationale:** 核心功能验证后，快速添加差异化功能和体验优化。

**Delivers:**
- 热力图小地图
- 实时文件监控 (事件流集成)
- 搜索历史
- 任务进度显示
- 搜索统计面板

**Addresses:** 状态同步问题 (P4 Pitfall), 内存管理 (P5 Pitfall)

**Avoids:** 纯 UI 堆砌，忽略架构质量

### Phase 4: 性能优化
**Rationale:** 大数据量场景下的性能问题必须解决。

**Delivers:**
- 虚拟滚动优化 (10万+ 行)
- 搜索性能调优
- 内存使用优化
- 异步处理完善

**Avoids:** 主线程阻塞 (P3 Pitfall)

### Phase 5: 高级功能 (v2+)
**Rationale:** 产品市场匹配确立后考虑。

**Delivers:**
- 虚拟文件系统
- 性能监控面板
- 模式/关键词管理
- 错误报告系统

### Phase Ordering Rationale

1. **依赖驱动**: Domain 层无依赖 -> Data 层依赖 Domain -> Presentation 层依赖 Domain 抽象
2. **风险前置**: FFI 边界问题 (最常见 Bug) 在 Phase 1-2 解决
3. **价值验证**: 核心搜索功能在 Phase 2 完成 MVP，尽早验证产品假设
4. **差异化跟进**: 热力图、实时监控需要基础稳定后才实现
5. **性能收尾**: 大数据量性能问题需要功能稳定后才能有效复现和优化

### Research Flags

**Phases likely needing deeper research during planning:**
- **Phase 1 (架构基础设施)**: FFI 错误传播机制具体实现，需要参考 FRB 文档细化
- **Phase 3 (完善与差异化)**: 热力图密度计算算法，实时监控事件流架构
- **Phase 4 (性能优化)**: 大数据虚拟滚动最佳实践，需要验证 Flutter 3.8 性能

**Phases with standard patterns (skip research-phase):**
- **Phase 2 (核心 MVP)**: 搜索/工作区/导入是标准 CRUD 模式，社区文档丰富
- **UI 组件**: Flutter 组件实现是标准模式，官方文档充足

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | 基于 Flutter 官方推荐和项目已有技术栈 |
| Features | HIGH | 后端功能已完成，前端集成路径清晰 |
| Architecture | HIGH | Clean Architecture + Riverpod 是社区共识 |
| Pitfalls | MEDIUM | FFI 边界问题来自实际项目经验，但具体实现细节需验证 |

**Overall confidence:** HIGH

**Gaps to Address:**
1. **热力图算法**: 密度计算方式未深入研究，需 Phase 3 前验证
2. **实时监控事件流**: Tauri 事件系统与 Flutter Riverpod 集成方式需确认
3. **大规模性能**: 10万+ 行虚拟滚动性能需实际测试验证

## Sources

### Primary (HIGH confidence)
- Flutter 官方架构建议 — https://docs.flutter.dev/app-architecture/recommendations
- Riverpod 3.0 官方文档 — https://pub.dev/packages/flutter_riverpod
- flutter_rust_bridge 官方文档 — https://cjycode.com/flutter_rust_bridge/
- Tantivy 0.22 文档 — 项目已有实现基础

### Secondary (MEDIUM confidence)
- Flutter Clean Architecture 模板 — https://ssoad.github.io/flutter_riverpod_clean_architecture/
- Riverpod vs BLoC 2026 对比 — https://flutterstudio.dev/blog/bloc-vs-riverpod-flutter-state-management-2026.html

### Tertiary (LOW confidence)
- FFI 性能优化技巧 — https://microsoft.github.io/rust-guidelines/guidelines/ffi/

---

*Research completed: 2026-02-28*
*Ready for roadmap: yes*
