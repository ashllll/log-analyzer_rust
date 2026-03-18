# 搜索引擎模块关键修复 Spec

## Why
根据深度分析报告，还有25个未修复问题需要处理，需在30分钟内优先完成高优先级修复。

## What Changes

### 高优先级修复（必须完成）
1. **ADV-M4**: `query_time_range` 无上界聚合 - 跨年查询内存累积
2. **SB-M1**: `cancel token` 未传入blocking线程 - 取消机制失效
3. **CSE-M1**: 并发平均值计算时序错误

### 中优先级修复（尽力完成）
4. **ADV-M1**: 空filter逻辑错误
5. **BQP-M1**: unwrap_or_default静默失败
6. **HLE-M1**: 查询解析失败降级丢词

## Impact
- Affected files: `advanced_features.rs`, `streaming_builder.rs`, `concurrent_search.rs`, `highlighting_engine.rs`, `boolean_query_processor.rs`
- 无破坏性变更，纯Bug修复

## ADDED Requirements

### Requirement: 时间分区查询内存保护
系统 SHALL 在 `query_time_range` 中限制聚合分区数量，防止跨百年查询导致内存耗尽。

#### Scenario: 跨年查询
- **WHEN** 用户查询跨越100年以上的时间范围
- **THEN** 系统最多聚合1000个分区，返回部分结果并记录警告

### Requirement: 取消令牌正确传递
系统 SHALL 在 `spawn_blocking` 闭包中正确传递和使用 `CancellationToken`。

#### Scenario: 用户取消索引
- **WHEN** 用户在索引过程中点击取消
- **THEN** blocking线程中的操作应立即检查并响应取消请求

### Requirement: 并发统计准确性
系统 SHALL 使用原子操作或正确同步机制计算并发统计。

#### Scenario: 并发查询统计
- **WHEN** 多个并发查询同时执行
- **THEN** 统计数据应准确反映实际执行情况

## MODIFIED Requirements

### Requirement: 空Filter处理
**MODIFIED**: 当 `filters` 为空时，应返回错误而非所有文档，避免逻辑混淆。

## 修复优先级

| 优先级 | 问题ID | 修复难度 | 预计时间 |
|--------|--------|---------|---------|
| P0 | ADV-M4 | 高 | 10min |
| P0 | SB-M1 | 低 | 5min |
| P1 | CSE-M1 | 中 | 5min |
| P2 | ADV-M1 | 中 | 5min |
| P2 | BQP-M1 | 低 | 3min |
| P2 | HLE-M1 | 低 | 2min |

**总计**: ~30分钟
