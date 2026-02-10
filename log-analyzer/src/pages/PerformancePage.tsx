/**
 * 性能监控页面
 *
 * 显示系统性能指标，包括：
 * - 搜索性能（延迟、吞吐量）
 * - 缓存命中率
 * - 内存使用情况
 * - 任务执行统计
 *
 * 使用 React Query 进行数据管理，符合项目"必须使用业内成熟方案"原则
 */

import { useState, useCallback } from 'react';
import { Activity, Cpu, HardDrive, Database, Zap, TrendingUp, Clock, Pause, Play } from 'lucide-react';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { getFullErrorMessage } from '../services/errors';
import {
  useAutoRefreshPerformanceMetrics,
  DEFAULT_PERFORMANCE_METRICS,
} from '../hooks';

/**
 * 格式化数字（添加千分位分隔符）
 */
function formatNumber(num: number): string {
  return num.toLocaleString('en-US');
}

/**
 * 格式化字节数为可读形式
 */
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

/**
 * 格式化百分比
 */
function formatPercent(value: number): string {
  return `${value.toFixed(1)}%`;
}

/**
 * 指标行组件
 */
interface MetricRowProps {
  label: string;
  value: string | number;
  color?: string;
  unit?: string;
}

function MetricRow({ label, value, color = 'text-text-main', unit = '' }: MetricRowProps) {
  return (
    <div className="flex justify-between items-center py-2 border-b border-border-base/50 last:border-0">
      <span className="text-sm text-text-dim">{label}</span>
      <span className={`text-sm font-medium ${color}`}>
        {typeof value === 'number' ? formatNumber(value) : value}{unit && ` ${unit}`}
      </span>
    </div>
  );
}

/**
 * 性能监控页面组件
 */
export function PerformancePage() {
  const [autoRefresh, setAutoRefresh] = useState(true);
  const refreshInterval = 5000; // 5秒

  // 使用 React Query 获取性能指标（符合项目成熟方案原则）
  const { data: metrics = DEFAULT_PERFORMANCE_METRICS, isLoading, error, refetch } =
    useAutoRefreshPerformanceMetrics(autoRefresh, refreshInterval);

  /**
   * 切换自动刷新
   */
  const handleToggleRefresh = useCallback(() => {
    setAutoRefresh((prev) => !prev);
  }, []);

  /**
   * 手动刷新
   */
  const handleRefresh = useCallback(() => {
    refetch();
  }, [refetch]);

  return (
    <div className="h-full p-6 overflow-y-auto bg-bg-main">
      <div className="max-w-6xl mx-auto space-y-6">
        {/* 标题栏 */}
        <div className="flex justify-between items-center">
          <div>
            <h1 className="text-2xl font-bold text-text-main flex items-center gap-2">
              <Activity className="w-6 h-6 text-primary" />
              Performance Monitor
            </h1>
            <p className="text-sm text-text-dim mt-1">系统性能指标实时监控</p>
          </div>
          <div className="flex gap-2">
            <Button
              variant="ghost"
              icon={autoRefresh ? Pause : Play}
              onClick={handleToggleRefresh}
              disabled={isLoading}
            >
              {autoRefresh ? 'Pause' : 'Resume'}
            </Button>
            <Button
              variant="secondary"
              icon={Zap}
              onClick={handleRefresh}
              disabled={isLoading}
            >
              Refresh
            </Button>
          </div>
        </div>

        {/* 错误状态 */}
        {error && (
          <div className="p-4 bg-red-500/10 border border-red-500/50 rounded-lg">
            <p className="text-sm text-red-400">
              Failed to load performance metrics: {getFullErrorMessage(error)}
            </p>
          </div>
        )}

        {/* 性能指标卡片网格 */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* 搜索性能 */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-text-main flex items-center gap-2">
                <Database size={18} className="text-blue-500" />
                Search Performance
              </h3>
              {isLoading && <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>}
            </div>
            <MetricRow label="Current Latency" value={metrics.searchLatency.current} unit="ms" />
            <MetricRow label="Average Latency" value={metrics.searchLatency.average} unit="ms" />
            <MetricRow label="P95 Latency" value={metrics.searchLatency.p95} unit="ms" />
            <MetricRow label="P99 Latency" value={metrics.searchLatency.p99} unit="ms" />
          </Card>

          {/* 吞吐量 */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-text-main flex items-center gap-2">
                <TrendingUp size={18} className="text-green-500" />
                Throughput
              </h3>
              {isLoading && <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>}
            </div>
            <MetricRow label="Current" value={metrics.searchThroughput.current} unit="req/s" />
            <MetricRow label="Average" value={metrics.searchThroughput.average} unit="req/s" />
            <MetricRow label="Peak" value={metrics.searchThroughput.peak} unit="req/s" />
          </Card>

          {/* 缓存性能 */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-text-main flex items-center gap-2">
                <Zap size={18} className="text-yellow-500" />
                Cache Performance
              </h3>
              {isLoading && <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>}
            </div>
            <MetricRow
              label="Hit Rate"
              value={formatPercent(metrics.cacheMetrics.hitRate)}
              color={metrics.cacheMetrics.hitRate >= 80 ? 'text-green-500' : metrics.cacheMetrics.hitRate >= 60 ? 'text-yellow-500' : 'text-red-500'}
            />
            <MetricRow label="Hit Count" value={metrics.cacheMetrics.hitCount} />
            <MetricRow label="Miss Count" value={metrics.cacheMetrics.missCount} color="text-red-400" />
            <MetricRow label="Cache Size" value={formatBytes(metrics.cacheMetrics.size)} />
            <MetricRow label="Capacity" value={formatBytes(metrics.cacheMetrics.capacity)} />
          </Card>

          {/* 内存使用 */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-text-main flex items-center gap-2">
                <Cpu size={18} className="text-purple-500" />
                Memory Usage
              </h3>
              {isLoading && <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>}
            </div>
            <MetricRow
              label="Used"
              value={metrics.memoryMetrics.used}
              unit="MB"
              color={metrics.memoryMetrics.used / metrics.memoryMetrics.total > 0.8 ? 'text-red-500' : 'text-text-main'}
            />
            <MetricRow label="Total" value={metrics.memoryMetrics.total} unit="MB" />
            <MetricRow label="Heap Used" value={metrics.memoryMetrics.heapUsed} unit="MB" />
            <MetricRow label="External" value={metrics.memoryMetrics.external} unit="MB" />
          </Card>

          {/* 任务统计 */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-text-main flex items-center gap-2">
                <Clock size={18} className="text-orange-500" />
                Task Statistics
              </h3>
              {isLoading && <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>}
            </div>
            <MetricRow label="Total" value={metrics.taskMetrics.total} />
            <MetricRow label="Running" value={metrics.taskMetrics.running} color="text-blue-500" />
            <MetricRow label="Completed" value={metrics.taskMetrics.completed} color="text-green-500" />
            <MetricRow label="Failed" value={metrics.taskMetrics.failed} color="text-red-500" />
            <MetricRow label="Avg Duration" value={metrics.taskMetrics.averageDuration} unit="ms" />
          </Card>

          {/* 索引状态 */}
          <Card className="p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-text-main flex items-center gap-2">
                <HardDrive size={18} className="text-cyan-500" />
                Index Status
              </h3>
              {isLoading && <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>}
            </div>
            <MetricRow label="Total Files" value={metrics.indexMetrics.totalFiles} />
            <MetricRow label="Indexed Files" value={metrics.indexMetrics.indexedFiles} color="text-green-500" />
            <MetricRow label="Total Size" value={formatBytes(metrics.indexMetrics.totalSize)} />
            <MetricRow label="Index Size" value={formatBytes(metrics.indexMetrics.indexSize)} />
          </Card>
        </div>

        {/* 自动刷新状态 */}
        {autoRefresh && (
          <div className="text-center text-xs text-text-dim">
            Auto-refreshing every {refreshInterval / 1000}s
          </div>
        )}
      </div>
    </div>
  );
}

export default PerformancePage;
