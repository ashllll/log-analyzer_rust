import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:fl_chart/fl_chart.dart';

import '../../../core/theme/app_theme.dart';
import '../../../core/constants/app_constants.dart';

/// 性能监控页面
///
/// 对应 React 版本的 PerformancePage.tsx
/// 功能：
/// - 搜索延迟监控
/// - 搜索吞吐量统计
/// - 缓存命中率
/// - 内存使用情况
/// - 任务指标图表
/// - 索引指标
class PerformancePage extends ConsumerStatefulWidget {
  const PerformancePage({super.key});

  @override
  ConsumerState<PerformancePage> createState() => _PerformancePageState();
}

class _PerformancePageState extends ConsumerState<PerformancePage> {
  // 自动刷新定时器
  Timer? _autoRefreshTimer;

  // 性能指标数据
  PerformanceMetrics? _metrics;
  bool _isLoading = false;

  // 图表时间范围
  ChartTimeRange _timeRange = ChartTimeRange.minutes5;

  @override
  void initState() {
    super.initState();
    _startAutoRefresh();
    _loadMetrics();
  }

  @override
  void dispose() {
    _autoRefreshTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: _buildAppBar(context),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : _metrics == null
              ? _buildEmptyState()
              : _buildMetricsContent(),
    );
  }

  /// 构建 AppBar
  PreferredSizeWidget _buildAppBar(BuildContext context) {
    return AppBar(
      backgroundColor: AppColors.bgMain,
      elevation: 0,
      title: const Text(
        '性能',
        style: TextStyle(
          fontSize: 18,
          fontWeight: FontWeight.w600,
        ),
      ),
      actions: [
        // 时间范围选择器
        PopupMenuButton<ChartTimeRange>(
          icon: const Icon(Icons.schedule),
          tooltip: '时间范围',
          onSelected: (range) {
            setState(() {
              _timeRange = range;
            });
            _loadMetrics();
          },
          itemBuilder: (context) => [
            const PopupMenuItem(
              value: ChartTimeRange.minutes1,
              child: Text('1 分钟'),
            ),
            const PopupMenuItem(
              value: ChartTimeRange.minutes5,
              child: Text('5 分钟'),
            ),
            const PopupMenuItem(
              value: ChartTimeRange.minutes15,
              child: Text('15 分钟'),
            ),
            const PopupMenuItem(
              value: ChartTimeRange.hour1,
              child: Text('1 小时'),
            ),
          ],
        ),
        // 刷新按钮
        IconButton(
          icon: const Icon(Icons.refresh),
          tooltip: '刷新',
          onPressed: _loadMetrics,
        ),
      ],
    );
  }

  /// 构建空状态
  Widget _buildEmptyState() {
    return const Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.speed_outlined,
            size: 64,
            color: AppColors.textMuted,
          ),
          SizedBox(height: 16),
          Text(
            '暂无性能数据',
            style: TextStyle(
              fontSize: 16,
              color: AppColors.textSecondary,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建性能指标内容
  Widget _buildMetricsContent() {
    if (_metrics == null) return const SizedBox.shrink();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 关键指标卡片
        _buildKeyMetricsRow(),
        const SizedBox(height: 16),

        // 搜索延迟图表
        _buildLatencyChart(),
        const SizedBox(height: 16),

        // 缓存指标图表
        _buildCacheMetricsChart(),
        const SizedBox(height: 16),

        // 任务分布图表
        _buildTaskDistributionChart(),
        const SizedBox(height: 16),

        // 索引指标卡片
        _buildIndexMetricsCard(),
      ],
    );
  }

  /// 构建关键指标行
  Widget _buildKeyMetricsRow() {
    return Row(
      children: [
        Expanded(
          child: _MetricCard(
            title: '搜索延迟',
            value: _metrics!.searchLatency.toString(),
            unit: 'ms',
            icon: Icons.flash_on,
            color: _getLatencyColor(_metrics!.searchLatency),
            trend: _metrics!.latencyTrend,
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: _MetricCard(
            title: '搜索吞吐量',
            value: _metrics!.searchThroughput.toString(),
            unit: '次/秒',
            icon: Icons.speed,
            color: AppColors.primary,
            trend: _metrics!.throughputTrend,
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: _MetricCard(
            title: '缓存命中率',
            value: '${_metrics!.cacheHitRate.toStringAsFixed(1)}',
            unit: '%',
            icon: Icons.cached,
            color: _getCacheRateColor(_metrics!.cacheHitRate),
            trend: _metrics!.cacheTrend,
          ),
        ),
      ],
    );
  }

  /// 构建搜索延迟图表
  Widget _buildLatencyChart() {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(
                  Icons.show_chart,
                  color: AppColors.primary,
                  size: 20,
                ),
                const SizedBox(width: 12),
                const Text(
                  '搜索延迟趋势',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: AppColors.textPrimary,
                  ),
                ),
                const Spacer(),
                Text(
                  '平均 ${_metrics!.avgLatency.toStringAsFixed(2)} ms',
                  style: const TextStyle(
                    fontSize: 13,
                    color: AppColors.textSecondary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            SizedBox(
              height: 200,
              child: LineChart(
                _buildLatencyChartData(),
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// 构建延迟图表数据
  LineChartData _buildLatencyChartData() {
    final spots = _metrics!.latencyHistory
        .asMap()
        .entries
        .map((entry) => FlSpot(
              entry.key.toDouble(),
              entry.value.toDouble(),
            ))
        .toList();

    return LineChartData(
      gridData: FlGridData(
        show: true,
        drawVerticalLine: false,
        horizontalInterval: spots.isEmpty ? 1 : _calculateInterval(spots.map((e) => e.y).toList()),
        getDrawingHorizontalLine: (value) => FlLine(
          color: AppColors.border.withValues(alpha: 0.3),
          strokeWidth: 1,
        ),
      ),
      titlesData: FlTitlesData(
        show: true,
        rightTitles: const AxisTitles(
          sideTitles: SideTitles(showTitles: false),
        ),
        topTitles: const AxisTitles(
          sideTitles: SideTitles(showTitles: false),
        ),
        bottomTitles: AxisTitles(
          sideTitles: SideTitles(
            showTitles: true,
            reservedSize: 30,
            getTitlesWidget: (value, meta) {
              if (value.toInt() % 5 != 0) return const Text('');
              final index = value.toInt();
              if (index < 0 || index >= spots.length) return const Text('');
              return Text(
                _formatTime(spots.length - index),
                style: const TextStyle(
                  color: AppColors.textMuted,
                  fontSize: 10,
                ),
              );
            },
          ),
        ),
        leftTitles: AxisTitles(
          sideTitles: SideTitles(
            showTitles: true,
            reservedSize: 50,
            getTitlesWidget: (value, meta) {
              return Text(
                value.toStringAsFixed(0),
                style: const TextStyle(
                  color: AppColors.textMuted,
                  fontSize: 10,
                ),
              );
            },
          ),
        ),
      ),
      borderData: FlBorderData(
        show: true,
        border: Border.all(
          color: AppColors.border.withValues(alpha: 0.3),
        ),
      ),
      minX: 0,
      maxX: (spots.length - 1).toDouble(),
      minY: spots.isEmpty ? 0 : _calculateMinY(spots.map((e) => e.y).toList()),
      maxY: spots.isEmpty ? 100 : _calculateMaxY(spots.map((e) => e.y).toList()),
      lineBarsData: [
        LineChartBarData(
          spots: spots,
          isCurved: true,
          color: AppColors.primary,
          barWidth: 2,
          dotData: FlDotData(
            show: true,
            getDotPainter: (spot, percent, barData, index) =>
                FlDotCirclePainter(
              radius: 3,
              color: AppColors.primary,
              strokeWidth: 0,
            ),
          ),
          belowBarData: BarAreaData(
            show: true,
            color: AppColors.primary.withValues(alpha: 0.1),
          ),
        ),
      ],
      lineTouchData: LineTouchData(
        enabled: true,
        touchTooltipData: LineTouchTooltipData(
          getTooltipColor: (touchedSpot) => AppColors.bgCard,
          getTooltipItems: (touchedSpots) {
            return touchedSpots.map((spot) {
              return LineTooltipItem(
                '${spot.y.toStringAsFixed(2)} ms',
                const TextStyle(
                  color: AppColors.textPrimary,
                  fontSize: 12,
                ),
              );
            }).toList();
          },
        ),
      ),
    );
  }

  /// 计算图表 Y 轴最小值
  double _calculateMinY(List<double> values) {
    if (values.isEmpty) return 0;
    final min = values.reduce((a, b) => a < b ? a : b);
    return (min * 0.9).floorToDouble();
  }

  /// 计算图表 Y 轴最大值
  double _calculateMaxY(List<double> values) {
    if (values.isEmpty) return 100;
    final max = values.reduce((a, b) => a > b ? a : b);
    return (max * 1.1).ceilToDouble();
  }

  /// 计算图表间隔
  double _calculateInterval(List<double> values) {
    if (values.isEmpty) return 20;
    final range = _calculateMaxY(values) - _calculateMinY(values);
    return (range / 5).ceilToDouble();
  }

  /// 格式化时间显示
  String _formatTime(int index) {
    final now = DateTime.now();
    final time = now.subtract(Duration(seconds: index * _timeRange.intervalSeconds));
    final minutes = time.minute.toString().padLeft(2, '0');
    final seconds = time.second.toString().padLeft(2, '0');
    return '$minutes:$seconds';
  }

  /// 构建缓存指标图表
  Widget _buildCacheMetricsChart() {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(
                  Icons.storage,
                  color: AppColors.success,
                  size: 20,
                ),
                SizedBox(width: 12),
                Text(
                  '缓存指标',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: AppColors.textPrimary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: _CacheMetricItem(
                    label: '命中率',
                    value: '${_metrics!.cacheHitRate.toStringAsFixed(1)}%',
                    color: _getCacheRateColor(_metrics!.cacheHitRate),
                  ),
                ),
                Expanded(
                  child: _CacheMetricItem(
                    label: '总查询',
                    value: _metrics!.totalQueries.toString(),
                    color: AppColors.textSecondary,
                  ),
                ),
                Expanded(
                  child: _CacheMetricItem(
                    label: '缓存命中',
                    value: _metrics!.cacheHits.toString(),
                    color: AppColors.success,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            // 缓存大小进度条
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text(
                      '缓存使用',
                      style: TextStyle(
                        fontSize: 13,
                        color: AppColors.textSecondary,
                      ),
                    ),
                    Text(
                      '${_metrics!.cacheSize} / ${AppConstants.maxCacheSize}',
                      style: const TextStyle(
                        fontSize: 13,
                        color: AppColors.textSecondary,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                LinearProgressIndicator(
                  value: _metrics!.cacheSize / AppConstants.maxCacheSize,
                  backgroundColor: AppColors.bgInput,
                  valueColor: const AlwaysStoppedAnimation<Color>(
                    AppColors.primary,
                  ),
                  minHeight: 8,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// 构建任务分布图表
  Widget _buildTaskDistributionChart() {
    final taskData = _metrics!.taskDistribution;

    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(
                  Icons.pie_chart,
                  color: AppColors.warning,
                  size: 20,
                ),
                SizedBox(width: 12),
                Text(
                  '任务分布',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: AppColors.textPrimary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            SizedBox(
              height: 200,
              child: PieChart(
                PieChartData(
                  sectionsSpace: 2,
                  centerSpaceRadius: 40,
                  sections: [
                    PieChartSectionData(
                      value: taskData.completed.toDouble(),
                      title: '${taskData.completed}',
                      color: AppColors.success,
                      radius: 50,
                      titleStyle: const TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    PieChartSectionData(
                      value: taskData.failed.toDouble(),
                      title: '${taskData.failed}',
                      color: AppColors.error,
                      radius: 50,
                      titleStyle: const TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    PieChartSectionData(
                      value: taskData.running.toDouble(),
                      title: '${taskData.running}',
                      color: AppColors.primary,
                      radius: 50,
                      titleStyle: const TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                  pieTouchData: PieTouchData(
                    enabled: true,
                    touchCallback: (FlTouchEvent event, PieTouchResponse? response) {
                      // 可以在这里处理点击事件
                    },
                  ),
                ),
              ),
            ),
            const SizedBox(height: 16),
            // 图例
            Wrap(
              spacing: 16,
              runSpacing: 8,
              children: [
                _buildLegendItem('已完成', AppColors.success, taskData.completed),
                _buildLegendItem('失败', AppColors.error, taskData.failed),
                _buildLegendItem('运行中', AppColors.primary, taskData.running),
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// 构建图例项
  Widget _buildLegendItem(String label, Color color, int count) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 12,
          height: 12,
          decoration: BoxDecoration(
            color: color,
            borderRadius: BorderRadius.circular(3),
          ),
        ),
        const SizedBox(width: 8),
        Text(
          '$label: $count',
          style: const TextStyle(
            fontSize: 13,
            color: AppColors.textSecondary,
          ),
        ),
      ],
    );
  }

  /// 构建索引指标卡片
  Widget _buildIndexMetricsCard() {
    final indexMetrics = _metrics!.indexMetrics;

    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(
                  Icons.dataset,
                  color: AppColors.warning,
                  size: 20,
                ),
                SizedBox(width: 12),
                Text(
                  '索引指标',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: AppColors.textPrimary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: _IndexMetricItem(
                    label: '总文档数',
                    value: indexMetrics.totalDocs.toString(),
                    icon: Icons.description,
                  ),
                ),
                Expanded(
                  child: _IndexMetricItem(
                    label: '索引大小',
                    value: indexMetrics.indexSize,
                    icon: Icons.storage,
                  ),
                ),
                Expanded(
                  child: _IndexMetricItem(
                    label: '段数量',
                    value: indexMetrics.segmentCount.toString(),
                    icon: Icons.view_module,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: _IndexMetricItem(
                    label: '平均查询时间',
                    value: '${indexMetrics.avgQueryTime}ms',
                    icon: Icons.query_stats,
                  ),
                ),
                Expanded(
                  child: _IndexMetricItem(
                    label: '更新时间',
                    value: indexMetrics.lastUpdateTime,
                    icon: Icons.update,
                  ),
                ),
                Expanded(
                  child: _IndexMetricItem(
                    label: '合并次数',
                    value: indexMetrics.mergeCount.toString(),
                    icon: Icons.merge_type,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// 获取延迟颜色
  Color _getLatencyColor(double latency) {
    if (latency < 50) return AppColors.success;
    if (latency < 200) return AppColors.warning;
    return AppColors.error;
  }

  /// 获取缓存命中率颜色
  Color _getCacheRateColor(double rate) {
    if (rate >= 80) return AppColors.success;
    if (rate >= 50) return AppColors.warning;
    return AppColors.error;
  }

  /// 启动自动刷新
  void _startAutoRefresh() {
    _autoRefreshTimer?.cancel();
    _autoRefreshTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => _loadMetrics(),
    );
  }

  /// 加载性能指标
  Future<void> _loadMetrics() async {
    setState(() {
      _isLoading = true;
    });

    // TODO: 等待 API 模型与页面模型适配后启用
    // 目前使用模拟数据
    await Future.delayed(const Duration(milliseconds: 500));

    if (mounted) {
      setState(() {
        // 暂时设置为 null，等待模型适配
        _metrics = null;
        _isLoading = false;
      });
    }
  }
}

/// 性能指标数据模型
class PerformanceMetrics {
  final double searchLatency;
  final double searchThroughput;
  final double cacheHitRate;
  final int cacheSize;
  final int totalQueries;
  final int cacheHits;
  final List<double> latencyHistory;
  final double avgLatency;
  final TrendType latencyTrend;
  final TrendType throughputTrend;
  final TrendType cacheTrend;
  final TaskDistribution taskDistribution;
  final IndexMetrics indexMetrics;

  const PerformanceMetrics({
    required this.searchLatency,
    required this.searchThroughput,
    required this.cacheHitRate,
    required this.cacheSize,
    required this.totalQueries,
    required this.cacheHits,
    required this.latencyHistory,
    required this.avgLatency,
    required this.latencyTrend,
    required this.throughputTrend,
    required this.cacheTrend,
    required this.taskDistribution,
    required this.indexMetrics,
  });
}

/// 任务分布数据
class TaskDistribution {
  final int completed;
  final int failed;
  final int running;

  const TaskDistribution({
    required this.completed,
    required this.failed,
    required this.running,
  });
}

/// 索引指标数据
class IndexMetrics {
  final int totalDocs;
  final String indexSize;
  final int segmentCount;
  final String avgQueryTime;
  final String lastUpdateTime;
  final int mergeCount;

  const IndexMetrics({
    required this.totalDocs,
    required this.indexSize,
    required this.segmentCount,
    required this.avgQueryTime,
    required this.lastUpdateTime,
    required this.mergeCount,
  });
}

/// 趋势类型
enum TrendType {
  up,
  down,
  stable;

  String get value {
    switch (this) {
      case TrendType.up:
        return 'up';
      case TrendType.down:
        return 'down';
      case TrendType.stable:
        return 'stable';
    }
  }
}

/// 图表时间范围
enum ChartTimeRange {
  minutes1,
  minutes5,
  minutes15,
  hour1;

  int get intervalSeconds {
    switch (this) {
      case ChartTimeRange.minutes1:
        return 60 ~/ 60; // 1 秒间隔
      case ChartTimeRange.minutes5:
        return 5 * 60 ~/ 30; // 10 秒间隔
      case ChartTimeRange.minutes15:
        return 15 * 60 ~/ 30; // 30 秒间隔
      case ChartTimeRange.hour1:
        return 60 * 60 ~/ 60; // 1 分钟间隔
    }
  }

  int get dataPoints {
    switch (this) {
      case ChartTimeRange.minutes1:
        return 60;
      case ChartTimeRange.minutes5:
        return 30;
      case ChartTimeRange.minutes15:
        return 30;
      case ChartTimeRange.hour1:
        return 60;
    }
  }
}

/// 性能指标卡片组件
class _MetricCard extends StatelessWidget {
  final String title;
  final String value;
  final String unit;
  final IconData icon;
  final Color color;
  final TrendType? trend;

  const _MetricCard({
    required this.title,
    required this.value,
    required this.unit,
    required this.icon,
    required this.color,
    this.trend,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  icon,
                  color: color,
                  size: 18,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    title,
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textSecondary,
                    ),
                  ),
                ),
                if (trend != null) ...[
                  Icon(
                    trend == TrendType.up
                        ? Icons.arrow_upward
                        : trend == TrendType.down
                            ? Icons.arrow_downward
                            : Icons.remove,
                    size: 14,
                    color: trend == TrendType.up
                        ? AppColors.error
                        : trend == TrendType.down
                            ? AppColors.success
                            : AppColors.textMuted,
                  ),
                ],
              ],
            ),
            const SizedBox(height: 8),
            Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                Text(
                  value,
                  style: TextStyle(
                    fontSize: 24,
                    fontWeight: FontWeight.w700,
                    color: color,
                  ),
                ),
                const SizedBox(width: 4),
                Padding(
                  padding: const EdgeInsets.only(bottom: 4),
                  child: Text(
                    unit,
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textMuted,
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// 缓存指标项组件
class _CacheMetricItem extends StatelessWidget {
  final String label;
  final String value;
  final Color color;

  const _CacheMetricItem({
    required this.label,
    required this.value,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Text(
          label,
          style: const TextStyle(
            fontSize: 12,
            color: AppColors.textSecondary,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          value,
          style: TextStyle(
            fontSize: 20,
            fontWeight: FontWeight.w600,
            color: color,
          ),
        ),
      ],
    );
  }
}

/// 索引指标项组件
class _IndexMetricItem extends StatelessWidget {
  final String label;
  final String value;
  final IconData icon;

  const _IndexMetricItem({
    required this.label,
    required this.value,
    required this.icon,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Icon(
          icon,
          size: 20,
          color: AppColors.textMuted,
        ),
        const SizedBox(height: 4),
        Text(
          label,
          style: const TextStyle(
            fontSize: 11,
            color: AppColors.textMuted,
          ),
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 2),
        Text(
          value,
          style: const TextStyle(
            fontSize: 14,
            fontWeight: FontWeight.w600,
            color: AppColors.textPrimary,
          ),
          textAlign: TextAlign.center,
        ),
      ],
    );
  }
}
