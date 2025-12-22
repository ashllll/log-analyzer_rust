# 性能优化指南

## 概述

本文档介绍日志分析器的性能优化功能和配置方法。系统采用业界成熟的技术栈，提供高性能的搜索、缓存和实时同步能力。

## 核心技术

### 1. Tantivy 搜索引擎

**特性：**
- Rust 原生全文搜索引擎，性能优于 Elasticsearch（单节点场景）
- 支持 O(log n) 复杂度的搜索查询
- 内存映射文件访问，支持大数据集（>1GB）
- 流式索引构建，处理超过可用 RAM 的数据集

**性能指标：**
- 搜索响应时间：< 200ms（100MB 数据集）
- 多关键词查询：< 1秒
- 缓存命中响应：< 50ms

### 2. 多层缓存系统

**架构：**
- **L1 缓存**：Moka 内存缓存，< 0.1ms 延迟
- **L2 缓存**：Redis 分布式缓存（可选）

**特性：**
- LRU 淘汰策略
- TTL 自动过期
- 智能缓存预热
- 模式匹配失效

### 3. 实时状态同步（可选）

**技术：**
- Tauri Events（进程内通信，< 10ms 延迟）
- WebSocket（可选，用于分布式场景）

**特性：**
- 自动重连
- 事件历史记录
- 状态一致性保证

## 配置指南

### 基础配置

配置文件位置：`src-tauri/config/performance.toml`

```toml
[search_engine]
default_timeout_ms = 200
max_results = 50000
index_path = "./search_index"

[cache]
l1_max_capacity = 1000
l1_ttl_seconds = 3600
enable_l2_cache = false
```

### 性能调优建议

#### 1. 搜索性能优化

**小数据集（< 100MB）：**
```toml
[search_engine]
default_timeout_ms = 100
writer_heap_size = 25000000  # 25MB
```

**大数据集（> 1GB）：**
```toml
[search_engine]
default_timeout_ms = 500
writer_heap_size = 100000000  # 100MB

[concurrency]
parallel_indexing_threads = 8  # 根据 CPU 核心数调整
```

#### 2. 缓存优化

**内存充足（> 8GB）：**
```toml
[cache]
l1_max_capacity = 5000
l1_ttl_seconds = 7200  # 2小时
```

**内存受限（< 4GB）：**
```toml
[cache]
l1_max_capacity = 500
l1_ttl_seconds = 1800  # 30分钟
```

#### 3. 并发优化

**高并发场景：**
```toml
[concurrency]
max_concurrent_searches = 20
reader_pool_size = 8
```

**低并发场景：**
```toml
[concurrency]
max_concurrent_searches = 5
reader_pool_size = 2
```

## 性能监控

### 查看性能指标

系统自动收集以下指标：
- 搜索响应时间（p50, p95, p99）
- 缓存命中率
- 系统资源使用（CPU、内存）
- 查询吞吐量

### 性能告警

当性能指标超过阈值时，系统会生成告警：

```toml
[monitoring.thresholds]
search_response_time_ms = 200
cache_response_time_ms = 50
cpu_usage_percent = 80
memory_usage_percent = 90
```

### 查看优化建议

系统会根据运行状态自动生成优化建议：
- 查询优化建议
- 索引调优建议
- 缓存配置建议
- 资源分配建议

## 自动调优

### 启用自动调优

```toml
[auto_tuning]
enable_index_optimizer = true
enable_cache_tuner = true
enable_dynamic_optimizer = true
optimization_interval_seconds = 300
```

### 自动调优功能

1. **索引优化器**
   - 检测频繁查询模式
   - 自动创建专用索引
   - 优化索引维护计划

2. **缓存调优器**
   - 动态调整缓存大小
   - 优化淘汰策略
   - 预测性数据预加载

3. **动态优化器**
   - 自动资源分配
   - 负载均衡
   - 性能瓶颈检测

## 常见问题排查

### 搜索速度慢

**症状：** 搜索响应时间 > 500ms

**排查步骤：**
1. 检查数据集大小
2. 查看缓存命中率
3. 检查并发查询数
4. 查看系统资源使用

**解决方案：**
- 增加缓存容量
- 启用查询优化
- 增加索引读取器池大小
- 优化查询语句

### 内存使用过高

**症状：** 内存使用 > 90%

**排查步骤：**
1. 检查缓存配置
2. 查看索引大小
3. 检查并发操作数

**解决方案：**
- 减小缓存容量
- 启用智能淘汰
- 限制并发查询数
- 启用压缩

### 缓存命中率低

**症状：** 缓存命中率 < 30%

**排查步骤：**
1. 检查 TTL 配置
2. 查看查询模式
3. 检查缓存容量

**解决方案：**
- 增加 TTL
- 增加缓存容量
- 启用缓存预热
- 优化查询模式

## 性能基准测试

### 运行基准测试

```bash
cargo bench --manifest-path log-analyzer/src-tauri/Cargo.toml
```

### 基准测试结果示例

```
search_100mb_dataset    time: [180.23 ms 185.45 ms 190.67 ms]
cache_hit_latency       time: [42.15 µs 45.23 µs 48.31 µs]
multi_keyword_search    time: [850.12 ms 875.34 ms 900.56 ms]
concurrent_searches     time: [195.67 ms 200.89 ms 206.11 ms]
```

## 生产环境部署

### 推荐配置

**标准配置（4核8GB）：**
```toml
[search_engine]
default_timeout_ms = 200
max_results = 50000

[cache]
l1_max_capacity = 2000
l1_ttl_seconds = 3600

[concurrency]
max_concurrent_searches = 10
reader_pool_size = 4
```

**高性能配置（8核16GB）：**
```toml
[search_engine]
default_timeout_ms = 150
max_results = 100000

[cache]
l1_max_capacity = 5000
l1_ttl_seconds = 7200
enable_l2_cache = true

[concurrency]
max_concurrent_searches = 20
reader_pool_size = 8
```

### 监控集成

系统支持集成以下监控工具：
- Sentry（错误监控）
- 自定义指标导出
- 日志聚合

## 最佳实践

1. **定期清理索引**：删除不再需要的工作区索引
2. **监控缓存命中率**：保持 > 60% 的命中率
3. **限制并发查询**：避免系统过载
4. **使用查询缓存**：重复查询自动缓存
5. **启用自动调优**：让系统自动优化性能
6. **定期查看优化建议**：根据建议调整配置

## 技术支持

如遇性能问题，请提供以下信息：
- 系统配置（CPU、内存）
- 数据集大小
- 查询模式
- 性能指标快照
- 错误日志

## 更新日志

### v0.0.62
- ✅ 集成 Tantivy 搜索引擎
- ✅ 实现多层缓存系统
- ✅ 添加性能监控和告警
- ✅ 实现自动调优系统
- ✅ 完成所有属性测试

---

**文档版本：** 1.0  
**最后更新：** 2025-12-22
