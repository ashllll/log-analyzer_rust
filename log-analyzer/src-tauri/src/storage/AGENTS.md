<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# storage (存储层)

## Purpose
内容寻址存储(CAS) + SQLite元数据存储，实现文件去重和元数据管理。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 模块入口 |
| `cas.rs` | 内容寻址存储实现 |
| `metadata.rs` | SQLite元数据存储 |
| `coordinator.rs` | 存储协调器(Saga事务) |

## For AI Agents

### Working In This Directory
- CAS使用 SHA-256 作为文件键
- SQLite使用 sqlx 进行异步操作
- 协调器使用事务保证原子性

### Testing Requirements
- CAS存储/检索测试
- 元数据CRUD测试
- 并发安全测试

### Common Patterns
- INSERT OR IGNORE + SELECT 处理并发
- 使用 DashSet 缓存对象存在性
- WAL模式提升SQLite性能

## Dependencies

### External
- **sqlx** - SQLite异步访问
- **dashmap** / **dashset** - 并发集合
- **walkdir** - 目录遍历

<!-- MANUAL: -->
