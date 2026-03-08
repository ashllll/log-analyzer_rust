# Flutter 日志分析桌面应用

## What This Is

使用 Flutter 全新实现的桌面日志分析应用，通过 FFI 与现有 Rust 后端通信。已完成 Phase 7 后端 API 集成（搜索历史、虚拟文件树、正则搜索、多关键词搜索），为后续 UI 阶段提供 FFI 桥接支持。

## Core Value

让用户能够高效地搜索、分析和监控日志文件，支持多种压缩包格式，提供实时更新能力。

## Current Milestone: v1.3 功能扩展 (计划中)

**Status:** 🔄 研究完成，准备规划
**Phases:** 12-17 (计划中)

**Target features:**
- 自定义过滤器 (用户可创建命名过滤器如"仅错误"、"最近24小时")
- 多工作区标签页 (多工作区标签页切换)
- 日志级别统计 (日志级别统计面板：错误/警告/信息数量)

**Previous milestone:** v1.2 UI 完善 (2026-03-07)
- ✅ Phase 9: 高级搜索 UI
- ✅ Phase 10: 虚拟文件系统 UI
- ✅ Phase 11: 集成与优化

## Next Milestone Goals

v1.3 功能扩展完成后定义后续里程碑

## Requirements

### Active

v1.3 功能扩展 (2026-03-07 研究完成):
- 🔄 自定义过滤器 - 侧边栏快捷访问 + 模态对话框创建/编辑
- 🔄 多工作区标签页 - Tab bar + PageView/IndexedStack + Riverpod family
- 🔄 日志级别统计 - fl_chart 图表 + 5秒自动刷新

### Validated

从现有 Rust 后端推断的功能：

- ✓ 全文搜索 (Tantivy) — 现有后端已实现
- ✓ 多模式匹配 (Aho-Corasick) — 现有后端已实现
- ✓ 正则表达式搜索 — 现有后端已实现
- ✓ 关键词高亮 — 现有后端已实现
- ✓ ZIP/TAR/GZ/RAR/7Z 压缩包解压 — 现有后端已实现
- ✓ 文件系统监控 — 现有后端已实现
- ✓ 增量索引更新 — 现有后端已实现
- ✓ CAS 内容寻址存储 — 现有后端已实现
- ✓ SQLite + FTS5 元 metadata — 现有后端已实现
- ✓ 任务进度跟踪 — 现有后端已实现
- ✓ Flutter 桌面 UI 框架搭建 — v1.0 MVP 已完成
- ✓ 搜索功能界面 — v1.0 MVP 已完成
- ✓ 多条件筛选 UI — v1.0 MVP 已完成
- ✓ 压缩包管理界面 — v1.0 MVP 已完成
- ✓ 工作区管理界面 — v1.0 MVP 已完成
- ✓ 实时监控面板 — v1.0 MVP 已完成
- ✓ 设置/配置界面 — v1.0 MVP 已完成
- ✓ 任务进度显示 UI — v1.0 MVP 已完成
- ✓ 搜索历史 FFI 桥接 — v1.1 Phase 7 已完成
- ✓ 虚拟文件树 FFI 桥接 — v1.1 Phase 7 已完成
- ✓ 正则表达式搜索 FFI 桥接 — v1.1 Phase 7 已完成
- ✓ 多关键词组合搜索 FFI 桥接 — v1.1 Phase 7 已完成
- ✓ SearchHistoryProvider with AsyncNotifier — v1.1 Phase 8 已完成
- ✓ VirtualFileTreeProvider with FFI integration — v1.1 Phase 8 已完成
- ✓ 乐观更新与错误回滚模式 — v1.1 Phase 8 已完成
- ✓ Riverpod 3.0 family pattern for workspace scoping — v1.1 Phase 8 已完成

### Validated

v1.2 UI 完善已交付 (2026-03-07):
- ✓ 正则表达式搜索 UI (ASEARCH-01, ASEARCH-02) — Phase 9
- ✓ 多关键词组合搜索 UI (ASEARCH-03, ASEARCH-04, ASEARCH-05, ASEARCH-06) — Phase 9
- ✓ 搜索历史记录 UI (HIST-01, HIST-02, HIST-03, HIST-04, HIST-05) — Phase 9
- ✓ 虚拟文件树 UI (VFS-01, VFS-02, VFS-03, VFS-04) — Phase 10
- ✓ 集成与优化 (INT-01, INT-02, INT-03, INT-04) — Phase 11 (部分：测试待 FFI 修复后运行)

### Out of Scope

- 移动端支持 — 用户明确不需要
- 云端同步 — 本地桌面应用
- 用户认证系统 — 本地应用不需要
- 日志热力图 — 延期到后续里程碑

## Context

**现有代码库**:
- Rust 后端已完成核心功能 (搜索、压缩包、监控)
- 已有 Flutter 项目结构 (`log-analyzer_flutter/`)
- 已有 FFI 绑定代码 (`frb_generated.rs`)
- 已有 HTTP API 端点 (axum)

**技术约束**:
- Flutter >=3.8.0 桌面应用
- 通过 flutter_rust_bridge 或 HTTP API 与 Rust 后端通信
- Windows/macOS/Linux 桌面平台

**v1.1 Phase 7 实现的 FFI 模式**:
- 三层 FFI 架构: `bridge.rs` (导出) → `commands_bridge.rs` (适配) → 业务逻辑
- 同步 FFI: 使用 `#[frb(sync)]` 简化 Flutter 集成
- 懒加载模式: 虚拟文件树支持按需加载子节点

**v1.1 Phase 8 实现的状态管理模式**:
- Riverpod 3.0 AsyncNotifier with family parameter for workspace scoping
- 乐观更新模式: save previous state → update UI → rollback on failure
- 本地 Dart model wrapper for FFI types (riverpod_generator 兼容性)
- Dart 3 pattern matching for sealed class FFI type conversion

## Constraints

- **性能**: 搜索响应时间 <200ms (继承现有后端能力)
- **兼容性**: 与现有 Rust 后端 API 兼容
- **平台**: 桌面端 (Windows/macOS/Linux)，不需要移动端

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Flutter 替代 Tauri 前端 | 更好的开发效率，更现代化的 UI | ✓ Good |
| 保留 Rust 后端所有功能 | 已有完整实现，无需重写 | ✓ Good |
| FFI + HTTP API 双通道 | FFI 优先，HTTP 作为备选 | ✓ Good |
| 三层 FFI 架构 | 分离关注点，易于维护和测试 | ✓ Good (Phase 7 验证) |
| 复用 SearchHistoryManager | 避免重复实现，保持代码一致性 | ✓ Good |
| 复用 PatternMatcher (Aho-Corasick) | O(n+m) 复杂度，高性能多模式匹配 | ✓ Good |
| 本地 Dart model wrapper for FFI types | riverpod_generator 无法处理外部类型 | ✓ Good |
| Dart 3 sealed class + pattern matching | 类型安全的 FFI 转换，编译时检查 | ✓ Good |
| 侧边栏 + 对话框模式 (过滤器) | 复用现有组件，熟悉的 UX | ✓ Good (v1.3 研究) |
| Tab bar + PageView/IndexedStack | 成熟桌面模式，Flutter 内置支持 | ✓ Good (v1.3 研究) |
| fl_chart + 5秒刷新 | 现有依赖，匹配监控模式 | ✓ Good (v1.3 研究) |

---

## v1.3 研究摘要

**研究完成日期:** 2026-03-07

**研究文件:**
- `.planning/research/v1.3-custom-filters.md` - 自定义过滤器 UI 模式
- `.planning/research/v1.3-multi-workspace-tabs.md` - 多工作区标签页模式
- `.planning/research/v1.3-log-level-stats.md` - 日志级别统计模式
- `.planning/research/v1.3-integration.md` - 集成模式

**关键决策:**
- 过滤器: 侧边栏快捷访问 + 模态对话框创建/编辑
- 标签页: Tab bar + PageView/IndexedStack + Riverpod family providers
- 统计: fl_chart 条形图/饼图 + 5秒自动刷新 + 30秒缓存 TTL

*Last updated: 2026-03-07 after v1.3 research complete*
