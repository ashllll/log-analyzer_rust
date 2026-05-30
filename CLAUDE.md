# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Log Analyzer is a **Tauri 2.x desktop app** for large-scale log file import, search, analysis, and real-time monitoring. Rust backend handles performance-critical operations; React/TypeScript frontend provides the UI.

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop framework | Tauri 2.x (IPC via Tauri Events, not WebSocket) |
| Backend language | Rust **1.85** (pinned in `rust-toolchain.toml`) |
| Frontend | React 19 + TypeScript 5.8 |
| State management | Zustand 5 + TanStack React Query 5 |
| Build tooling | Vite 7, ESLint 9, Jest 30 |
| CSS | Tailwind CSS 3 |

## Essential Commands

All commands run from within `log-analyzer/` unless otherwise noted.

### Frontend

```bash
cd log-analyzer
npm install                     # Install dependencies
npm run dev                     # Vite dev server (frontend-only)
npm run build                   # TypeScript compile + Vite production build
npm run lint                    # ESLint check
npm run lint:fix                # ESLint auto-fix
npm run type-check              # tsc --noEmit
npm test                        # Jest unit tests
npm test -- --testPathPattern=<pattern>  # Run specific test file
```

### Rust Backend

```bash
cd log-analyzer/src-tauri
cargo fmt -- --check            # Format check
cargo clippy --all-features --all-targets -- -D warnings  # Lint
cargo test --workspace          # All workspace tests
cargo test -p la-core           # Tests for a specific crate
cargo test --all-features       # With all features (including rar)
cargo check --workspace         # Fast compile check (no codegen)
cargo tarpaulin --config tarpaulin.toml --out Html --output-dir coverage  # Code coverage
```

### Tauri Development

```bash
cd log-analyzer
npm run tauri dev               # Full Tauri dev mode (Rust + frontend with hot reload)
npm run tauri build             # Production build
npm run tauri build -- --debug --no-bundle  # Debug smoke build
```

### CI Validation (pre-push)

```bash
# Full local CI check (from repo root):
bash scripts/validate-ci.sh

# IPC consistency check:
bash scripts/check_ipc_consistency.sh

# Release preparation check:
node scripts/prepare-release.mjs check
```

## Architecture: Clean Architecture Layers

```
interfaces/ (collapsed into commands/)  ← #[tauri::command] definitions
commands/                               ← Parameter validation + delegate to use cases
application/                            ← Use cases: search, import, watch, workspace, config, export
infrastructure/                         ← Adapters implementing domain traits
services/                               ← Engines: query_planner, query_executor, regex_engine, file_watcher, query_validator
models/                                 ← AppState container
utils/                                  ← encoding, validation, cache, retry, cancellation, paths, resource tracking
state_sync/                             ← Frontend-backend state synchronization models
task_manager/                           ← Async task lifecycle: create → update → complete → cancel
```

**Key rule**: Application (use cases) depends on domain traits. Infrastructure implements those traits. Tauri commands delegate to use cases. Traits are defined in `la-core` crate's `domain/` module — zero dependency on Tauri or filesystem.

## Workspace Crates

Located under `log-analyzer/src-tauri/crates/`:

| Crate | Purpose |
|---|---|
| `la-core` | Domain traits (`LogSearcher`, `ContentStorage`, etc.), error types (`AppError`), models (`SearchQuery`, `LogEntry`, config) |
| `la-storage` | CAS (Content Addressable Storage with SHA-256), MetadataStore (SQLite via `sqlx`), GC, integrity checks |
| `la-search` | Query engine (Aho-Corasick / Regex / Memchr), DiskResultStore, highlighting, Tantivy manager |
| `la-archive` | Archive extraction (ZIP, TAR, GZ, 7Z, RAR), extraction orchestration, symlink guard, security detection |

## Key Domain Traits (la-core `domain/`)

All defined in `la-core/src/domain/mod.rs`, implemented by `infrastructure/`:
- **`LogSearcher`** — Build execution plan from query, match content against plan (sync — runs in `spawn_blocking`)
- **`LogFileRepository`** — Read log files by hash/virtual path
- **`SearchResultRepository`** — Store/search paginated results
- **`ArchiveExtractor`** — Extract archive formats
- **`EventPublisher`** — Push progress/results to frontend via Tauri Events
- **`WorkspaceRepository`** — Workspace CRUD
- **`TaskScheduler`** — Async task lifecycle management

Separate `la-core/src/traits.rs` defines: `QueryValidation`, `ContentStorage`, `MetadataStorage`, `AppConfigProvider`.

## Key Design Decisions

1. **Tauri Events for IPC** — No WebSocket; desktop-only app uses Tauri's built-in event system
2. **CAS dedup** — Log content stored by SHA-256 hash; identical content shared across workspaces
3. **`spawn_blocking` isolation** — Search/import run on Rayon thread pool, never block the Tauri event loop
4. **DiskResultStore** — Search results paged to disk to avoid OOM on large result sets
5. **ReDoS protection** — `regex_engine` validates queries for exponential backtracking before execution
6. **Rust 1.85 pinned** — `rust-toolchain.toml` enforces reproducible builds

## Cargo Features

```toml
default = ["rar-support", "enhanced-extraction"]
rar-support = ["unrar"]
enhanced-extraction = ["la-archive/enhanced-extraction"]
test = []
```

## Version Consistency

Before release, these three files must have matching versions:
- `log-analyzer/package.json`
- `log-analyzer/src-tauri/Cargo.toml`
- `log-analyzer/src-tauri/tauri.conf.json`

## Offline-First

This project is designed for fully offline local use. All dependencies must be vendorable. No runtime network calls except optional error reporting (Sentry, feature-gated).

---

## 附录：Claude Code Skill 中文速查表

> 以下对照表帮助阅读 `/` 命令的英文简介。Skill 描述内嵌于 Claude Code 系统文件中，无法在不破坏升级的前提下直接修改。本表作为补充参考。

### 🔧 开发辅助

| 命令 | 中文说明 |
|------|---------|
| `/agent-development` | 智能体开发 — 创建自定义 agent、编写 system prompt、定义触发条件 |
| `/command-development` | 命令开发 — 创建自定义斜杠命令、命令注册解耦 |
| `/skill-development` | Skill 开发 — 编写 skill 文件、frontmatter 规范、工具配置 |
| `/hook-development` | Hook 开发 — 自定义生命周期钩子（PreToolUse/PostToolUse 等） |
| `/mcp-integration` | MCP 集成 — 接入 Model Context Protocol 服务器 |
| `/plugin-structure` | 插件结构 — Claude Code 插件目录结构说明 |
| `/plugin-settings` | 插件设置 — 配置插件参数 |

### 🔍 代码审查与重构

| 命令 | 中文说明 |
|------|---------|
| `/code-review` | 代码审查 — 检查 diff 正确性、可复用性、效率优化（`--comment` 内联评论 / `--fix` 自动修复） |
| `/review` | 全面审查 — 覆盖代码正确性和架构问题 |
| `/simplify` | 简化代码 — 只优化质量，不找 bug |
| `/security-review` | 安全审查 — 安全漏洞和合规检查 |
| `/request-refactor-plan` | 请求重构方案 — 生成详细重构实施计划 |
| `/triage` | 问题分类 — bug 优先级分类和路由 |
| `/diagnose` | 诊断 — 分析错误日志、堆栈跟踪，定位根因 |

### 🏗️ 架构与设计

| 命令 | 中文说明 |
|------|---------|
| `/design-an-interface` | 接口设计 — 并行生成多个截然不同的接口方案 |
| `/improve-codebase-architecture` | 改进代码库架构 — 分析并建议架构改进 |
| `/frontend-design` | 前端设计 — UI/UX 设计辅助、组件设计建议 |
| `/prototype` | 原型开发 — 快速构建 MVP 原型 |

### ✍️ 写作与文档

| 命令 | 中文说明 |
|------|---------|
| `/to-prd` | 转 PRD — 将需求描述转换为产品需求文档 |
| `/to-issues` | 转 Issues — 将讨论内容提取为 GitHub Issues |
| `/edit-article` | 文章编辑 — 长文编辑和润色 |
| `/grill-me` | 拷问我 — 通过提问帮助深入思考 |
| `/grill-with-docs` | 文档拷问 — 基于文档提出挑战性问题 |
| `/qa` | 问答 — 基于上下文回答技术问题 |

### ⚙️ 系统配置与工具

| 命令 | 中文说明 |
|------|---------|
| `/update-config` | 更新配置 — 修改 settings.json（hooks、权限、环境变量、MCP） |
| `/keybindings-help` | 快捷键帮助 — 自定义键盘快捷键 |
| `/setup-pre-commit` | 配置 pre-commit — 设置 git pre-commit hooks |
| `/verify` | 验证 — 运行应用验证代码变更 |
| `/run` | 运行 — 启动并驱动项目应用 |
| `/init` | 初始化 — 创建新的 CLAUDE.md 项目文档 |
| `/find-skills` | 查找 Skill — 从开放生态发现和安装 skill |

### 🧪 测试与质量

| 命令 | 中文说明 |
|------|---------|
| `/tdd` | 测试驱动开发 — 按 TDD 流程编写测试和实现 |
| `/handoff` | 交接 — 将对话压缩为交接文档供其他 agent 接手 |

### 🔄 工作流与循环

| 命令 | 中文说明 |
|------|---------|
| `/loop` | 循环 — 按间隔重复执行命令（如 `/loop 5m /foo`） |
| `/fewer-permission-prompts` | 减少权限提示 — 扫描历史生成权限白名单 |

### 🔌 插件专属

| 命令 | 中文说明 |
|------|---------|
| `/hookify:configure` / `:help` / `:hookify` / `:list` | Hookify 插件 — 配置/帮助/主功能/列表 |
| `/claude-hud:setup` / `:configure` | Claude HUD — 状态栏配置 |

### 🔬 其他

| 命令 | 中文说明 |
|------|---------|
| `/claude-api` | Claude API 开发 — 构建/调试/优化 Anthropic SDK 应用 |
| `/deep-research` | 深度研究 — 多源搜索、对抗性验证、合成引用报告 |
| `/claude-opus-4-5-migration` | Opus 4.5 迁移 — 从旧版本 prompt/代码迁移 |
| `/caveman` | 极简模式 — 压缩 token 用量约 75%，去除废话 |
| `/obsidian-vault` | Obsidian 知识库 — 与 Obsidian 笔记集成 |

### 📌 内置命令（非 Skill）

| 命令 | 中文说明 |
|------|---------|
| `/config` | 配置 — 修改 theme、model、language 等简单设置 |
| `/help` | 帮助 — 显示帮助信息 |
| `/clear` | 清空 — 清空当前对话上下文 |
| `/exit` | 退出 — 退出 Claude Code |
| `/cost` | 费用 — 显示 token 消耗和费用估算 |
