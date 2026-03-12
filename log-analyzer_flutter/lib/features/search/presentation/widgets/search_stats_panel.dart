import 'package:flutter/material.dart';

import '../../../../shared/models/search.dart';
import '../../../../core/theme/app_theme.dart';

/// 搜索统计面板组件
///
/// 对应 React 版本的 KeywordStatsPanel
/// 显示：
/// - 总匹配数量
/// - 搜索耗时
/// - 各关键词统计（数量 + 百分比）
class SearchStatsPanel extends StatelessWidget {
  final SearchResultSummary? summary;
  final VoidCallback? onExport;
  final bool isLoading;

  const SearchStatsPanel({
    super.key,
    this.summary,
    this.onExport,
    this.isLoading = false,
  });

  @override
  Widget build(BuildContext context) {
    if (summary == null && !isLoading) {
      return const SizedBox.shrink();
    }

    final stats = summary;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: const BoxDecoration(
        color: AppColors.bgCard,
        border: Border(top: BorderSide(color: AppColors.border, width: 1)),
      ),
      child: isLoading
          ? _buildLoading()
          : stats != null
          ? _buildStats(stats)
          : const SizedBox.shrink(),
    );
  }

  /// 构建加载状态
  Widget _buildLoading() {
    return const Row(
      children: [
        SizedBox(
          width: 16,
          height: 16,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
        SizedBox(width: 12),
        Text(
          '搜索中...',
          style: TextStyle(color: AppColors.textSecondary, fontSize: 14),
        ),
      ],
    );
  }

  /// 构建统计信息
  Widget _buildStats(SearchResultSummary summary) {
    return Row(
      children: [
        // 总体统计
        Expanded(child: _buildSummaryText(summary)),
        // 导出按钮
        if (onExport != null)
          TextButton.icon(
            onPressed: onExport,
            icon: const Icon(Icons.download, size: 16),
            label: const Text('导出'),
            style: TextButton.styleFrom(
              foregroundColor: AppColors.textSecondary,
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            ),
          ),
      ],
    );
  }

  /// 构建摘要文本
  Widget _buildSummaryText(SearchResultSummary summary) {
    final duration = summary.durationMs < 1000
        ? '${summary.durationMs}ms'
        : '${(summary.durationMs / 1000).toStringAsFixed(1)}s';

    return Wrap(
      spacing: 16,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        _buildStatItem('总计', '${summary.totalCount} 条', AppColors.primary),
        _buildStatItem('耗时', duration, AppColors.textSecondary),
        // 关键词统计
        ...summary.keywordStats.take(3).map((stat) => _buildKeywordStat(stat)),
        if (summary.keywordStats.length > 3)
          _buildMoreIndicator(summary.keywordStats.length - 3),
      ],
    );
  }

  /// 构建统计项
  Widget _buildStatItem(String label, String value, Color color) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          '$label: ',
          style: const TextStyle(color: AppColors.textMuted, fontSize: 13),
        ),
        Text(
          value,
          style: TextStyle(
            color: color,
            fontSize: 13,
            fontWeight: FontWeight.w600,
          ),
        ),
      ],
    );
  }

  /// 构建关键词统计项
  Widget _buildKeywordStat(KeywordStatistic stat) {
    final color = AppColors.fromColorKey(stat.keyword);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: color.withOpacity(0.3), width: 1),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            stat.keyword,
            style: TextStyle(
              color: color,
              fontSize: 12,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            '${stat.matchCount} (${stat.matchPercentage.toStringAsFixed(1)}%)',
            style: TextStyle(
              color: color.withOpacity(0.8),
              fontSize: 12,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建"更多"指示器
  Widget _buildMoreIndicator(int remaining) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: AppColors.bgHover,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        '+$remaining',
        style: const TextStyle(
          color: AppColors.textMuted,
          fontSize: 12,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}
