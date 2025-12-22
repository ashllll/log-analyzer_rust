# 生产监控和性能仪表板

本文档描述如何设置和使用生产环境的监控和性能追踪系统。

## 概述

应用程序集成了以下监控工具：

1. **Sentry** - 错误追踪和性能监控
2. **Tracing** - 结构化日志和分布式追踪
3. **Criterion** - 性能基准测试和回归检测
4. **Property-Based Testing** - 正确性验证

## Sentry 配置

### 环境变量

在生产环境中设置以下环境变量：

```bash
# Sentry DSN（从 Sentry 项目设置中获取）
export SENTRY_DSN="https://your-dsn@sentry.io/project-id"

# 环境名称
export ENVIRONMENT="production"  # 或 "staging", "development"

# 应用版本（自动从 Cargo.toml 读取）
# 无需手动设置
```

### 初始化

在应用启动时，Sentry 会自动初始化：

```rust
use log_analyzer::monitoring::{init_sentry_monitoring, SentryMonitoringConfig};

fn main() {
    // 生产环境配置
    let config = SentryMonitoringConfig::production();
    let _guard = init_sentry_monitoring(config);
    
    // 应用程序代码...
}
```

### 性能追踪

使用性能事务追踪关键操作：

```rust
use log_analyzer::monitoring::performance::PerformanceTransaction;

async fn search_logs(query: &str) -> Result<Vec<LogEntry>> {
    let mut transaction = PerformanceTransaction::start("search_logs");
    transaction.set_tag("query", query);
    
    // 执行搜索...
    let results = perform_search(query).await?;
    
    transaction.set_data("result_count", results.len());
    transaction.finish();
    
    Ok(results)
}
```

### 错误监控

捕获和报告错误：

```rust
use log_analyzer::monitoring::error_monitoring;

fn process_file(path: &str) -> Result<()> {
    match read_file(path) {
        Ok(content) => process_content(content),
        Err(e) => {
            error_monitoring::capture_error(&e, "file_processing");
            Err(e)
        }
    }
}
```

### 性能指标

记录自定义性能指标：

```rust
use log_analyzer::monitoring::performance;

// 缓存性能
performance::record_cache_metrics(
    hit_rate,        // 命中率 (0.0-1.0)
    eviction_count,  // 驱逐次数
    cache_size       // 缓存大小
);

// 搜索性能
performance::record_search_metrics(
    query_time_ms,   // 查询时间（毫秒）
    result_count,    // 结果数量
    files_scanned    // 扫描文件数
);
```

## 性能基准测试

### 运行基准测试

```bash
cd log-analyzer/src-tauri

# 运行所有基准测试
cargo bench

# 运行特定基准测试
cargo bench --bench production_benchmarks

# 生成 HTML 报告
cargo bench -- --output-format bencher | tee benchmark-results.txt
```

### 基准测试套件

1. **production_cache** - 缓存性能测试
   - 插入操作性能
   - 读取操作性能
   - 并发访问性能

2. **production_search** - 搜索性能测试
   - 简单关键词搜索
   - 正则表达式搜索
   - 多条件搜索
   - 大数据集搜索

3. **production_validation** - 验证性能测试
   - 工作区配置验证
   - 搜索查询验证
   - 路径安全验证

4. **production_concurrent** - 并发性能测试
   - 多线程缓存操作
   - 并发搜索请求
   - 资源竞争测试

5. **production_load** - 负载测试
   - 高并发场景
   - 大数据量处理
   - 长时间运行稳定性

6. **regression_detection** - 回归检测
   - 基线性能对比
   - 自动回归警告

### 性能基线

当前性能基线（参考值）：

| 操作 | 基线时间 | 阈值 |
|------|---------|------|
| 缓存插入 | < 1ms | 1ms |
| 缓存读取 | < 100µs | 100µs |
| 搜索（1000条） | < 10ms | 10ms |
| 配置验证 | < 500µs | 500µs |

### CI/CD 集成

性能测试已集成到 CI/CD 流程：

1. **Pull Request** - 自动运行基准测试并与基线对比
2. **Main Branch** - 更新性能基线
3. **性能报告** - 自动生成并评论到 PR

查看 `.github/workflows/performance-regression.yml` 了解详情。

## 属性测试

### 运行属性测试

```bash
cd log-analyzer/src-tauri

# 运行所有属性测试
cargo test --lib property

# 运行特定属性测试
cargo test --lib property_22  # 路径遍历保护
cargo test --lib property_8   # 死锁预防

# 增加测试用例数量
PROPTEST_CASES=10000 cargo test --lib property
```

### 已实现的属性

#### 错误处理属性
- **Property 2**: 错误类型一致性
- **Property 4**: 错误传播一致性
- **Property 6**: 归档错误详情
- **Property 7**: 搜索错误通信

#### 并发安全属性
- **Property 8**: 死锁预防
- **Property 9**: 线程安全缓存访问
- **Property 10**: 工作区状态保护

#### 资源管理属性
- **Property 17**: 临时目录清理
- **Property 19**: 搜索取消

#### 验证属性
- **Property 22**: 路径遍历保护
- **Property 23**: 工作区 ID 安全

## 日志和追踪

### 日志级别

设置日志级别：

```bash
# 环境变量
export RUST_LOG=info  # trace, debug, info, warn, error

# 或在代码中
tracing_subscriber::fmt()
    .with_env_filter("info")
    .init();
```

### 结构化日志

使用 tracing 宏记录结构化日志：

```rust
use tracing::{info, warn, error, debug, trace};

// 简单日志
info!("Application started");

// 带字段的日志
info!(
    workspace_id = %workspace_id,
    file_count = files.len(),
    "Workspace loaded successfully"
);

// 错误日志
error!(
    error = %e,
    path = %file_path,
    "Failed to read file"
);
```

### 日志文件

日志文件位置：

- **开发环境**: `log-analyzer/src-tauri/logs/app.log`
- **生产环境**: 配置的日志目录

日志轮转配置：

- 每日轮转
- 保留 7 天
- 最大文件大小 10MB

## 性能仪表板

### Sentry 仪表板

访问 Sentry 项目查看：

1. **错误追踪**
   - 错误频率和趋势
   - 错误堆栈跟踪
   - 受影响用户数

2. **性能监控**
   - 事务性能
   - 慢查询识别
   - 性能趋势

3. **发布健康**
   - 崩溃率
   - 会话统计
   - 版本对比

### 自定义指标

在 Sentry 中查看自定义指标：

- `cache.hit_rate` - 缓存命中率
- `cache.eviction_count` - 缓存驱逐次数
- `search.query_time` - 搜索查询时间
- `search.result_count` - 搜索结果数量

## 告警配置

### Sentry 告警

在 Sentry 项目设置中配置告警：

1. **错误率告警**
   - 错误率超过阈值
   - 新错误类型出现

2. **性能告警**
   - 响应时间超过阈值
   - 吞吐量下降

3. **发布告警**
   - 新版本崩溃率高
   - 性能回归

### 告警通知

配置告警通知渠道：

- Email
- Slack
- PagerDuty
- Webhook

## 故障排查

### 性能问题

1. 检查 Sentry 性能仪表板
2. 查看慢查询日志
3. 运行性能基准测试对比
4. 检查缓存命中率

### 错误追踪

1. 在 Sentry 中查看错误详情
2. 检查错误堆栈跟踪
3. 查看相关日志
4. 检查用户操作路径（面包屑）

### 资源泄漏

1. 运行资源追踪属性测试
2. 检查临时目录清理日志
3. 监控内存使用趋势
4. 使用 Valgrind 或类似工具

## 最佳实践

### 性能监控

1. **关键路径追踪** - 为所有关键操作添加性能事务
2. **合理采样** - 生产环境使用 5-10% 采样率
3. **定期审查** - 每周审查性能趋势
4. **基线更新** - 每次发布后更新性能基线

### 错误监控

1. **上下文信息** - 捕获错误时添加足够的上下文
2. **用户隐私** - 不要记录敏感信息
3. **错误分组** - 合理配置错误分组规则
4. **及时响应** - 设置合理的告警阈值

### 测试策略

1. **持续测试** - 在 CI/CD 中运行所有测试
2. **性能回归** - 每次 PR 都运行基准测试
3. **属性测试** - 增加测试用例数量以发现边缘情况
4. **负载测试** - 定期进行负载测试

## 参考资源

- [Sentry 文档](https://docs.sentry.io/)
- [Tracing 文档](https://docs.rs/tracing/)
- [Criterion 文档](https://bheisler.github.io/criterion.rs/)
- [Proptest 文档](https://altsysrq.github.io/proptest-book/)
