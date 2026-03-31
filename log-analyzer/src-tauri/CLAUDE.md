[根目录](../../CLAUDE.md) > **src-tauri (Rust 后端)**

# Rust 后端架构文档

> Tauri 2.0 + Rust 1.70+ | 最后更新: 2026-03-31

## 架构说明

后端采用 **Cargo Workspace** 架构，主 crate (`src-tauri`) 依赖 4 个 workspace crate：

| Crate | 路径 | 职责 |
|-------|------|------|
| **la-core** | `crates/la-core/` | 共享错误类型 (`AppError`, `CommandError`)、数据模型、trait |
| **la-storage** | `crates/la-storage/` | CAS (`cas.rs`)、SQLite 元数据 (`metadata_store.rs`)、Saga 事务 |
| **la-search** | `crates/la-search/` | Tantivy 索引、搜索管理器、`ReaderPool`、高亮引擎、流式构建器 |
| **la-archive** | `crates/la-archive/` | 压缩包处理 (ZIP/TAR/GZ/RAR/7Z)、安全检测、递归深度限制 |

其余模块保留在主 crate 中：
- `commands/` - Tauri IPC 命令层 (16+ 子模块)
- `services/` - 业务逻辑：QueryExecutor (Validator→Planner→Executor)、PatternMatcher、FileWatcher
- `models/` - 数据模型 + AppState (`parking_lot::Mutex`)
- `task_manager/` - Actor 模型异步任务管理 (有界 `mpsc` 通道 + 版本号幂等)
- `state_sync/` - Tauri 事件状态同步
- `monitoring/` - 性能指标收集 (`MetricsCollector`)
- `utils/` - 工具函数

## 入口文件

- `src/main.rs` - 应用入口：`log_analyzer::run()`
- `src/lib.rs` - 模块导出、全局 panic hook、Rayon 线程池设置
- `src/error.rs` - 主 crate 错误处理（注意：共享错误类型在 `la-core::error`）

## 核心外部依赖

```toml
tauri = "2.0.0"
tokio = { version = "1", features = ["full"] }
aho-corasick = "1.1"
tantivy = { version = "0.22", features = ["mmap"] }
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "sqlite"] }
rayon = "1.8"
parking_lot = "0.12"
dashmap = "6"
```

## 测试

```bash
# 在主 crate 目录下执行
cargo test --workspace --all-features
cargo test --all-features pattern_matcher
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt -- --check
```

## 2026-03-31 集中修复摘要

- `cache_key` 哈希逻辑补全（`services/query_executor.rs`）
- 无界通道替换为有界通道 `tokio::sync::mpsc::channel(1000)`（`task_manager/`）
- 孤儿文件清理竞态条件修复（`la-storage/src/coordinator.rs`，数据库事务原子性）
- `ReaderPool.acquire()` 实现（`la-search/src/concurrent_search.rs`）
- 已移除模块：`services/fuzzy_matcher.rs`、`services/metaphone.rs`（2026-03-22 清理）

## 相关文件

- `src/services/pattern_matcher.rs` - Aho-Corasick 模式匹配
- `src/services/query_executor.rs` - 查询执行协调器
- `src/services/query_validator.rs` - 查询验证器
- `src/services/query_planner.rs` - 查询计划器
- `crates/la-search/src/concurrent_search.rs` - `ReaderPool` 并发搜索
- `crates/la-storage/src/cas.rs` - CAS 内容寻址存储
- `crates/la-archive/src/` - 压缩包处理 crate

---

*详细架构规范、编码规范、故障排查请参见根目录 [CLAUDE.md](../../CLAUDE.md)*
