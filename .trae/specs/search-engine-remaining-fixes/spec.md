# 搜索引擎剩余缺陷修复 Spec

## Why
TODO.md中还有22个未修复问题需要处理，包括高/中优先级的关键缺陷和多个低优先级的代码质量问题。

## What Changes

### 高优先级修复
1. **HLE-M2**: SnippetCacheKey Hash依赖Debug格式不稳定
2. **CSE-M2**: ReaderPool创建失败无降级处理
3. **SEM-M1**: SearchError不区分可恢复/不可恢复错误
4. **MAN-L1**: 多关键词判断未考虑引号短语

### 中优先级修复
5. **ADV-L1**: document_count只记录最大ID，删除后不更新
6. **ADV-L2**: Regex编译失败错误消息不含pattern
7. **ADV-L3**: 负时间戳分区键计算错误
8. **IOP-L1**: created_indexes Vec无去重

### 低优先级修复
9. **IOP-L4**: 时钟调回cleanup_old_patterns跳过
10. **QOP-L1**: 硬编码top 3建议数
11. **SEM-L1**: delete_file_documents持锁期间commit

## Impact
- Affected files: `highlighting_engine.rs`, `concurrent_search.rs`, `advanced_features.rs`, `index_optimizer.rs`, `query_optimizer.rs`, `manager.rs`
- 无破坏性变更，纯Bug修复

## 修复优先级

| 优先级 | 问题ID | 修复难度 | 预计时间 |
|--------|--------|---------|---------|
| P1 | HLE-M2 | 低 | 5min |
| P1 | CSE-M2 | 中 | 10min |
| P2 | SEM-M1 | 高 | 15min |
| P2 | MAN-L1 | 中 | 10min |
| P3 | ADV-L1/L2/L3 | 低 | 10min |
| P4 | IOP-L1/L4, QOP-L1, SEM-L1 | 低 | 10min |

**总计**: ~60分钟
