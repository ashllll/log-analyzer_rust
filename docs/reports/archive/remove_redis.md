# Redis 移除计划

## 背景
本地单机应用不需要分布式缓存（Redis L2），只需要 L1 Moka 内存缓存即可。

## 需要移除的内容

### 1. Cargo.toml
- 移除 `redis` 依赖

### 2. cache_manager.rs
需要移除的字段和方法：
- `CacheConfig::enable_l2_cache`
- `CacheConfig::redis_url`
- `CacheConfig::l2_prefix`
- `CacheMetrics::l2_hit_count`
- `CacheMetrics::l2_miss_count`
- `CacheMetrics::record_l2_hit()`
- `CacheMetrics::record_l2_miss()`
- `CacheMetricsSnapshot::l2_hit_count`
- `CacheMetricsSnapshot::l2_miss_count`
- `CacheMetricsSnapshot::l2_hit_rate`
- `CacheManager::redis_conn`
- `CacheManager::invalidate_l2_workspace_cache()`
- 所有 Redis 连接初始化代码
- 所有 L2 缓存读写代码

### 3. cache_tuner.rs
- 移除 `CacheTunerConfig::redis_url`

### 4. dynamic_optimizer.rs
- 移除 "Enable L2 Cache" 推荐

### 5. 文档
- 更新所有提到 Redis 的文档
- 更新 tasks.md 标记 Redis 任务为已移除

## 保留的成熟功能
- ✅ L1 Moka 缓存（企业级，基于 Caffeine）
- ✅ 智能缓存失效
- ✅ 访问模式追踪
- ✅ 缓存压缩
- ✅ 性能监控和告警
- ✅ 缓存预热
- ✅ 自动驱逐策略

这些都是成熟的、生产就绪的功能。
