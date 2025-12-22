# 生产部署指南

## 概述

本文档提供了将日志分析器应用程序部署到生产环境的完整指南，包括所有重大变更、迁移步骤、故障排除和回滚程序。

## 目录

1. [重大变更](#重大变更)
2. [迁移步骤](#迁移步骤)
3. [性能调优建议](#性能调优建议)
4. [故障排除指南](#故障排除指南)
5. [回滚程序](#回滚程序)
6. [服务配置](#服务配置)
7. [监控和告警](#监控和告警)

---

## 重大变更

### Phase 1: 核心基础设施

#### 1.1 错误处理系统 (eyre + miette)

**变更内容:**
- 将自定义 `AppError` 替换为 `eyre::Result<T>`
- 使用 `miette` 提供用户友好的错误诊断
- 集成 `color-eyre` 用于增强的错误报告

**影响:**
- 所有返回 `Result<T, AppError>` 的函数现在返回 `eyre::Result<T>`
- 错误消息格式已更改，包含更多上下文信息
- 错误堆栈跟踪现在自动捕获

**迁移要求:**
- 更新所有错误处理代码以使用 `eyre::Result`
- 使用 `.context()` 方法添加错误上下文
- 在 `main()` 中初始化 `color_eyre::install()`

#### 1.2 结构化日志 (tracing)

**变更内容:**
- 将所有 `println!` 和 `eprintln!` 替换为 `tracing` 宏
- 实现 JSON 日志输出用于生产环境
- 添加日志轮转和级别过滤

**影响:**
- 日志格式已更改为结构化 JSON
- 日志文件现在自动轮转
- 支持动态日志级别配置

**迁移要求:**
- 配置 `tracing-subscriber` 用于日志输出
- 设置日志文件路径和轮转策略
- 配置日志级别（开发环境：DEBUG，生产环境：INFO）

#### 1.3 错误监控 (Sentry)

**变更内容:**
- 集成 Sentry 用于错误跟踪和性能监控
- 自动捕获 panic 和 eyre 错误
- 添加性能事务跟踪

**影响:**
- 所有错误自动报告到 Sentry
- 性能指标自动收集
- 需要 Sentry DSN 配置

**迁移要求:**
- 在环境变量中设置 `SENTRY_DSN`
- 配置 Sentry 采样率
- 设置发布版本标识

### Phase 2: 并发和性能

#### 2.1 高性能锁 (parking_lot)

**变更内容:**
- 将 `std::sync::Mutex` 替换为 `parking_lot::Mutex`
- 将 `std::sync::RwLock` 替换为 `parking_lot::RwLock`
- 添加超时机制 `try_lock_for()`

**影响:**
- 锁性能提升 2-3倍
- 内置死锁检测
- 更公平的锁调度

**迁移要求:**
- 更新所有锁类型导入
- 添加超时处理逻辑
- 测试并发场景

#### 2.2 无锁数据结构 (crossbeam)

**变更内容:**
- 使用 `crossbeam::queue::SegQueue` 替代互斥队列
- 使用 `crossbeam::channel` 用于高吞吐量消息传递

**影响:**
- 队列操作性能提升 5-10倍
- 减少锁竞争
- 更好的并发扩展性

**迁移要求:**
- 替换所有 `Mutex<Vec<T>>` 为 `SegQueue<T>`
- 更新通道创建代码
- 测试高并发场景

#### 2.3 企业级缓存 (moka)

**变更内容:**
- 将 `lru::LruCache` 替换为 `moka::Cache`
- 添加 TTL (5分钟) 和 TTI (1分钟) 过期策略
- 实现异步缓存操作

**影响:**
- 缓存命中率提升 20-30%
- 自动过期和驱逐
- 内置缓存指标

**迁移要求:**
- 更新缓存创建代码
- 配置 TTL/TTI 策略
- 实现缓存预热逻辑

### Phase 3: 前端状态管理

#### 3.1 现代状态管理 (zustand + react-query)

**变更内容:**
- 将 Context+Reducer 替换为 zustand
- 使用 `@tanstack/react-query` 管理服务器状态
- 实现自动任务去重

**影响:**
- 状态管理代码减少 50%
- 自动后台重新获取
- 乐观更新和自动回滚

**迁移要求:**
- 重构所有 Context 为 zustand stores
- 将 API 调用迁移到 react-query
- 更新组件以使用新的 hooks

#### 3.2 原生事件管理 (React)

**变更内容:**
- 移除第三方事件库
- 使用 React 内置事件系统
- 实现 `useEffect` 清理模式

**影响:**
- 减少依赖
- 自动内存泄漏防护
- 更好的 React DevTools 集成

**迁移要求:**
- 重构事件监听器
- 添加清理函数
- 测试组件卸载

### Phase 4: 验证和资源管理

#### 4.1 生产验证框架 (validator)

**变更内容:**
- 使用 `validator` 框架进行结构化验证
- 添加路径安全验证
- 实现归档提取限制

**影响:**
- 自动验证错误报告
- 增强的安全性
- 一致的验证规则

**迁移要求:**
- 为所有输入添加验证
- 配置验证规则
- 处理验证错误

#### 4.2 自动资源管理 (scopeguard + tokio-util)

**变更内容:**
- 使用 `scopeguard` 实现 RAII 模式
- 添加 `tokio-util::CancellationToken` 用于优雅取消
- 实现资源追踪

**影响:**
- 自动资源清理
- 优雅的操作取消
- 资源泄漏检测

**迁移要求:**
- 为临时资源添加 guards
- 实现取消逻辑
- 测试资源清理

---

## 迁移步骤

### 准备阶段

1. **备份当前系统**
   ```bash
   # 备份数据库
   cp -r ~/.log-analyzer/data ~/.log-analyzer/data.backup
   
   # 备份配置
   cp -r ~/.log-analyzer/config ~/.log-analyzer/config.backup
   ```

2. **检查依赖版本**
   ```bash
   # Rust 版本 >= 1.70
   rustc --version
   
   # Node 版本 >= 18
   node --version
   ```

3. **审查配置文件**
   - 检查 `Cargo.toml` 中的依赖版本
   - 检查 `package.json` 中的依赖版本
   - 准备环境变量配置

### Phase 1 迁移: 核心基础设施

1. **更新 Rust 依赖**
   ```bash
   cd log-analyzer/src-tauri
   cargo update
   cargo build --release
   ```

2. **配置 Sentry**
   ```bash
   # 设置环境变量
   export SENTRY_DSN="your-sentry-dsn"
   export SENTRY_ENVIRONMENT="production"
   export SENTRY_RELEASE="v0.0.58"
   ```

3. **配置日志**
   ```toml
   # config/logging.toml
   [logging]
   level = "info"
   format = "json"
   rotation = "daily"
   max_files = 30
   ```

4. **测试错误处理**
   ```bash
   cargo test --release
   ```

### Phase 2 迁移: 并发和性能

1. **更新并发代码**
   ```bash
   # 运行并发测试
   cargo test --release concurrency
   ```

2. **配置缓存**
   ```toml
   # config/cache.toml
   [cache]
   max_capacity = 1000
   ttl_seconds = 300
   tti_seconds = 60
   ```

3. **性能基准测试**
   ```bash
   cargo bench --bench production_validation_benchmarks
   ```

### Phase 3 迁移: 前端状态管理

1. **更新前端依赖**
   ```bash
   cd log-analyzer
   npm install
   npm run build
   ```

2. **测试前端**
   ```bash
   npm run lint
   npm test
   ```

3. **集成测试**
   ```bash
   npm run tauri build
   ```

### Phase 4 迁移: 验证和资源管理

1. **配置验证规则**
   ```toml
   # config/validation.toml
   [validation]
   max_path_length = 4096
   max_archive_size_mb = 100
   max_archive_files = 1000
   ```

2. **测试资源管理**
   ```bash
   cargo test --release resource_management
   ```

### 最终验证

1. **运行完整测试套件**
   ```bash
   # 后端测试
   cargo test --release --all
   
   # 前端测试
   npm test
   
   # 集成测试
   cargo test --release --test '*'
   ```

2. **性能验证**
   ```bash
   cargo bench
   ```

3. **部署到生产环境**
   ```bash
   npm run tauri build
   ```

---

## 性能调优建议

### 后端性能调优

#### 1. 缓存配置

```toml
[cache]
# 根据可用内存调整
max_capacity = 5000  # 生产环境建议值
ttl_seconds = 600    # 10分钟
tti_seconds = 120    # 2分钟

# 缓存预热
enable_warmup = true
warmup_queries = ["common_query_1", "common_query_2"]
```

#### 2. 并发配置

```toml
[concurrency]
# 工作线程数（建议为 CPU 核心数）
worker_threads = 8

# 队列大小
task_queue_size = 10000
cleanup_queue_size = 1000

# 超时设置
lock_timeout_ms = 100
operation_timeout_ms = 30000
```

#### 3. 日志配置

```toml
[logging]
# 生产环境使用 INFO 级别
level = "info"

# JSON 格式用于日志聚合
format = "json"

# 日志轮转
rotation = "daily"
max_files = 30
max_file_size_mb = 100
```

#### 4. Sentry 配置

```toml
[sentry]
# 采样率（生产环境建议 0.1-0.2）
traces_sample_rate = 0.1
profiles_sample_rate = 0.1

# 错误过滤
ignore_errors = ["ConnectionReset", "Timeout"]
```

### 前端性能调优

#### 1. React Query 配置

```typescript
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,  // 5分钟
      cacheTime: 10 * 60 * 1000,  // 10分钟
      refetchOnWindowFocus: false,
      retry: 2,
    },
  },
});
```

#### 2. Zustand 配置

```typescript
// 使用 immer 中间件
import { immer } from 'zustand/middleware/immer';

// 使用 devtools 中间件（仅开发环境）
import { devtools } from 'zustand/middleware';

export const useAppStore = create(
  devtools(
    immer((set) => ({
      // store implementation
    })),
    { name: 'app-store', enabled: process.env.NODE_ENV === 'development' }
  )
);
```

---

## 故障排除指南

### 常见问题

#### 1. 编译错误

**问题:** `error[E0599]: no method named 'inner' found`

**解决方案:**
```rust
// 错误的代码
transaction.inner().set_tag(key, value);

// 正确的代码
transaction.set_tag(key, value);
```

#### 2. 缓存未命中

**问题:** 缓存命中率低于预期

**诊断:**
```rust
// 检查缓存统计
let stats = cache.entry_count();
tracing::info!("Cache entries: {}", stats);
```

**解决方案:**
- 增加 `max_capacity`
- 调整 TTL/TTI 值
- 实现缓存预热

#### 3. 死锁

**问题:** 应用程序挂起

**诊断:**
```bash
# 启用死锁检测
export RUST_BACKTRACE=1
export PARKING_LOT_DEADLOCK_DETECTION=1
```

**解决方案:**
- 使用 `try_lock_for()` 添加超时
- 确保锁顺序一致
- 使用 `parking_lot` 的死锁检测

#### 4. 内存泄漏

**问题:** 内存使用持续增长

**诊断:**
```rust
// 检查资源追踪器
let report = resource_tracker.generate_report();
tracing::warn!("Active resources: {}", report.total);
```

**解决方案:**
- 检查资源清理逻辑
- 使用 `scopeguard` 确保清理
- 启用泄漏检测

#### 5. 前端状态不同步

**问题:** UI 状态与后端不一致

**诊断:**
```typescript
// 检查 React Query 缓存
queryClient.getQueryData(['workspace', id]);
```

**解决方案:**
- 使用 `invalidateQueries` 刷新缓存
- 检查乐观更新逻辑
- 验证事件监听器

### 性能问题

#### 1. 搜索缓慢

**诊断:**
```bash
# 启用性能跟踪
export RUST_LOG=debug
```

**解决方案:**
- 增加工作线程数
- 优化查询计划
- 使用索引

#### 2. 高内存使用

**诊断:**
```rust
// 监控缓存大小
let size = cache.weighted_size();
tracing::info!("Cache size: {} bytes", size);
```

**解决方案:**
- 减少缓存容量
- 缩短 TTL
- 实现更激进的驱逐策略

---

## 回滚程序

### Phase 1 回滚: 核心基础设施

如果遇到严重问题，可以回滚到之前的版本：

1. **停止应用程序**
   ```bash
   # 停止所有进程
   pkill -f log-analyzer
   ```

2. **恢复备份**
   ```bash
   # 恢复数据
   rm -rf ~/.log-analyzer/data
   cp -r ~/.log-analyzer/data.backup ~/.log-analyzer/data
   
   # 恢复配置
   rm -rf ~/.log-analyzer/config
   cp -r ~/.log-analyzer/config.backup ~/.log-analyzer/config
   ```

3. **回滚代码**
   ```bash
   git checkout <previous-version-tag>
   cargo build --release
   npm run tauri build
   ```

4. **重启应用程序**
   ```bash
   ./target/release/log-analyzer
   ```

### Phase 2-4 回滚

每个阶段都可以独立回滚：

```bash
# 回滚到特定阶段
git checkout phase-1-complete
cargo build --release
npm run tauri build
```

### 紧急回滚

如果需要立即回滚：

1. **使用预构建的二进制文件**
   ```bash
   # 使用之前的稳定版本
   cp backup/log-analyzer-stable ./log-analyzer
   chmod +x ./log-analyzer
   ./log-analyzer
   ```

2. **禁用新功能**
   ```toml
   # config/features.toml
   [features]
   use_eyre = false
   use_parking_lot = false
   use_moka = false
   ```

---

## 服务配置

### 开发环境配置

```toml
# config/development.toml
[environment]
name = "development"

[logging]
level = "debug"
format = "pretty"

[cache]
max_capacity = 100
ttl_seconds = 60

[concurrency]
worker_threads = 4

[sentry]
enabled = false
```

### 生产环境配置

```toml
# config/production.toml
[environment]
name = "production"

[logging]
level = "info"
format = "json"
rotation = "daily"
max_files = 30

[cache]
max_capacity = 5000
ttl_seconds = 600
tti_seconds = 120

[concurrency]
worker_threads = 8
task_queue_size = 10000

[sentry]
enabled = true
dsn = "${SENTRY_DSN}"
traces_sample_rate = 0.1
profiles_sample_rate = 0.1

[monitoring]
performance_monitoring = true
health_checks = true
health_check_interval_seconds = 60
```

### 服务生命周期管理

```rust
// 创建服务
let services = AppServices::builder()
    .with_production_config()
    .build()?;

// 启动所有服务
services.start_all()?;

// 健康检查
let health = services.overall_health();
if health.status != HealthStatus::Healthy {
    tracing::error!("Services unhealthy: {:?}", health);
}

// 停止所有服务
services.stop_all()?;
```

---

## 监控和告警

### Sentry 监控

1. **错误监控**
   - 自动捕获所有 panic 和错误
   - 错误分组和去重
   - 错误趋势分析

2. **性能监控**
   - 事务跟踪
   - 慢查询检测
   - 资源使用监控

3. **告警配置**
   ```yaml
   # sentry-alerts.yaml
   alerts:
     - name: "High Error Rate"
       condition: "error_rate > 10%"
       action: "email"
       
     - name: "Slow Performance"
       condition: "p95_duration > 5s"
       action: "slack"
   ```

### 日志监控

1. **日志聚合**
   ```bash
   # 使用 ELK Stack 或类似工具
   filebeat -c filebeat.yml
   ```

2. **日志查询**
   ```json
   {
     "query": {
       "bool": {
         "must": [
           { "match": { "level": "error" } },
           { "range": { "@timestamp": { "gte": "now-1h" } } }
         ]
       }
     }
   }
   ```

### 健康检查

```rust
// 实现健康检查端点
#[tauri::command]
async fn health_check(services: State<'_, AppServices>) -> Result<HealthReport> {
    let health = services.overall_health();
    Ok(HealthReport {
        status: health.status,
        services: health.healthy_services,
        total: health.total_services,
        timestamp: SystemTime::now(),
    })
}
```

### 指标收集

```rust
// 收集关键指标
#[derive(Debug, Serialize)]
struct Metrics {
    cache_hit_rate: f64,
    active_searches: usize,
    memory_usage_mb: usize,
    cpu_usage_percent: f32,
}

fn collect_metrics(services: &AppServices) -> Metrics {
    Metrics {
        cache_hit_rate: calculate_hit_rate(),
        active_searches: services.cancellation_manager().active_count(),
        memory_usage_mb: get_memory_usage(),
        cpu_usage_percent: get_cpu_usage(),
    }
}
```

---

## 总结

本指南涵盖了将日志分析器应用程序部署到生产环境的所有关键方面。遵循这些步骤和建议将确保平稳的迁移和可靠的生产运行。

### 关键要点

1. **分阶段迁移** - 按照 Phase 1-4 的顺序逐步迁移
2. **充分测试** - 在每个阶段进行全面测试
3. **监控和告警** - 设置适当的监控和告警
4. **准备回滚** - 始终准备好回滚计划
5. **性能调优** - 根据实际负载调整配置

### 支持资源

- **文档**: `docs/` 目录中的所有文档
- **示例配置**: `config/` 目录中的示例文件
- **测试**: `src-tauri/tests/` 和 `src/__tests__/` 中的测试
- **基准测试**: `src-tauri/benches/` 中的性能基准

### 联系方式

如有问题或需要支持，请：
- 查看故障排除指南
- 检查 GitHub Issues
- 联系开发团队
