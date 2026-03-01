# Flutter 日志分析桌面应用

## What This Is

使用 Flutter 全新实现的桌面日志分析应用，通过 FFI 或 HTTP API 与现有 Rust 后端通信。替代现有 Tauri 前端，提供更现代化的 UI 和更好的开发效率。

## Core Value

让用户能够高效地搜索、分析和监控日志文件，支持多种压缩包格式，提供实时更新能力。

## Requirements

### Validated

从现有 Rust 后端推断的功能：

- ✓ 全文搜索 (Tantivy) — 现有后端已实现
- ✓ 多模式匹配 (Aho-Corasick) — 现有后端已实现
- ✓ 正则表达式搜索 — 现有后端已实现
- ✓ 关键词高亮 — 现有后端已实现
- ✓ ZIP 压缩包解压 — 现有后端已实现
- ✓ TAR 压缩包解压 — 现有后端已实现
- ✓ GZIP 压缩包解压 — 现有后端已实现
- ✓ RAR 压缩包解压 — 现有后端已实现
- ✓ 7Z 压缩包解压 — 现有后端已实现
- ✓ 文件系统监控 — 现有后端已实现
- ✓ 增量索引更新 — 现有后端已实现
- ✓ CAS 内容寻址存储 — 现有后端已实现
- ✓ SQLite + FTS5 元数据 — 现有后端已实现
- ✓ 任务进度跟踪 — 现有后端已实现

### Active

- [ ] Flutter 桌面 UI 框架搭建
- [ ] 搜索功能界面 (搜索框、结果列表、关键词高亮显示)
- [ ] 多条件筛选 UI (日期范围、文件类型、级别筛选)
- [ ] 压缩包管理界面 (解压、预览、内嵌搜索)
- [ ] 工作区管理界面 (创建、打开、删除工作区)
- [ ] 文件树/虚拟文件系统 UI
- [ ] 实时监控面板 (文件变化、索引状态)
- [ ] 搜索历史记录 UI
- [ ] 设置/配置界面
- [ ] 任务进度显示 UI

### Out of Scope

- 移动端支持 — 用户明确不需要
- 云端同步 — 本地桌面应用
- 用户认证系统 — 本地应用不需要

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

## Constraints

- **性能**: 搜索响应时间 <200ms (继承现有后端能力)
- **兼容性**: 与现有 Rust 后端 API 兼容
- **平台**: 桌面端 (Windows/macOS/Linux)，不需要移动端

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Flutter 替代 Tauri 前端 | 更好的开发效率，更现代化的 UI | — Pending |
| 保留 Rust 后端所有功能 | 已有完整实现，无需重写 | — Pending |
| FFI + HTTP API 双通道 | FFI 优先，HTTP 作为备选 | — Pending |

---
*Last updated: 2026-02-28 after initialization*
