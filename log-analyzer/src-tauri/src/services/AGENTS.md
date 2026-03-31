<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# services (核心业务服务)

## Purpose
核心业务逻辑层，实现日志搜索、查询处理、文件监听等核心功能。

## Key Files

| File | Description |
|------|-------------|
| `pattern_matcher.rs` | Aho-Corasick多模式匹配 |
| `query_executor.rs` | 查询执行器 |
| `query_validator.rs` | 查询验证器 |
| `query_planner.rs` | 查询计划器 |
| `file_watcher_async.rs` | 异步文件监听 |
| `search_statistics.rs` | 搜索统计 |

## For AI Agents

### Working In This Directory
- 三层架构：Validator → Planner → Executor
- 使用 Aho-Corasick 算法实现高性能匹配
- 并行处理使用 Rayon

### Testing Requirements
- 单元测试覆盖率 >80%
- 属性测试验证边界条件

### Common Patterns
- 使用 builder 模式构建复杂对象
- 错误使用 `Result<T, AppError>`
- 缓存使用 LRU 策略

## Dependencies

### Internal
- `models/` - 搜索模型
- `utils/` - 工具函数

### External
- **aho-corasick** - 多模式匹配
- **regex** - 正则表达式
- **rayon** - 并行处理
- **lru** - LRU缓存

<!-- MANUAL: -->
