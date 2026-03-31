[根目录](../../../CLAUDE.md) > [src-tauri](../) > **services (核心业务服务)**

# 核心业务服务模块文档

> 高性能日志搜索与查询执行引擎 | 最后更新: 2026-03-31

## 模块职责

Services 模块是核心业务逻辑层，负责：

- **高性能搜索**: Aho-Corasick 多模式匹配
- **结构化查询**: Validator→Planner→Executor 三层架构
- **异步文件处理**: tokio 非阻塞 I/O
- **并行搜索**: Rayon 多线程加速
- **文件监听**: 实时检测文件变化

## 三层查询架构

```
SearchQuery → QueryValidator → QueryPlanner → QueryExecutor → SearchResults
                ↓                  ↓               ↓
            空查询检查        正则编译缓存      并行搜索
            启用项检查        优先级排序       模式匹配
            正则验证         执行计划制定      结果聚合
```

## 核心服务

### 1. PatternMatcher (`pattern_matcher.rs`)

**算法**: Aho-Corasick 多模式匹配

- `matches_all()` - AND 逻辑匹配
- `matches_any()` - OR 逻辑匹配
- `find_matches()` - 匹配详情返回

**性能**: 时间复杂度 O(n + m + z)，相比朴素算法提升 80%+

### 2. QueryValidator (`query_validator.rs`)

验证查询合法性：
- 空查询检查
- 启用项检查
- 正则表达式验证
- 值长度限制

### 3. QueryPlanner (`query_planner.rs`)

构建执行计划：
- 正则表达式编译和缓存
- 优先级排序（高优先级优先匹配）
- 执行策略制定（Sequential/Parallel/Mixed）

### 4. QueryExecutor (`query_executor.rs`)

协调三层架构，执行搜索：
```rust
pub async fn execute_query(
    &self,
    query: &SearchQuery,
    workspace_paths: &[PathBuf],
    max_results: usize,
) -> Result<SearchResults>
```

**注意**: `generate_cache_key()` 在 2026-03-31 修复，现在包含所有查询字段哈希

### 5. FileWatcher (`file_watcher.rs`)

- `read_file_from_offset()` - 异步偏移读取
- 大文件分块处理（8KB buffer）
- 文件变化监听（`notify` crate）

### 6. SearchStatistics (`search_statistics.rs`)

搜索结果统计分析：
- 关键词匹配数量
- 占比计算
- 颜色编码分配

## 相关文件

- `pattern_matcher.rs` - Aho-Corasick 模式匹配
- `query_executor.rs` - 查询执行协调器
- `query_validator.rs` - 查询验证器
- `query_planner.rs` - 查询计划器
- `search_statistics.rs` - 搜索统计
- `file_watcher.rs` - 文件监听

---

*详细架构规范请参见根目录 [CLAUDE.md](../../../CLAUDE.md)*
