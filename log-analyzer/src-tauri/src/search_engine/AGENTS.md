<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# search_engine (搜索引擎)

## Purpose
基于 Tantivy 的全文搜索引擎，支持索引管理、搜索执行和结果存储。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 模块入口 |
| `manager.rs` | 搜索引擎管理器 |
| `index.rs` | 索引操作 |
| `streaming_builder.rs` | 流式索引构建 |
| `concurrent_search.rs` | 并发搜索 |
| `pattern_matcher.rs` | 模式匹配 |

## For AI Agents

### Working In This Directory
- 使用 Tantivy 0.22 实现全文搜索
- Aho-Corasick 实现多模式匹配
- 搜索结果使用磁盘存储避免内存溢出

### Testing Requirements
- 属性测试验证搜索正确性
- 性能测试验证响应时间

### Common Patterns
- ReaderPool 管理 Tantivy IndexReader
- 批量导入后调用 commit_and_wait_merge
- 使用 DiskResultStore 存储结果

## Dependencies

### External
- **tantivy** - 全文搜索引擎
- **aho-corasick** - 多模式匹配
- **rayon** - 并行处理

<!-- MANUAL: -->
