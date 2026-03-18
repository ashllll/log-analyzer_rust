# 搜索引擎最终优化 Spec

## Why
TODO.md中还有6个部分修复的问题需要进一步优化，以及3处async测试问题需要修复。

## What Changes

### 部分修复问题优化
1. **BQP-H3**: Tantivy无法中止 - 段级别已支持，文档级需改进
2. **BQP-H5**: 成本估计算法 - 需更精确的选择性计算
3. **HLE-H4**: chars().count() 每次O(n) - 需缓存计数
4. **ADV-H1**: 时间戳0值分区 - 需更严格的验证
5. **ADV-M3**: count_nodes递归 - 需改迭代
6. **BQP-M3**: limit常量 - 已优化

### 测试问题修复
7. **测试async**: manager.rs和property_tests.rs中的tokio::test问题

## Impact
- Affected files: `boolean_query_processor.rs`, `highlighting_engine.rs`, `advanced_features.rs`
- 无破坏性变更

## 修复优先级

| 优先级 | 问题ID | 修复难度 | 预计时间 |
|--------|--------|---------|---------|
| P1 | BQP-H3 | 中 | 15min |
| P1 | ADV-H1 | 低 | 10min |
| P2 | BQP-H5 | 高 | 20min |
| P2 | HLE-H4 | 中 | 15min |
| P3 | ADV-M3 | 中 | 10min |
| P3 | BQP-M3 | 低 | 5min |

**总计**: ~75分钟
