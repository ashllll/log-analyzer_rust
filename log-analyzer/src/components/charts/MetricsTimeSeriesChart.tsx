/**
 * 性能指标时间序列图表组件
 *
 * 使用 Recharts 3.6.0 实现业内成熟的时间序列可视化方案
 * 显示性能指标随时间变化的趋势
 *
 * ## 功能特性
 *
 * - 支持多指标同时显示
 * - 自动缩放和自适应
 * - 交互式数据提示
 * - 响应式设计
 */

import { useMemo } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts';

// ============================================================================
// 类型定义
// ============================================================================

/**
 * 指标类型定义
 */
export type MetricType =
  | 'search_latency_current'
  | 'search_latency_average'
  | 'search_latency_p95'
  | 'search_latency_p99'
  | 'throughput_current'
  | 'throughput_average'
  | 'cache_hit_rate'
  | 'memory_used';

/**
 * 指标配置
 */
interface MetricConfig {
  key: MetricType;
  label: string;
  color: string;
  unit?: string;
  yAxisId?: 'left' | 'right';
}

/**
 * 指标配置映射
 */
// 主题色彩系统 - 与Tailwind配置保持一致
const THEME_COLORS = {
  // 主色调 (Blue)
  primary: {
    500: '#3B82F6',
    400: '#60A5FA',
    300: '#93C5FD',
    200: '#BFDBFE',
  },
  // CTA/成功色 (Green)
  success: {
    500: '#22C55E',
    400: '#4ADE80',
  },
  // 警告色 (Amber)
  warning: {
    500: '#F59E0B',
  },
  // 紫色 (Purple)
  purple: {
    500: '#8B5CF6',
  },
  // 状态色
  status: {
    error: '#EF4444',
  },
} as const;

export const METRIC_CONFIGS: Record<MetricType, MetricConfig> = {
  search_latency_current: {
    key: 'search_latency_current',
    label: 'Current Latency',
    color: THEME_COLORS.primary[500],
    unit: 'ms',
    yAxisId: 'left',
  },
  search_latency_average: {
    key: 'search_latency_average',
    label: 'Average Latency',
    color: THEME_COLORS.primary[400],
    unit: 'ms',
    yAxisId: 'left',
  },
  search_latency_p95: {
    key: 'search_latency_p95',
    label: 'P95 Latency',
    color: THEME_COLORS.primary[300],
    unit: 'ms',
    yAxisId: 'left',
  },
  search_latency_p99: {
    key: 'search_latency_p99',
    label: 'P99 Latency',
    color: THEME_COLORS.primary[200],
    unit: 'ms',
    yAxisId: 'left',
  },
  throughput_current: {
    key: 'throughput_current',
    label: 'Throughput',
    color: THEME_COLORS.success[500],
    unit: 'req/s',
    yAxisId: 'right',
  },
  throughput_average: {
    key: 'throughput_average',
    label: 'Avg Throughput',
    color: THEME_COLORS.success[400],
    unit: 'req/s',
    yAxisId: 'right',
  },
  cache_hit_rate: {
    key: 'cache_hit_rate',
    label: 'Cache Hit Rate',
    color: THEME_COLORS.warning[500],
    unit: '%',
    yAxisId: 'right',
  },
  memory_used: {
    key: 'memory_used',
    label: 'Memory Used',
    color: THEME_COLORS.purple[500],
    unit: 'MB',
    yAxisId: 'right',
  },
} as const;

/**
 * 数据点类型（从后端 MetricsSnapshot 映射）
 */
export interface MetricsDataPoint {
  timestamp: number;
  search_latency_current: number;
  search_latency_average: number;
  search_latency_p95: number;
  search_latency_p99: number;
  throughput_current: number;
  throughput_average: number;
  cache_hit_rate: number;
  cache_hit_count: number;
  cache_miss_count: number;
  cache_size: number;
  cache_capacity: number;
  memory_used: number;
  memory_total: number;
  task_total: number;
  task_running: number;
  task_completed: number;
  task_failed: number;
  index_total_files: number;
  index_indexed_files: number;
}

// ============================================================================
// 组件属性
// ============================================================================

interface MetricsTimeSeriesChartProps {
  /** 数据点数组 */
  data: MetricsDataPoint[];
  /** 要显示的指标类型 */
  metrics: MetricType[];
  /** 图表高度 */
  height?: number;
  /** 是否显示网格 */
  showGrid?: boolean;
  /** 是否显示图例 */
  showLegend?: boolean;
  /** 警告阈值（可选） */
  alertThreshold?: {
    metric: MetricType;
    value: number;
    label?: string;
  };
}

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 格式化时间戳为可读字符串
 */
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffHours / 24);

  if (diffDays > 0) {
    return `${diffDays}d ago`;
  } else if (diffHours > 0) {
    return `${diffHours}h ago`;
  } else {
    const diffMins = Math.floor(diffMs / (1000 * 60));
    return `${diffMins}m ago`;
  }
}

/**
 * 自定义 Tooltip 组件
 */
interface CustomTooltipProps {
  active?: boolean;
  payload?: Array<{ value: number; name: string; color: string }>;
  label?: string;
}

function CustomTooltip({ active, payload, label }: CustomTooltipProps) {
  if (!active || !payload || !payload.length) {
    return null;
  }

  return (
    <div className="bg-bg-popover border border-border-base rounded-lg shadow-lg p-3">
      <p className="text-sm text-text-dim mb-2">{label}</p>
      {payload.map((entry, index) => (
        <div key={index} className="flex items-center gap-2 text-sm">
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: entry.color }}
          />
          <span className="text-text-dim">{entry.name}:</span>
          <span className="text-text-main font-medium">
            {entry.value.toLocaleString()}
          </span>
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// 主组件
// ============================================================================

export function MetricsTimeSeriesChart({
  data,
  metrics,
  height = 300,
  showGrid = true,
  showLegend = true,
  alertThreshold,
}: MetricsTimeSeriesChartProps) {
  // 处理数据：格式化时间戳
  const chartData = useMemo(() => {
    return data.map((point) => ({
      ...point,
      formattedTime: formatTimestamp(point.timestamp),
    }));
  }, [data]);

  // 确定是否需要双 Y 轴
  const needsDualAxis = useMemo(() => {
    const yAxisIds = new Set(
      metrics.map((m) => METRIC_CONFIGS[m].yAxisId || 'left')
    );
    return yAxisIds.size > 1;
  }, [metrics]);

  if (data.length === 0) {
    return (
      <div
        className="flex items-center justify-center border border-border-base rounded-lg bg-bg-subtle"
        style={{ height }}
      >
        <p className="text-text-dim text-sm">No data available</p>
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart
        data={chartData}
        margin={{ top: 5, right: needsDualAxis ? 30 : 5, left: needsDualAxis ? 30 : 5, bottom: 5 }}
      >
        {showGrid && (
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="currentColor"
            strokeOpacity={0.1}
          />
        )}

        <XAxis
          dataKey="formattedTime"
          stroke="currentColor"
          strokeOpacity={0.3}
          tick={{ fill: 'currentColor', fillOpacity: 0.5, fontSize: 12 }}
          tickLine={false}
        />

        <YAxis
          yAxisId="left"
          stroke="currentColor"
          strokeOpacity={0.3}
          tick={{ fill: 'currentColor', fillOpacity: 0.5, fontSize: 12 }}
          tickLine={false}
          orientation={needsDualAxis ? 'left' : 'left'}
        />

        {needsDualAxis && (
          <YAxis
            yAxisId="right"
            stroke="currentColor"
            strokeOpacity={0.3}
            tick={{ fill: 'currentColor', fillOpacity: 0.5, fontSize: 12 }}
            tickLine={false}
            orientation="right"
          />
        )}

        <Tooltip content={<CustomTooltip />} />

        {showLegend && (
          <Legend
            wrapperStyle={{ fontSize: 12 }}
            iconType="circle"
            verticalAlign="top"
            height={36}
          />
        )}

        {/* 渲染选定的指标线 */}
        {metrics.map((metricType) => {
          const config = METRIC_CONFIGS[metricType];
          return (
            <Line
              key={config.key}
              yAxisId={config.yAxisId || 'left'}
              type="monotone"
              dataKey={config.key}
              name={config.label}
              stroke={config.color}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4 }}
              isAnimationActive={true}
              animationDuration={300}
              connectNulls={false}
            />
          );
        })}

        {/* 警告阈值线 */}
        {alertThreshold && metrics.includes(alertThreshold.metric) && (
          <ReferenceLine
            y={alertThreshold.value}
            yAxisId={METRIC_CONFIGS[alertThreshold.metric].yAxisId || 'left'}
            stroke={THEME_COLORS.status.error}
            strokeDasharray="5 5"
            label={{
              value: alertThreshold.label || 'Alert',
              position: 'right',
              fill: THEME_COLORS.status.error,
              fontSize: 11,
            }}
          />
        )}
      </LineChart>
    </ResponsiveContainer>
  );
}

export default MetricsTimeSeriesChart;
