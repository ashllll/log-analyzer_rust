<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# src-tauri (Rust 后端)

## Purpose
Tauri 2.0 后端主 crate，提供日志分析的核心业务能力。采用 Cargo Workspace 架构，主 crate 依赖 4 个 workspace crate。

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | 主 crate 配置，workspace 定义 |
| `src/main.rs` | 应用入口：`log_analyzer::run()` |
| `src/lib.rs` | 模块导出、全局 panic hook、Rayon 线程池设置 |
| `src/error.rs` | 主 crate 错误处理 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `src/` | 主 crate 源码（命令、服务、模型等） |
| `crates/` | Workspace crates（见各 crate AGENTS.md） |
| `tests/` | 集成测试 |

## For AI Agents

### Working In This Directory
- 使用 `cargo test --workspace` 测试全部 workspace
- 修改 crate 源码后需重新编译整个 workspace
- 遵循 Rust 1.70+ 编码规范

### Common Patterns
- 命令层使用 `tauri::command` 宏
- 服务层使用依赖注入模式
- 状态管理使用 `parking_lot::Mutex`

## Dependencies

### Internal (Workspace Crates)
- `la-core` - 共享错误类型和数据模型
- `la-storage` - CAS 存储和 SQLite 元数据
- `la-search` - Tantivy 全文搜索
- `la-archive` - 压缩包处理

### External
- `tauri = "2.0.0"` - 桌面应用框架
- `tokio` - 异步运行时
- `rayon` - 数据并行

<!-- MANUAL: -->
