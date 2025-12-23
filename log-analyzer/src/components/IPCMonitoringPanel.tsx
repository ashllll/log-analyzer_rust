/**
 * IPC 监控面板组件
 * 
 * 显示 IPC 连接的健康状态、性能指标和告警信息
 */

import React, { useEffect, useState } from 'react';
import { getIPCMetricsCollector, IPCAggregatedMetrics, Alert } from '../utils/ipcMetrics';
import { getIPCHealthChecker, IPCHealthStatus } from '../utils/ipcHealthCheck';
import { getCircuitBreakerState } from '../utils/ipcRetry';

export const IPCMonitoringPanel: React.FC = () => {
  const [metrics, setMetrics] = useState<IPCAggregatedMetrics | null>(null);
  const [healthStatus, setHealthStatus] = useState<IPCHealthStatus | null>(null);
  const [circuitBreakerState, setCircuitBreakerState] = useState<string>('CLOSED');
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [isExpanded, setIsExpanded] = useState(false);
  
  useEffect(() => {
    // 初始加载
    updateMetrics();
    
    // 定期更新（每 5 秒）
    const interval = setInterval(updateMetrics, 5000);
    
    return () => clearInterval(interval);
  }, []);
  
  const updateMetrics = () => {
    const metricsCollector = getIPCMetricsCollector();
    const healthChecker = getIPCHealthChecker();
    
    setMetrics(metricsCollector.getAggregatedMetrics());
    setHealthStatus(healthChecker.getHealthStatus());
    setCircuitBreakerState(getCircuitBreakerState());
    setAlerts(metricsCollector.getRecentAlerts(5));
  };
  
  const clearAlerts = () => {
    const metricsCollector = getIPCMetricsCollector();
    metricsCollector.clearAlerts();
    updateMetrics();
  };
  
  if (!metrics || !healthStatus) {
    return null;
  }
  
  // 确定健康状态颜色
  const getHealthColor = () => {
    if (!healthStatus.isHealthy) return 'text-red-600';
    if (circuitBreakerState === 'OPEN') return 'text-red-600';
    if (circuitBreakerState === 'HALF_OPEN') return 'text-yellow-600';
    if (metrics.successRate < 0.9) return 'text-yellow-600';
    return 'text-green-600';
  };
  
  const getHealthLabel = () => {
    if (!healthStatus.isHealthy) return 'Unhealthy';
    if (circuitBreakerState === 'OPEN') return 'Circuit Open';
    if (circuitBreakerState === 'HALF_OPEN') return 'Recovering';
    if (metrics.successRate < 0.9) return 'Degraded';
    return 'Healthy';
  };
  
  return (
    <div className="fixed bottom-4 right-4 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 max-w-md z-50">
      {/* Header */}
      <div 
        className="flex items-center justify-between p-3 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700 rounded-t-lg"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-2">
          <div className={`w-3 h-3 rounded-full ${getHealthColor().replace('text-', 'bg-')}`} />
          <span className="font-semibold text-sm">IPC Monitor</span>
          <span className={`text-xs ${getHealthColor()}`}>{getHealthLabel()}</span>
        </div>
        <button className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
          {isExpanded ? '▼' : '▲'}
        </button>
      </div>
      
      {/* Expanded Content */}
      {isExpanded && (
        <div className="p-4 border-t border-gray-200 dark:border-gray-700 space-y-4">
          {/* Key Metrics */}
          <div className="grid grid-cols-2 gap-3 text-sm">
            <div>
              <div className="text-gray-500 dark:text-gray-400 text-xs">Success Rate</div>
              <div className="font-semibold">{(metrics.successRate * 100).toFixed(1)}%</div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400 text-xs">Total Calls</div>
              <div className="font-semibold">{metrics.totalCalls}</div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400 text-xs">Avg Latency</div>
              <div className="font-semibold">{metrics.averageDuration.toFixed(0)}ms</div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400 text-xs">P95 Latency</div>
              <div className="font-semibold">{metrics.p95Duration.toFixed(0)}ms</div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400 text-xs">Total Retries</div>
              <div className="font-semibold">{metrics.totalRetries}</div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400 text-xs">Circuit State</div>
              <div className="font-semibold">{circuitBreakerState}</div>
            </div>
          </div>
          
          {/* Alerts */}
          {alerts.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="text-xs font-semibold text-gray-700 dark:text-gray-300">
                  Recent Alerts ({alerts.length})
                </div>
                <button
                  onClick={clearAlerts}
                  className="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
                >
                  Clear
                </button>
              </div>
              <div className="space-y-1 max-h-40 overflow-y-auto">
                {alerts.map((alert, index) => (
                  <div
                    key={index}
                    className={`text-xs p-2 rounded ${
                      alert.severity === 'critical'
                        ? 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300'
                        : alert.severity === 'warning'
                        ? 'bg-yellow-50 dark:bg-yellow-900/20 text-yellow-700 dark:text-yellow-300'
                        : 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300'
                    }`}
                  >
                    <div className="font-semibold">{alert.type}</div>
                    <div>{alert.message}</div>
                    <div className="text-xs opacity-75 mt-1">
                      {new Date(alert.timestamp).toLocaleTimeString()}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
          
          {/* Command Stats */}
          {metrics.commandStats.size > 0 && (
            <div className="space-y-2">
              <div className="text-xs font-semibold text-gray-700 dark:text-gray-300">
                Top Commands
              </div>
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {Array.from(metrics.commandStats.values())
                  .sort((a, b) => b.totalCalls - a.totalCalls)
                  .slice(0, 5)
                  .map((stat, index) => (
                    <div
                      key={index}
                      className="text-xs p-2 bg-gray-50 dark:bg-gray-700 rounded flex justify-between items-center"
                    >
                      <div className="flex-1 truncate">{stat.command}</div>
                      <div className="flex gap-2 text-xs">
                        <span className="text-gray-500 dark:text-gray-400">
                          {stat.totalCalls} calls
                        </span>
                        <span className={stat.successRate >= 0.9 ? 'text-green-600' : 'text-red-600'}>
                          {(stat.successRate * 100).toFixed(0)}%
                        </span>
                      </div>
                    </div>
                  ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
