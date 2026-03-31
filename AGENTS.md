<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# log-analyzer_rust

## Purpose
高性能桌面日志分析工具，基于 Tauri 2.0 + Rust 后端 + React 前端构建。支持多格式压缩包解压、全文搜索、实时监控、性能分析等功能。

## Key Files

| File | Description |
|------|-------------|
| `CLAUDE.md` | 项目架构文档和开发规范 |
| `FIX_PLAN.md` | 已知问题修复计划（36个问题已全部修复） |
| `README.md` | 项目简介和快速开始指南 |
| `package.json` | 前端依赖管理 |
| `Cargo.toml` | Rust Workspace 配置 |
| `tsconfig.json` | TypeScript 配置 |
| `vite.config.ts` | Vite 构建配置 |
| `tailwind.config.js` | Tailwind CSS 主题配置 |
| `eslint.config.js` | ESLint 规则配置 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `docs/` | 项目文档（架构、开发指南、性能报告）(see `docs/AGENTS.md`) |
| `log-analyzer/` | 主应用代码（前端+后端）(see `log-analyzer/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 所有代码修改必须通过 `cargo test` 和 `npm run type-check` 验证
- 遵循 CLAUDE.md 中的编码规范
- 使用中文编写代码注释
- 优先采用业内成熟方案，避免简单修复

### Testing Requirements
- Rust: `cargo test --all-features --lib` (298个测试)
- TypeScript: `npm run type-check`
- 前端测试: `npm test`

### Common Patterns
- Git 提交遵循 conventional commits 规范
- PR 前必须更新相关文档
- 复杂变更需先通过 planner agent 制定计划

## Dependencies

### External
- **Tauri 2.0** - 桌面应用框架
- **React 19** - UI 框架
- **TypeScript 5.8** - 类型安全
- **Tailwind CSS** - 样式系统
- **Rust 1.70+** - 后端开发

<!-- MANUAL: -->
