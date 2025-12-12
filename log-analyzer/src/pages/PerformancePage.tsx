import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw } from "lucide-react";

import type { PerformanceStats } from '../types/common';
import { Button, Card } from '../components/ui';

interface PerformancePageProps {
  addToast: (type: 'success' | 'error' | 'info', message: string) => void;
}

interface PerformanceMetricsResponse {
  memory_used_mb: number;
  path_map_size: number;
  cache_size: number;
  last_search_duration_ms: number;
  cache_hit_rate: number;
  indexed_files_count: number;
  index_file_size_mb: number;
}

/**
 * 性能监控页面
 * 显示应用性能指标，包括内存使用、索引大小、缓存命中率等
 */
const PerformancePage = ({ addToast }: PerformancePageProps) => {
  const [stats, setStats] = useState<PerformanceStats | null>(null);
  const [loading, setLoading] = useState(false);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const metrics = await invoke<PerformanceMetricsResponse>('get_performance_metrics');
      setStats({
        memoryUsed: metrics.memory_used_mb ?? 0,
        pathMapSize: metrics.path_map_size ?? 0,
        cacheSize: metrics.cache_size ?? 0,
        lastSearchDuration: metrics.last_search_duration_ms ?? 0,
        cacheHitRate: metrics.cache_hit_rate ?? 0,
        indexedFilesCount: metrics.indexed_files_count ?? 0,
        indexFileSizeMb: metrics.index_file_size_mb ?? 0
      });
    } catch (e) {
      addToast('error', `Failed to load stats: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 5000); // 每5秒刷新
    return () => clearInterval(interval);
  }, [loadStats]);

  if (!stats) {
    return <div className="p-10 text-center text-text-dim">Loading performance stats...</div>;
  }

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-text-main">Performance Monitor</h1>
        <Button icon={RefreshCw} onClick={loadStats} disabled={loading}>
          {loading ? 'Refreshing...' : 'Refresh'}
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {/* 内存使用 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Memory Usage</div>
          <div className="text-3xl font-bold text-primary">
            {stats.memoryUsed > 0 ? `${stats.memoryUsed.toFixed(1)} MB` : 'N/A'}
          </div>
          <div className="text-xs text-text-muted mt-1">进程内存占用</div>
        </Card>

        {/* 索引文件数 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Indexed Files</div>
          <div className="text-3xl font-bold text-emerald-400">
            {stats.indexedFilesCount.toLocaleString()}
          </div>
          <div className="text-xs text-text-muted mt-1">已索引文件数量</div>
        </Card>

        {/* 缓存大小 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Cache Size</div>
          <div className="text-3xl font-bold text-blue-400">
            {stats.cacheSize}
          </div>
          <div className="text-xs text-text-muted mt-1">搜索缓存条目数</div>
        </Card>

        {/* 搜索耗时 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Last Search</div>
          <div className="text-3xl font-bold text-amber-400">
            {stats.lastSearchDuration > 0 ? `${stats.lastSearchDuration} ms` : 'N/A'}
          </div>
          <div className="text-xs text-text-muted mt-1">最近搜索耗时</div>
        </Card>

        {/* 缓存命中率 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Cache Hit Rate</div>
          <div className="text-3xl font-bold text-purple-400">
            {stats.cacheHitRate.toFixed(1)}%
          </div>
          <div className="text-xs text-text-muted mt-1">缓存命中率</div>
        </Card>

        {/* 索引文件大小 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Index Size</div>
          <div className="text-3xl font-bold text-red-400">
            {stats.indexFileSizeMb.toFixed(2)} MB
          </div>
          <div className="text-xs text-text-muted mt-1">索引文件磁盘占用</div>
        </Card>
      </div>
    </div>
  );
};

export default PerformancePage;
