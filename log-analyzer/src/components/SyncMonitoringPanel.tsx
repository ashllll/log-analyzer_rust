/**
 * Sync Monitoring Panel Component
 * 
 * Displays synchronization metrics, latency, success rates, and error messages.
 * Provides manual refresh capabilities as fallback option.
 * 
 * Requirements: 4.2 - Synchronization monitoring and error handling
 */

import React, { useState } from 'react';
import { useSyncMonitoring } from '../hooks/useSyncMonitoring';
import { Button } from './ui/Button';
import { Card } from './ui/Card';

// ============================================================================
// Props
// ============================================================================

export interface SyncMonitoringPanelProps {
  className?: string;
  compact?: boolean;
  showLatencyGraph?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export const SyncMonitoringPanel: React.FC<SyncMonitoringPanelProps> = ({
  className = '',
  compact = false,
  showLatencyGraph = false,
}) => {
  const {
    stats,
    healthStatus,
    measureLatency,
    clearErrors,
    resetStats,
    manualRefresh,
  } = useSyncMonitoring();

  const [isMeasuring, setIsMeasuring] = useState(false);

  // ============================================================================
  // Handlers
  // ============================================================================

  const handleMeasureLatency = async () => {
    setIsMeasuring(true);
    try {
      await measureLatency();
    } catch {
      // Error is already recorded by the hook
    } finally {
      setIsMeasuring(false);
    }
  };

  // ============================================================================
  // Health Status Badge
  // ============================================================================

  const HealthBadge = () => {
    const colors = {
      healthy: 'bg-green-100 text-green-800 border-green-200',
      degraded: 'bg-yellow-100 text-yellow-800 border-yellow-200',
      unhealthy: 'bg-red-100 text-red-800 border-red-200',
    };

    const icons = {
      healthy: '✓',
      degraded: '⚠',
      unhealthy: '✕',
    };

    return (
      <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${colors[healthStatus]}`}>
        <span className="mr-1">{icons[healthStatus]}</span>
        {healthStatus.charAt(0).toUpperCase() + healthStatus.slice(1)}
      </span>
    );
  };

  // ============================================================================
  // Compact View
  // ============================================================================

  if (compact) {
    return (
      <div className={`flex items-center gap-4 ${className}`}>
        <HealthBadge />
        {stats.currentLatency !== null && (
          <span className="text-sm text-gray-600">
            {stats.currentLatency}ms
          </span>
        )}
        <span className="text-sm text-gray-600">
          {(stats.successRate * 100).toFixed(1)}%
        </span>
        <button
          onClick={manualRefresh}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          Refresh
        </button>
      </div>
    );
  }

  // ============================================================================
  // Full View
  // ============================================================================

  return (
    <Card className={`p-4 ${className}`}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-medium text-gray-900">Sync Status</h3>
        <HealthBadge />
      </div>

      {/* Metrics Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
        {/* Latency */}
        <MetricCard
          label="Current Latency"
          value={stats.currentLatency !== null ? `${stats.currentLatency}ms` : '-'}
          subValue={`Avg: ${stats.averageLatency.toFixed(0)}ms`}
          status={getLatencyStatus(stats.currentLatency)}
        />

        {/* Success Rate */}
        <MetricCard
          label="Success Rate"
          value={`${(stats.successRate * 100).toFixed(1)}%`}
          subValue={`${stats.successfulMessages}/${stats.totalMessages}`}
          status={getSuccessRateStatus(stats.successRate)}
        />

        {/* Uptime */}
        <MetricCard
          label="Connection Uptime"
          value={formatUptime(stats.connectionUptime)}
          subValue={`Reconnects: ${stats.reconnectCount}`}
          status={stats.connectionUptime > 0 ? 'good' : 'bad'}
        />

        {/* Errors */}
        <MetricCard
          label="Errors"
          value={stats.errors.length.toString()}
          subValue={stats.lastError ? 'Last: ' + formatTimeAgo(stats.lastError.timestamp) : 'None'}
          status={stats.errors.length === 0 ? 'good' : stats.errors.length < 5 ? 'warning' : 'bad'}
        />
      </div>

      {/* Latency Graph */}
      {showLatencyGraph && stats.latencyHistory.length > 0 && (
        <div className="mb-4">
          <h4 className="text-sm font-medium text-gray-700 mb-2">Latency History</h4>
          <LatencyGraph data={stats.latencyHistory} />
        </div>
      )}

      {/* Error List */}
      {stats.errors.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-sm font-medium text-gray-700">Recent Errors</h4>
            <button
              onClick={clearErrors}
              className="text-xs text-gray-500 hover:text-gray-700"
            >
              Clear
            </button>
          </div>
          <div className="max-h-32 overflow-y-auto space-y-1">
            {stats.errors.slice(-5).reverse().map((error, index) => (
              <div
                key={index}
                className="text-xs p-2 bg-red-50 text-red-700 rounded"
              >
                <span className="text-gray-500 mr-2">
                  {formatTimeAgo(error.timestamp)}
                </span>
                {error.message}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex items-center gap-2 pt-2 border-t border-gray-200">
        <Button
          onClick={handleMeasureLatency}
          disabled={isMeasuring}
          variant="secondary"
          className="text-sm h-8 px-3"
        >
          {isMeasuring ? 'Measuring...' : 'Measure Latency'}
        </Button>
        <Button
          onClick={manualRefresh}
          variant="secondary"
          className="text-sm h-8 px-3"
        >
          Manual Refresh
        </Button>
        <Button
          onClick={resetStats}
          variant="ghost"
          className="text-sm h-8 px-3"
        >
          Reset Stats
        </Button>
      </div>
    </Card>
  );
};

// ============================================================================
// Metric Card Component
// ============================================================================

interface MetricCardProps {
  label: string;
  value: string;
  subValue?: string;
  status: 'good' | 'warning' | 'bad';
}

const MetricCard: React.FC<MetricCardProps> = ({ label, value, subValue, status }) => {
  const statusColors = {
    good: 'text-green-600',
    warning: 'text-yellow-600',
    bad: 'text-red-600',
  };

  return (
    <div className="bg-gray-50 rounded-lg p-3">
      <div className="text-xs text-gray-500 mb-1">{label}</div>
      <div className={`text-lg font-semibold ${statusColors[status]}`}>{value}</div>
      {subValue && <div className="text-xs text-gray-400">{subValue}</div>}
    </div>
  );
};

// ============================================================================
// Latency Graph Component
// ============================================================================

interface LatencyGraphProps {
  data: Array<{ timestamp: Date; latency: number }>;
}

const LatencyGraph: React.FC<LatencyGraphProps> = ({ data }) => {
  const maxLatency = Math.max(...data.map((d) => d.latency), 100);
  const graphHeight = 60;

  return (
    <div className="bg-gray-50 rounded-lg p-2">
      <svg width="100%" height={graphHeight} className="overflow-visible">
        {/* Grid lines */}
        <line x1="0" y1={graphHeight / 2} x2="100%" y2={graphHeight / 2} stroke="#e5e7eb" strokeDasharray="4" />
        
        {/* Data points */}
        {data.slice(-50).map((point, index, arr) => {
          const x = (index / (arr.length - 1 || 1)) * 100;
          const y = graphHeight - (point.latency / maxLatency) * graphHeight;
          
          return (
            <g key={index}>
              <circle
                cx={`${x}%`}
                cy={y}
                r={2}
                fill={point.latency > 100 ? '#ef4444' : point.latency > 50 ? '#f59e0b' : '#22c55e'}
              />
              {index > 0 && (
                <line
                  x1={`${((index - 1) / (arr.length - 1 || 1)) * 100}%`}
                  y1={graphHeight - (arr[index - 1].latency / maxLatency) * graphHeight}
                  x2={`${x}%`}
                  y2={y}
                  stroke="#9ca3af"
                  strokeWidth={1}
                />
              )}
            </g>
          );
        })}
      </svg>
      <div className="flex justify-between text-xs text-gray-400 mt-1">
        <span>0ms</span>
        <span>{maxLatency}ms</span>
      </div>
    </div>
  );
};

// ============================================================================
// Helper Functions
// ============================================================================

function getLatencyStatus(latency: number | null): 'good' | 'warning' | 'bad' {
  if (latency === null) return 'warning';
  if (latency < 100) return 'good';
  if (latency < 500) return 'warning';
  return 'bad';
}

function getSuccessRateStatus(rate: number): 'good' | 'warning' | 'bad' {
  if (rate >= 0.95) return 'good';
  if (rate >= 0.8) return 'warning';
  return 'bad';
}

function formatUptime(ms: number): string {
  if (ms === 0) return 'Disconnected';
  
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  }
  return `${seconds}s`;
}

function formatTimeAgo(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHour = Math.floor(diffMin / 60);
  return `${diffHour}h ago`;
}

export default SyncMonitoringPanel;
