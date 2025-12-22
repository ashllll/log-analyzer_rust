import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import {
  Activity,
  AlertTriangle,
  TrendingUp,
  RefreshCw,
  Settings as SettingsIcon,
  Zap,
  Database,
  Cpu,
  HardDrive,
} from 'lucide-react';
import { Card } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { useToastManager } from '../hooks/useToastManager';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

// ========================================================================
// 类型定义
// ========================================================================

interface QueryTimingStats {
  query_count: number;
  avg_parsing_ms: number;
  avg_execution_ms: number;
  avg_formatting_ms: number;
  avg_highlighting_ms: number;
  avg_total_ms: number;
  p50_total_ms: number;
  p95_total_ms: number;
  p99_total_ms: number;
  min_total_ms: number;
  max_total_ms: number;
}

interface CacheMetricsSnapshot {
  l1_hit_count: number;
  l1_miss_count: number;
  l1_hit_rate: number;
  l1_size: number;
  l1_capacity: number;
  l1_eviction_count: number;
  l2_hit_count: number;
  l2_miss_count: number;
  l2_hit_rate: number;
  avg_access_time_ms: number;
  avg_load_time_ms: number;
}

interface SystemResourceMetrics {
  cpu_usage_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  memory_usage_percent: number;
  disk_read_bytes: number;
  disk_write_bytes: number;
  process_count: number;
  uptime_seconds: number;
  timestamp: string;
}

interface StateSyncStats {
  total_operations: number;
  success_count: number;
  failure_count: number;
  success_rate: number;
  avg_latency_ms: number;
}

interface PerformanceMetricsSummary {
  query_stats: QueryTimingStats;
  cache_metrics: CacheMetricsSnapshot;
  system_metrics: SystemResourceMetrics | null;
  state_sync_stats: StateSyncStats;
}

interface Alert {
  id: string;
  alert_type: string;
  severity: string;
  message: string;
  timestamp: string;
  metadata: Record<string, any>;
}

// ========================================================================
// API 调用函数
// ========================================================================

async function fetchPerformanceMetrics(): Promise<PerformanceMetricsSummary> {
  return await invoke('get_performance_metrics');
}

async function fetchPerformanceAlerts(limit?: number): Promise<Alert[]> {
  return await invoke('get_performance_alerts', { limit });
}

async function fetchPerformanceRecommendations(limit?: number): Promise<string[]> {
  return await invoke('get_performance_recommendations', { limit });
}

async function resetPerformanceMetrics(): Promise<void> {
  return await invoke('reset_performance_metrics');
}

// ========================================================================
// 主组件
// ========================================================================

export function PerformanceMonitoringPage() {
  const { t } = useTranslation();
  const { showToast } = useToastManager();
  const queryClient = useQueryClient();
  const [autoRefresh, setAutoRefresh] = useState(true);
  const refreshInterval = 5000; // 5秒

  // 查询性能指标
  const {
    data: metrics,
    isLoading: metricsLoading,
    error: metricsError,
    refetch: refetchMetrics,
  } = useQuery({
    queryKey: ['performance-metrics'],
    queryFn: fetchPerformanceMetrics,
    refetchInterval: autoRefresh ? refreshInterval : false,
  });

  // 查询告警
  const {
    data: alerts,
    isLoading: alertsLoading,
    refetch: refetchAlerts,
  } = useQuery({
    queryKey: ['performance-alerts'],
    queryFn: () => fetchPerformanceAlerts(20),
    refetchInterval: autoRefresh ? refreshInterval : false,
  });

  // 查询优化建议
  const {
    data: recommendations,
    isLoading: recommendationsLoading,
    refetch: refetchRecommendations,
  } = useQuery({
    queryKey: ['performance-recommendations'],
    queryFn: () => fetchPerformanceRecommendations(10),
    refetchInterval: autoRefresh ? refreshInterval * 2 : false, // 建议刷新频率低一些
  });

  // 重置指标
  const resetMutation = useMutation({
    mutationFn: resetPerformanceMetrics,
    onSuccess: () => {
      showToast('success', t('performance.reset_success'));
      queryClient.invalidateQueries({ queryKey: ['performance-metrics'] });
      queryClient.invalidateQueries({ queryKey: ['performance-alerts'] });
      queryClient.invalidateQueries({ queryKey: ['performance-recommendations'] });
    },
    onError: (error) => {
      showToast('error', t('performance.reset_error', { error: String(error) }));
    },
  });

  // 手动刷新所有数据
  const handleRefreshAll = () => {
    refetchMetrics();
    refetchAlerts();
    refetchRecommendations();
    showToast('info', t('performance.refreshed'));
  };

  // 处理重置
  const handleReset = () => {
    if (window.confirm(t('performance.reset_confirm'))) {
      resetMutation.mutate();
    }
  };

  if (metricsError) {
    return (
      <div className="flex items-center justify-center h-full">
        <Card className="p-6 max-w-md">
          <div className="flex items-center gap-3 text-red-600 mb-4">
            <AlertTriangle size={24} />
            <h3 className="text-lg font-semibold">{t('performance.error_title')}</h3>
          </div>
          <p className="text-text-secondary mb-4">
            {t('performance.error_message', { error: String(metricsError) })}
          </p>
          <Button onClick={handleRefreshAll} variant="primary">
            {t('performance.retry')}
          </Button>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* 页面头部 */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border-base bg-bg-sidebar">
        <div className="flex items-center gap-3">
          <div className="h-10 w-10 bg-primary/10 rounded-lg flex items-center justify-center">
            <Activity className="text-primary" size={20} />
          </div>
          <div>
            <h1 className="text-xl font-bold">{t('performance.title')}</h1>
            <p className="text-sm text-text-secondary">{t('performance.subtitle')}</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="rounded"
            />
            {t('performance.auto_refresh')}
          </label>

          <Button
            onClick={handleRefreshAll}
            variant="secondary"
            disabled={metricsLoading}
          >
            <RefreshCw size={16} className={metricsLoading ? 'animate-spin' : ''} />
            {t('performance.refresh')}
          </Button>

          <Button onClick={handleReset} variant="secondary">
            <SettingsIcon size={16} />
            {t('performance.reset')}
          </Button>
        </div>
      </div>

      {/* 主内容区域 */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* 概览卡片 */}
        <OverviewCards metrics={metrics} loading={metricsLoading} />

        {/* 搜索性能图表 */}
        <SearchPerformanceCharts metrics={metrics} loading={metricsLoading} />

        {/* 缓存性能 */}
        <CachePerformanceSection metrics={metrics} loading={metricsLoading} />

        {/* 系统资源 */}
        <SystemResourcesSection metrics={metrics} loading={metricsLoading} />

        {/* 告警列表 */}
        <AlertsSection alerts={alerts} loading={alertsLoading} />

        {/* 优化建议 */}
        <RecommendationsSection
          recommendations={recommendations}
          loading={recommendationsLoading}
        />
      </div>
    </div>
  );
}

// ========================================================================
// 子组件
// ========================================================================

interface OverviewCardsProps {
  metrics?: PerformanceMetricsSummary;
  loading: boolean;
}

function OverviewCards({ metrics, loading }: OverviewCardsProps) {
  const { t } = useTranslation();

  if (loading || !metrics) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i} className="p-4 animate-pulse">
            <div className="h-20 bg-bg-hover rounded" />
          </Card>
        ))}
      </div>
    );
  }

  const cards = [
    {
      icon: Zap,
      label: t('performance.avg_search_time'),
      value: `${metrics.query_stats.avg_total_ms.toFixed(1)}ms`,
      subValue: `P95: ${metrics.query_stats.p95_total_ms.toFixed(1)}ms`,
      color: 'text-blue-500',
      bgColor: 'bg-blue-500/10',
    },
    {
      icon: Database,
      label: t('performance.cache_hit_rate'),
      value: `${(metrics.cache_metrics.l1_hit_rate * 100).toFixed(1)}%`,
      subValue: `${metrics.cache_metrics.l1_hit_count} hits`,
      color: 'text-green-500',
      bgColor: 'bg-green-500/10',
    },
    {
      icon: Cpu,
      label: t('performance.cpu_usage'),
      value: metrics.system_metrics
        ? `${metrics.system_metrics.cpu_usage_percent.toFixed(1)}%`
        : 'N/A',
      subValue: metrics.system_metrics
        ? `${metrics.system_metrics.process_count} processes`
        : '',
      color: 'text-orange-500',
      bgColor: 'bg-orange-500/10',
    },
    {
      icon: HardDrive,
      label: t('performance.memory_usage'),
      value: metrics.system_metrics
        ? `${metrics.system_metrics.memory_usage_percent.toFixed(1)}%`
        : 'N/A',
      subValue: metrics.system_metrics
        ? `${(metrics.system_metrics.memory_used_bytes / 1024 / 1024 / 1024).toFixed(1)}GB`
        : '',
      color: 'text-purple-500',
      bgColor: 'bg-purple-500/10',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {cards.map((card, index) => (
        <Card key={index} className="p-4">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <p className="text-sm text-text-secondary mb-1">{card.label}</p>
              <p className="text-2xl font-bold mb-1">{card.value}</p>
              {card.subValue && (
                <p className="text-xs text-text-secondary">{card.subValue}</p>
              )}
            </div>
            <div className={`${card.bgColor} ${card.color} p-2 rounded-lg`}>
              <card.icon size={20} />
            </div>
          </div>
        </Card>
      ))}
    </div>
  );
}

interface SearchPerformanceChartsProps {
  metrics?: PerformanceMetricsSummary;
  loading: boolean;
}

function SearchPerformanceCharts({ metrics, loading }: SearchPerformanceChartsProps) {
  const { t } = useTranslation();

  if (loading || !metrics) {
    return (
      <Card className="p-6">
        <div className="h-64 bg-bg-hover rounded animate-pulse" />
      </Card>
    );
  }

  // 准备图表数据
  const phaseData = [
    { name: t('performance.parsing'), time: metrics.query_stats.avg_parsing_ms },
    { name: t('performance.execution'), time: metrics.query_stats.avg_execution_ms },
    { name: t('performance.formatting'), time: metrics.query_stats.avg_formatting_ms },
    { name: t('performance.highlighting'), time: metrics.query_stats.avg_highlighting_ms },
  ];

  const percentileData = [
    { name: 'Min', time: metrics.query_stats.min_total_ms },
    { name: 'P50', time: metrics.query_stats.p50_total_ms },
    { name: 'Avg', time: metrics.query_stats.avg_total_ms },
    { name: 'P95', time: metrics.query_stats.p95_total_ms },
    { name: 'P99', time: metrics.query_stats.p99_total_ms },
    { name: 'Max', time: metrics.query_stats.max_total_ms },
  ];

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">{t('performance.query_phases')}</h3>
        <ResponsiveContainer width="100%" height={250}>
          <BarChart data={phaseData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#333" />
            <XAxis dataKey="name" stroke="#888" />
            <YAxis stroke="#888" label={{ value: 'ms', angle: -90, position: 'insideLeft' }} />
            <Tooltip
              contentStyle={{ backgroundColor: '#1a1a1a', border: '1px solid #333' }}
            />
            <Bar dataKey="time" fill="#3b82f6" />
          </BarChart>
        </ResponsiveContainer>
      </Card>

      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">{t('performance.response_time_distribution')}</h3>
        <ResponsiveContainer width="100%" height={250}>
          <LineChart data={percentileData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#333" />
            <XAxis dataKey="name" stroke="#888" />
            <YAxis stroke="#888" label={{ value: 'ms', angle: -90, position: 'insideLeft' }} />
            <Tooltip
              contentStyle={{ backgroundColor: '#1a1a1a', border: '1px solid #333' }}
            />
            <Line type="monotone" dataKey="time" stroke="#10b981" strokeWidth={2} />
          </LineChart>
        </ResponsiveContainer>
      </Card>
    </div>
  );
}

interface CachePerformanceSectionProps {
  metrics?: PerformanceMetricsSummary;
  loading: boolean;
}

function CachePerformanceSection({ metrics, loading }: CachePerformanceSectionProps) {
  const { t } = useTranslation();

  if (loading || !metrics) {
    return (
      <Card className="p-6">
        <div className="h-48 bg-bg-hover rounded animate-pulse" />
      </Card>
    );
  }

  const cacheData = [
    { name: t('performance.hits'), value: metrics.cache_metrics.l1_hit_count },
    { name: t('performance.misses'), value: metrics.cache_metrics.l1_miss_count },
  ];

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">{t('performance.cache_performance')}</h3>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div>
          <ResponsiveContainer width="100%" height={200}>
            <BarChart data={cacheData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#333" />
              <XAxis dataKey="name" stroke="#888" />
              <YAxis stroke="#888" />
              <Tooltip
                contentStyle={{ backgroundColor: '#1a1a1a', border: '1px solid #333' }}
              />
              <Bar dataKey="value" fill="#10b981" />
            </BarChart>
          </ResponsiveContainer>
        </div>

        <div className="space-y-3">
          <div className="flex justify-between items-center">
            <span className="text-text-secondary">{t('performance.hit_rate')}</span>
            <span className="font-semibold">
              {(metrics.cache_metrics.l1_hit_rate * 100).toFixed(1)}%
            </span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-text-secondary">{t('performance.cache_size')}</span>
            <span className="font-semibold">
              {metrics.cache_metrics.l1_size} / {metrics.cache_metrics.l1_capacity}
            </span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-text-secondary">{t('performance.evictions')}</span>
            <span className="font-semibold">{metrics.cache_metrics.l1_eviction_count}</span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-text-secondary">{t('performance.avg_access_time')}</span>
            <span className="font-semibold">
              {metrics.cache_metrics.avg_access_time_ms.toFixed(2)}ms
            </span>
          </div>
        </div>
      </div>
    </Card>
  );
}

interface SystemResourcesSectionProps {
  metrics?: PerformanceMetricsSummary;
  loading: boolean;
}

function SystemResourcesSection({ metrics, loading }: SystemResourcesSectionProps) {
  const { t } = useTranslation();

  if (loading || !metrics || !metrics.system_metrics) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">{t('performance.system_resources')}</h3>
        <p className="text-text-secondary">{t('performance.no_system_metrics')}</p>
      </Card>
    );
  }

  const sys = metrics.system_metrics;

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">{t('performance.system_resources')}</h3>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="space-y-3">
          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sm text-text-secondary">{t('performance.cpu_usage')}</span>
              <span className="text-sm font-semibold">{sys.cpu_usage_percent.toFixed(1)}%</span>
            </div>
            <div className="w-full bg-bg-hover rounded-full h-2">
              <div
                className="bg-orange-500 h-2 rounded-full transition-all"
                style={{ width: `${Math.min(sys.cpu_usage_percent, 100)}%` }}
              />
            </div>
          </div>

          <div>
            <div className="flex justify-between mb-1">
              <span className="text-sm text-text-secondary">{t('performance.memory_usage')}</span>
              <span className="text-sm font-semibold">{sys.memory_usage_percent.toFixed(1)}%</span>
            </div>
            <div className="w-full bg-bg-hover rounded-full h-2">
              <div
                className="bg-purple-500 h-2 rounded-full transition-all"
                style={{ width: `${Math.min(sys.memory_usage_percent, 100)}%` }}
              />
            </div>
          </div>
        </div>

        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('performance.memory_used')}</span>
            <span className="font-mono">
              {(sys.memory_used_bytes / 1024 / 1024 / 1024).toFixed(2)} GB
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('performance.memory_total')}</span>
            <span className="font-mono">
              {(sys.memory_total_bytes / 1024 / 1024 / 1024).toFixed(2)} GB
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('performance.process_count')}</span>
            <span className="font-mono">{sys.process_count}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('performance.uptime')}</span>
            <span className="font-mono">
              {Math.floor(sys.uptime_seconds / 3600)}h {Math.floor((sys.uptime_seconds % 3600) / 60)}m
            </span>
          </div>
        </div>
      </div>
    </Card>
  );
}

interface AlertsSectionProps {
  alerts?: Alert[];
  loading: boolean;
}

function AlertsSection({ alerts, loading }: AlertsSectionProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <Card className="p-6">
        <div className="h-32 bg-bg-hover rounded animate-pulse" />
      </Card>
    );
  }

  if (!alerts || alerts.length === 0) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">{t('performance.alerts')}</h3>
        <div className="flex items-center gap-3 text-green-600">
          <TrendingUp size={20} />
          <p>{t('performance.no_alerts')}</p>
        </div>
      </Card>
    );
  }

  const getSeverityColor = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'critical':
        return 'text-red-600 bg-red-600/10';
      case 'warning':
        return 'text-yellow-600 bg-yellow-600/10';
      case 'info':
        return 'text-blue-600 bg-blue-600/10';
      default:
        return 'text-gray-600 bg-gray-600/10';
    }
  };

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">{t('performance.alerts')}</h3>
      <div className="space-y-2 max-h-64 overflow-y-auto">
        {alerts.map((alert) => (
          <div
            key={alert.id}
            className="flex items-start gap-3 p-3 rounded-lg bg-bg-hover hover:bg-bg-hover/80 transition-colors"
          >
            <AlertTriangle size={18} className="mt-0.5 flex-shrink-0" />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span
                  className={`text-xs px-2 py-0.5 rounded-full ${getSeverityColor(alert.severity)}`}
                >
                  {alert.severity}
                </span>
                <span className="text-xs text-text-secondary">
                  {new Date(alert.timestamp).toLocaleString()}
                </span>
              </div>
              <p className="text-sm">{alert.message}</p>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}

interface RecommendationsSectionProps {
  recommendations?: string[];
  loading: boolean;
}

function RecommendationsSection({ recommendations, loading }: RecommendationsSectionProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <Card className="p-6">
        <div className="h-32 bg-bg-hover rounded animate-pulse" />
      </Card>
    );
  }

  if (!recommendations || recommendations.length === 0) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">{t('performance.recommendations')}</h3>
        <p className="text-text-secondary">{t('performance.no_recommendations')}</p>
      </Card>
    );
  }

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">{t('performance.recommendations')}</h3>
      <div className="space-y-3">
        {recommendations.map((rec, index) => (
          <div
            key={index}
            className="flex items-start gap-3 p-3 rounded-lg bg-blue-500/5 border border-blue-500/20"
          >
            <TrendingUp size={18} className="text-blue-500 mt-0.5 flex-shrink-0" />
            <p className="text-sm flex-1">{rec}</p>
          </div>
        ))}
      </div>
    </Card>
  );
}
