import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../core/theme/app_theme.dart';
import '../../../../shared/providers/log_level_stats_provider.dart';
import 'log_level_card.dart';
import 'log_level_distribution_chart.dart';

/// 日志级别统计面板主组件
///
/// 显示日志级别统计信息，包括：
/// - 级别卡片行（显示每个级别的计数）
/// - 级别分布饼图
/// - 实时更新（5秒自动刷新，由 LogLevelStatsProvider 处理）
class LogLevelStatsPanel extends ConsumerWidget {
  /// 工作区 ID
  final String workspaceId;

  /// 级别筛选回调（可选）
  final void Function(List<String> levels)? onLevelFilter;

  const LogLevelStatsPanel({
    super.key,
    required this.workspaceId,
    this.onLevelFilter,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // 监听日志级别统计状态
    final statsAsync = ref.watch(logLevelStatsProvider(workspaceId));

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: AppColors.bgCard,
        borderRadius: const BorderRadius.all(Radius.circular(8)),
        border: Border.all(color: AppColors.border, width: 1),
      ),
      child: statsAsync.when(
        loading: () => _buildLoading(),
        error: (error, stack) => _buildError(error.toString()),
        data: (stats) => _buildContent(stats),
      ),
    );
  }

  /// 构建加载状态
  Widget _buildLoading() {
    return const Column(
      children: [
        // 级别卡片行加载状态
        Row(
          children: [
            Expanded(
              child: SizedBox(
                height: 80,
                child: Center(
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            ),
          ],
        ),
        SizedBox(height: 16),
        // 饼图加载状态
        SizedBox(
          height: 180,
          child: Center(
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
        ),
      ],
    );
  }

  /// 构建错误状态
  Widget _buildError(String error) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(
            Icons.error_outline,
            color: AppColors.error,
            size: 24,
          ),
          const SizedBox(height: 8),
          Text(
            '加载失败: $error',
            style: const TextStyle(
              color: AppColors.error,
              fontSize: 14,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建内容
  Widget _buildContent(LogLevelStats stats) {
    // 处理空数据状态
    if (stats.total == 0) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.inbox_outlined,
                color: AppColors.textMuted,
                size: 48,
              ),
              SizedBox(height: 12),
              Text(
                '暂无日志数据',
                style: TextStyle(
                  color: AppColors.textMuted,
                  fontSize: 14,
                ),
              ),
            ],
          ),
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // 标题
        const Text(
          '日志级别统计',
          style: TextStyle(
            color: AppColors.textPrimary,
            fontSize: 16,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 16),
        // 级别卡片行（水平滚动）
        SizedBox(
          height: 100,
          child: ListView(
            scrollDirection: Axis.horizontal,
            children: [
              _buildLevelCard('FATAL', stats.fatalCount, stats.total),
              _buildLevelCard('ERROR', stats.errorCount, stats.total),
              _buildLevelCard('WARN', stats.warnCount, stats.total),
              _buildLevelCard('INFO', stats.infoCount, stats.total),
              _buildLevelCard('DEBUG', stats.debugCount, stats.total),
              _buildLevelCard('TRACE', stats.traceCount, stats.total),
              _buildLevelCard('UNKNOWN', stats.unknownCount, stats.total),
            ],
          ),
        ),
        const SizedBox(height: 20),
        // 饼图区域
        Center(
          child: LogLevelDistributionChart(
            stats: stats,
            onLevelTap: (level) {
              onLevelFilter?.call([level]);
            },
          ),
        ),
        const SizedBox(height: 16),
        // 总计数显示
        Center(
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            decoration: BoxDecoration(
              color: AppColors.bgInput,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Text(
              '总计: ${stats.total} 条',
              style: const TextStyle(
                color: AppColors.textPrimary,
                fontSize: 14,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
      ],
    );
  }

  /// 构建级别卡片
  Widget _buildLevelCard(String level, int count, int total) {
    return Padding(
      padding: const EdgeInsets.only(right: 12),
      child: LogLevelCard(
        level: level,
        count: count,
        total: total,
        color: LogLevelCard.getColorForLevel(level),
        onTap: onLevelFilter != null ? () => onLevelFilter!([level]) : null,
      ),
    );
  }
}
