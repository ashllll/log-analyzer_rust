[根目录](../../CLAUDE.md) > **src-tauri (Rust后端)**

# Rust 后端架构文档

> Tauri 2.0 + Rust 1.70+ | 版本: 0.0.43

## 模块职责

Rust 后端是整个应用的核心，负责高性能的日志处理和搜索功能。采用现代化的 Rust 异步编程模式，提供：

- **高性能搜索**: Aho-Corasick 算法，O(n+m) 复杂度
- **多格式支持**: ZIP/TAR/GZ/RAR 压缩包递归解压
- **结构化查询**: Validator/Planner/Executor 三层架构
- **异步I/O**: tokio 实现非阻塞文件操作
- **并行处理**: Rayon 多线程加速
- **索引持久化**: Gzip 压缩存储，节省空间 50%+
- **实时监听**: 文件变化自动增量更新

## 入口与启动

### 核心入口文件

**main.rs** - 应用入口点
```rust
fn main() {
    log_analyzer::run()
}
```

**lib.rs** - 核心初始化
- 配置全局 panic hook
- 设置 Rayon 线程池（多核优化）
- 注册所有 Tauri 命令
- 初始化应用状态

**error.rs** - 统一错误处理
- 使用 `thiserror` 创建 `AppError` 枚举
- 支持错误链和上下文信息
- 统一的 `Result<T>` 类型定义

## 对外接口

### Tauri 命令接口 (commands/)

| 命令文件 | 功能描述 | 主要方法 |
|---------|---------|---------|
| **search.rs** | 日志搜索及缓存 | `search_logs()` |
| **import.rs** | 导入文件/文件夹 | `import_folder()` |
| **workspace.rs** | 工作区管理 | `load_workspace()`, `delete_workspace()` |
| **query.rs** | 结构化查询 | `execute_structured_query()` |
| **export.rs** | 导出搜索结果 | `export_results()` |
| **watch.rs** | 文件监听 | `start_watch()`, `stop_watch()` |
| **config.rs** | 配置管理 | `load_config()`, `save_config()` |
| **performance.rs** | 性能监控 | `get_performance_metrics()` |

### IPC 通信模式
- **前端调用**: `invoke('command_name', params)`
- **后端推送**: `app.emit('event_name', data)`
- **命令注册**: `tauri::command` 宏装饰

## 核心服务 (services/)

### 1. PatternMatcher - 模式匹配器
- **算法**: Aho-Corasick 多模式匹配
- **性能**: 搜索复杂度 O(n+m)，性能提升 80%+
- **特性**:
  - 支持大小写敏感/不敏感
  - 支持 AND/OR/NOT 逻辑
  - 支持正则表达式
  - 高性能匹配位置追踪

```rust
pub struct PatternMatcher {
    ac: Option<AhoCorasick>,
    patterns: Vec<String>,
    case_insensitive: bool,
}
```

### 2. QueryExecutor - 查询执行器
- **职责**: 协调 Validator/Planner/Executor 三层
- **特性**:
  - 查询验证和计划构建
  - 并行搜索执行
  - LRU 缓存支持
  - 搜索结果统计

### 3. QueryValidator - 查询验证器
- **功能**: 验证查询合法性
- **检查项**:
  - 空查询检测
  - 启用条件检查
  - 值长度限制
  - 正则表达式有效性
  - 重复项警告

### 4. QueryPlanner - 查询计划器
- **功能**: 构建执行计划
- **优化**:
  - 正则表达式缓存
  - 按优先级排序
  - 合并相同模式
  - 区分大小写优化

### 5. FileWatcher - 文件监听器
- **技术**: `notify` crate 实现文件系统监听
- **特性**:
  - 异步文件读取
  - 增量更新索引
  - 支持大文件分块读取
  - 文件变化检测

### 6. SearchStatistics - 搜索统计
- **功能**: 计算关键词统计信息
- **指标**:
  - 匹配数量和占比
  - 性能指标（延迟/吞吐量）
  - 缓存命中率

## 数据模型 (models/)

### SearchQuery - 搜索查询
```rust
pub struct SearchQuery {
    pub id: String,
    pub terms: Vec<SearchTerm>,
    pub global_operator: QueryOperator,
    pub filters: Option<SearchFilters>,
    pub metadata: QueryMetadata,
}
```

### SearchTerm - 搜索条件
- 支持多种操作符: AND/OR/NOT
- 支持正则表达式
- 优先级系统
- 启用/禁用控制

### AppState - 应用状态
- 工作区映射
- 搜索缓存
- 性能指标
- 文件监听状态

## 压缩包处理 (archive/)

### ArchiveHandler Trait
统一的压缩处理器接口，支持策略模式扩展。

### 支持格式
- **ZIP**: `zip` crate，高兼容性
- **TAR**: `tar` crate，支持 .tar/.tar.gz/.tgz
- **GZ**: `flate2` crate，纯 gzip 文件
- **RAR**: `unrar` crate，内置多平台二进制

### 安全特性
- 文件大小限制 (默认 100MB)
- 总大小限制 (默认 1GB)
- 文件数量限制 (默认 1000)
- 路径安全检查

### 处理器特性
```rust
pub struct ArchiveManager {
    handlers: Vec<Box<dyn ArchiveHandler>>,
    max_file_size: u64,
    max_total_size: u64,
    max_file_count: usize,
}
```

## 工具模块 (utils/)

### 核心工具
- **path.rs**: 路径处理，Windows UNC 支持
- **encoding.rs**: 多编码支持 (UTF-8/GBK/Windows-1252)
- **validation.rs**: 输入验证
- **retry.rs**: 重试机制
- **cleanup.rs**: 资源清理

## 关键依赖 (Cargo.toml)

### 核心依赖
```toml
# Tauri
tauri = { version = "2.0.0" }
tauri-plugin-dialog = "2.0.0"

# 性能优化
aho-corasick = "1.0"    # 多模式匹配
rayon = "1.8"           # 并行处理
lru = "0.12"            # LRU缓存
tokio = { version = "1", features = ["full"] }  # 异步I/O

# 错误处理
thiserror = "1.0"       # 统一错误处理
async-trait = "0.1"     # 异步trait支持

# 压缩支持
zip = "0.6"             # ZIP格式
tar = "0.4"             # TAR格式
flate2 = "1.0"          # GZIP压缩/解压
unrar = "0.5"           # RAR格式

# 系统支持
dunce = "1.0"           # Windows路径规范化
encoding_rs = "0.8"     # 多编码支持
notify = "6.1"          # 文件系统监听
sysinfo = "0.31"        # 系统信息
```

## 测试策略

### 测试覆盖
- **40+ 测试用例**
- **覆盖率: 80%+**

### 测试分类

#### 单元测试
- **pattern_matcher.rs**: 9个测试（算法正确性、性能、边界条件）
- **query_validator.rs**: 6个测试（验证逻辑、错误检测）
- **query_planner.rs**: 7个测试（计划构建、正则缓存）
- **file_watcher_async.rs**: 5个测试（异步读取、大文件处理）
- **search_statistics.rs**: 3个测试（统计计算）
- **error.rs**: 17个测试（错误处理、上下文）

#### 集成测试
- **tests/helper_functions.rs**: 9个测试（应用结构、文件操作、权限处理）

#### 压缩测试
- **archive/*.rs**: 各格式处理器测试
- **tar_handler.rs**: 3个测试（TAR/压缩TAR处理）
- **zip_handler.rs**: ZIP格式测试
- **rar_handler.rs**: RAR格式测试

### 运行测试
```bash
# 运行所有测试
cargo test --all-features

# 运行特定模块测试
cargo test pattern_matcher
cargo test query_validator

# 性能基准测试
cargo test --bench

# 代码覆盖率
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

## 性能优化

### 核心优化
1. **Aho-Corasick算法**: 多模式匹配，性能提升80%+
2. **并行搜索**: Rayon多线程，充分利用多核CPU
3. **LRU缓存**: 搜索结果缓存，减少重复计算
4. **异步I/O**: tokio非阻塞文件操作
5. **索引压缩**: Gzip压缩存储，节省空间50%+

### 基准测试
- **吞吐量**: 10,000+ 次搜索/秒
- **延迟**: 毫秒级响应
- **内存**: 优化的内存使用

## 常见问题 (FAQ)

### Q: 如何添加新的压缩格式？
A: 实现 `ArchiveHandler` Trait 并在 `ArchiveManager` 中注册。

### Q: 搜索性能优化建议？
A:
1. 使用具体搜索词减少结果数量
2. 利用关键词过滤功能
3. 避免过于宽泛的正则表达式
4. 启用查询缓存

### Q: 如何处理大文件？
A:
1. 文件监听器自动分块读取
2. 增量索引更新
3. 虚拟滚动优化渲染

### Q: Windows兼容性问题？
A:
1. 使用 `dunce` 处理UNC路径
2. 支持长路径（>260字符）
3. 自动处理只读文件

## 相关文件清单

### 核心文件
- `src/lib.rs` - 主库入口
- `src/main.rs` - 应用入口
- `src/error.rs` - 错误处理
- `Cargo.toml` - 依赖配置

### 服务层
- `src/services/pattern_matcher.rs` - 模式匹配
- `src/services/query_executor.rs` - 查询执行
- `src/services/query_validator.rs` - 查询验证
- `src/services/query_planner.rs` - 查询计划
- `src/services/file_watcher_async.rs` - 文件监听

### 命令层
- `src/commands/search.rs` - 搜索命令
- `src/commands/import.rs` - 导入命令
- `src/commands/workspace.rs` - 工作区命令

### 模型层
- `src/models/search.rs` - 搜索模型
- `src/models/state.rs` - 状态模型
- `src/models/search_statistics.rs` - 统计模型

### 压缩处理
- `src/archive/archive_handler.rs` - 处理器接口
- `src/archive/zip_handler.rs` - ZIP处理器
- `src/archive/rar_handler.rs` - RAR处理器
- `src/archive/tar_handler.rs` - TAR处理器
- `src/archive/gz_handler.rs` - GZ处理器

### 测试文件
- `tests/helper_functions.rs` - 集成测试
- `src/*/mod.rs` - 各模块内单元测试

---

## 变更记录 (Changelog)

### [2025-12-13] AI上下文初始化
- ✅ 完整模块架构分析
- ✅ 核心服务和命令梳理
- ✅ 测试覆盖统计完成
- ✅ 性能优化要点总结

### [2025-12-10] 架构重构
- ✅ QueryExecutor职责拆分 (Validator/Planner/Executor)
- ✅ Aho-Corasick算法集成
- ✅ 统一错误处理机制
- ✅ 异步I/O优化完成

---

*本文档由 AI 架构师自动生成，基于 Rust 后端代码分析*
