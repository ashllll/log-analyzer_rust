<!-- Parent: ../../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# la-storage (存储库)

## Purpose
存储层库，提供CAS内容寻址存储和SQLite元数据存储。

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Crate配置 |
| `src/lib.rs` | 库入口 |
| `src/cas.rs` | CAS实现 |
| `src/metadata_store.rs` | 元数据存储 |
| `src/coordinator.rs` | 存储协调器 |

## For AI Agents

### Working In This Directory
- CAS使用SHA-256去重
- SQLite使用WAL模式
- 协调器使用Saga模式保证一致性

### Testing Requirements
- 存储/检索测试
- 事务一致性测试

## Dependencies

### Internal
- `la-core` - 核心模型

### External
- **sqlx** - SQLite访问
- **dashmap** - 并发缓存

<!-- MANUAL: -->
