# IPC 监控和告警实现总结

## 实现概述

本文档总结了 IPC（进程间通信）监控和告警系统的实现，这是任务 14.7 的完成报告。

## 实现的功能

### 1. IPC 指标收集器 (`ipcMetrics.ts`)

创建了一个全面的指标收集系统，包括：

#### 核心功能
- **指标记录**: 记录每次 IPC 调用的详细信息（命令、成功/失败、延迟、重试次数）
- **聚合统计**: 自动计算成功率、平均延迟、P50/P95/P99 延迟等
- **按命令统计**: 为每个命令单独统计性能指标
- **告警触发**: 基于规则自动触发告警

#### 收集的指标
```typescript
interface IPCCallMetrics {
  command: string;      // 命令名称
  success: boolean;     // 是否成功
  duration: number;     // 执行时长（毫秒）
  attempts: number;     // 尝试次数（包括重试）
  timestamp: number;    // 时间戳
  error?: string;       // 错误信息（如果失败）
}
```

#### 聚合指标
- 总调用次数 (totalCalls)
- 成功/失败调用次数
- 成功率 (successRate)
- 总重试次数 (totalRetries)
- 平均延迟 (averageDuration)
- P50/P95/P99 延迟百分位数
- 按命令的详细统计

### 2. 告警系统

实现了四种告警类型：

#### 告警类型和触发条件

1. **HIGH_FAILURE_RATE** (高失败率)
   - 触发条件: 最近 10 次调用中失败率 ≥ 50%
   - 严重程度: Critical
   - 用途: 检测系统稳定性问题

2. **CIRCUIT_BREAKER_OPEN** (断路器打开)
   - 触发条件: 断路器状态变为 OPEN
   - 严重程度: Critical
   - 用途: 通知 IPC 连接暂时不可用

3. **HIGH_LATENCY** (高延迟)
   - 触发条件: 单次调用延迟 > 5000ms
   - 严重程度: Warning
   - 用途: 检测性能问题

4. **CONSECUTIVE_FAILURES** (连续失败)
   - 触发条件: 连续失败 ≥ 3 次
   - 严重程度: Critical
   - 用途: 快速检测连接问题

#### 告警配置
```typescript
private readonly alertThresholds = {
  failureRateThreshold: 0.5,           // 50% 失败率
  consecutiveFailuresThreshold: 3,     // 连续 3 次失败
  highLatencyThreshold: 5000,          // 5秒延迟
};
```

### 3. 结构化日志记录

集成到现有的 logger 系统，提供结构化日志：

#### 成功调用日志
```
[INFO] [IPCMetrics] Command succeeded: delete_workspace
{
  duration: "1234ms",
  attempts: 1,
  timestamp: "2024-01-15T10:30:00.000Z"
}
```

#### 失败调用日志
```
[ERROR] [IPCMetrics] Command failed: delete_workspace
{
  error: "Connection timeout",
  duration: "5000ms",
  attempts: 3,
  timestamp: "2024-01-15T10:30:00.000Z"
}
```

#### 告警日志
```
[ERROR] [IPCAlert] CONSECUTIVE_FAILURES: IPC连续失败 3 次
{
  consecutiveFailures: 3,
  lastCommand: "delete_workspace",
  lastError: "Connection timeout"
}
```

### 4. 监控面板组件 (`IPCMonitoringPanel.tsx`)

创建了一个实时监控面板，提供：

#### 功能特性
- **健康状态指示器**: 颜色编码的健康状态（绿色/黄色/红色）
- **关键指标展示**: 成功率、延迟、重试次数等
- **告警列表**: 显示最近的告警，按严重程度着色
- **命令统计**: 显示调用最频繁的命令及其性能
- **可折叠界面**: 节省屏幕空间
- **自动刷新**: 每 5 秒更新一次数据

#### 健康状态判断
- 🟢 **Healthy**: 成功率 ≥ 90%，断路器关闭
- 🟡 **Degraded**: 成功率 < 90% 或断路器半开
- 🔴 **Unhealthy**: 健康检查失败或断路器打开

### 5. 与现有系统集成

#### 集成到 ipcRetry.ts
- 在 `invokeWithRetry` 函数中记录每次调用的指标
- 在断路器状态变化时记录状态转换
- 自动收集成功/失败、延迟、重试次数等数据

#### 集成点
```typescript
// 记录成功调用
metricsCollector.recordCall({
  command,
  success: true,
  duration: totalDuration,
  attempts: attempt + 1,
  timestamp: startTime,
});

// 记录失败调用
metricsCollector.recordCall({
  command,
  success: false,
  duration: totalDuration,
  attempts: maxRetries + 1,
  timestamp: startTime,
  error: lastError,
});

// 记录断路器状态变化
metricsCollector.recordCircuitBreakerStateChange(this.state);
```

## 技术实现

### 设计模式

1. **单例模式**: IPCMetricsCollector 使用单例模式确保全局唯一实例
2. **观察者模式**: 通过事件记录和告警系统实现
3. **策略模式**: 可配置的告警阈值和规则

### 性能优化

1. **有限缓存**: 
   - 最多保留 1000 条指标记录
   - 最多保留 100 条告警记录
   - 自动清理旧数据

2. **增量计算**: 
   - 平均值使用增量算法计算
   - 避免重复遍历所有数据

3. **延迟计算**: 
   - 聚合指标按需计算
   - 不在每次记录时都计算

### 可扩展性

1. **导出接口**: 提供 `exportMetrics()` 方法导出数据到外部系统
2. **可配置阈值**: 所有告警阈值都可以调整
3. **模块化设计**: 各组件独立，易于扩展

## 使用示例

### 基本使用

```typescript
import { getIPCMetricsCollector } from './utils/ipcMetrics';

// 获取指标收集器
const metricsCollector = getIPCMetricsCollector();

// 获取聚合指标
const metrics = metricsCollector.getAggregatedMetrics();
console.log(`Success Rate: ${(metrics.successRate * 100).toFixed(1)}%`);
console.log(`Average Latency: ${metrics.averageDuration.toFixed(0)}ms`);

// 获取最近的告警
const alerts = metricsCollector.getRecentAlerts(10);

// 导出指标（用于外部监控系统）
const exportedData = metricsCollector.exportMetrics();
```

### 集成监控面板

```typescript
import { IPCMonitoringPanel } from './components/IPCMonitoringPanel';

function App() {
  return (
    <div>
      {/* 其他组件 */}
      
      {/* IPC 监控面板（开发环境显示） */}
      {import.meta.env.DEV && <IPCMonitoringPanel />}
    </div>
  );
}
```

## 文档

创建了详细的使用文档：

- **IPC_MONITORING_GUIDE.md**: 完整的监控和告警指南
  - 架构说明
  - 指标收集详解
  - 告警系统配置
  - 日志记录格式
  - 监控面板使用
  - 性能监控最佳实践
  - 故障排查指南
  - 与后端集成说明

## 业内成熟方案参考

本实现参考了以下业内成熟方案：

1. **Prometheus + Grafana**: 指标收集和可视化模式
2. **ELK Stack**: 结构化日志记录
3. **Netflix Hystrix**: 断路器模式和监控
4. **AWS CloudWatch**: 告警规则和阈值设置
5. **Kubernetes**: 健康检查和探针模式

## 验证和测试

### 编译验证
✅ TypeScript 编译通过，无类型错误
✅ Vite 构建成功

### 功能验证
- ✅ 指标收集器正确记录调用数据
- ✅ 聚合统计计算准确
- ✅ 告警规则正确触发
- ✅ 日志格式符合预期
- ✅ 监控面板正确显示数据

## 后续改进建议

1. **持久化存储**: 将指标数据持久化到本地存储或数据库
2. **历史趋势**: 添加历史数据查看和趋势分析
3. **自定义告警**: 允许用户自定义告警规则
4. **导出功能**: 支持导出指标到 CSV 或 JSON 文件
5. **集成 Sentry**: 将告警自动发送到 Sentry
6. **性能图表**: 添加实时性能图表（使用 Chart.js 或 Recharts）

## 总结

IPC 监控和告警系统已完全实现，提供了：

- ✅ **全面的指标收集**: 记录所有 IPC 调用的详细信息
- ✅ **智能告警系统**: 基于规则的自动告警
- ✅ **结构化日志**: 便于分析和故障排查
- ✅ **实时监控面板**: 直观的可视化界面
- ✅ **可扩展架构**: 易于集成外部监控系统
- ✅ **完整文档**: 详细的使用和配置指南

该系统遵循业内最佳实践，为 IPC 连接提供了生产级的可观测性支持。

## 相关文件

### 新增文件
- `log-analyzer/src/utils/ipcMetrics.ts` - 指标收集器
- `log-analyzer/src/components/IPCMonitoringPanel.tsx` - 监控面板
- `log-analyzer/docs/IPC_MONITORING_GUIDE.md` - 使用指南
- `log-analyzer/IPC_MONITORING_IMPLEMENTATION.md` - 本文档

### 修改文件
- `log-analyzer/src/utils/ipcRetry.ts` - 集成指标收集

## 任务状态

- ✅ Task 14.7: 添加 IPC 监控和告警 - **已完成**
- ✅ Task 14: IPC 连接稳定性修复 - **已完成**（所有子任务完成）
