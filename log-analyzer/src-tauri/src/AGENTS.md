<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# src (Rust后端源码)

## Purpose
Tauri后端核心业务逻辑，提供高性能的日志处理、搜索和存储服务。

## Key Files

| File | Description |
|------|-------------|
| `lib.rs` | 库入口，模块导出 |
| `main.rs` | 应用入口点 |
| `error.rs` | 全局错误类型定义 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `commands/` | Tauri IPC命令处理器 (see `commands/AGENTS.md`) |
| `services/` | 核心业务逻辑 (see `services/AGENTS.md`) |
| `models/` | 数据模型和状态 (see `models/AGENTS.md`) |
| `search_engine/` | 搜索引擎实现 (see `search_engine/AGENTS.md`) |
| `storage/` | 存储层 (CAS + SQLite) (see `storage/AGENTS.md`) |
| `archive/` | 压缩包处理 (see `archive/AGENTS.md`) |
| `task_manager/` | 异步任务管理 (see `task_manager/AGENTS.md`) |
| `utils/` | 工具模块 (see `utils/AGENTS.md`) |
| `state_sync/` | 状态同步 (see `state_sync/AGENTS.md`) |
| `monitoring/` | 性能监控 (see `monitoring/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 使用 `cargo fmt` 和 `cargo clippy` 格式化代码
- 错误处理使用 `?` 传播，生产代码消除 panic
- 使用 `parking_lot::Mutex` 和 `DashMap` 处理并发
- 锁不跨 `.await` 点持有

### Testing Requirements
- `cargo test --all-features --lib`
- 298个测试必须全部通过
- 属性测试使用 proptest

### Common Patterns
- 异步函数使用 `async fn` + `tokio`
- 错误类型使用 `thiserror` 定义
- 配置使用 `config` crate
- 日志使用 `tracing`

## Dependencies

### Workspace Crates
- `la-core` - 核心模型和错误类型
- `la-search` - 搜索引擎
- `la-storage` - 存储系统
- `la-archive` - 压缩包处理

### External
- **tauri** - 桌面框架
- **tokio** - 异步运行时
- **serde** - 序列化
- **sqlx** - SQLite访问
- **tantivy** - 全文搜索
- **aho-corasick** - 多模式匹配

<!-- MANUAL: -->
