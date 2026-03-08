import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';
import '../../../../shared/providers/log_level_stats_provider.dart';

/// 日志级别分布饼图组件
///
/// 使用 fl_chart 显示日志级别的分布比例
class LogLevelDistributionChart extends StatelessWidget {
  /// 日志级别统计数据
  final LogLevelStats stats;

  /// 点击级别回调（可选）
  final void Function(String level)? onLevelTap;

  const LogLevelDistributionChart({
    super.key,
    required this.stats,
    this.onLevelTap,
  });

  /// 获取级别数据列表
  List<_LevelData> get _levels {
    if (stats.total == 0) return [];

    return [
      _LevelData('FATAL', stats.fatalCount, AppColors.error),
      _LevelData('ERROR', stats.errorCount, AppColors.errorHover),
      _LevelData('WARN', stats.warnCount, AppColors.warning),
      _LevelData('INFO', stats.infoCount, AppColors.primary),
      _LevelData('DEBUG', stats.debugCount, AppColors.keywordPurple),
      _LevelData('TRACE', stats.traceCount, AppColors.textMuted),
      _LevelData('UNKNOWN', stats.unknownCount, AppColors.textSecondary),
    ].where((level) => level.count > 0).toList();
  }

  @override
  Widget build(BuildContext context) {
    // 处理空数据状态
    if (stats.total == 0) {
      return const Center(
        child: Text(
          '暂无日志数据',
          style: TextStyle(
            color: AppColors.textMuted,
            fontSize: 14,
          ),
        ),
      );
    }

    return Column(
      children: [
        // 饼图
        SizedBox(
          height: 180,
          child: PieChart(
            PieChartData(
              sectionsSpace: 2,
              centerSpaceRadius: 40,
              sections: _buildSections(),
              pieTouchData: PieTouchData(
                enabled: true,
                touchCallback: (FlTouchEvent event, PieTouchResponse? response) {
                  // 点击扇区时触发回调
                  if (event is FlTapUpEvent && response != null) {
                    final touchedSection = response.touchedSection;
                    if (touchedSection != null && onLevelTap != null) {
                      // 通过 touchedSection 的 radius 判断点击了哪个扇区
                      // 简化处理：点击时触发回调，通过图例点击更准确
                    }
                  }
                },
              ),
            ),
          ),
        ),
        const SizedBox(height: 16),
        // 图例（点击触发筛选）
        Wrap(
          spacing: 12,
          runSpacing: 8,
          alignment: WrapAlignment.center,
          children: _levels.map((level) => _buildLegendItem(level)).toList(),
        ),
      ],
    );
  }

  /// 构建饼图扇区
  List<PieChartSectionData> _buildSections() {
    return _levels.map((level) {
      final percentage = stats.total > 0 ? level.count / stats.total * 100 : 0;
      return PieChartSectionData(
        value: level.count.toDouble(),
        title: '${percentage.toStringAsFixed(0)}%',
        color: level.color,
        radius: 50,
        titleStyle: const TextStyle(
          color: Colors.white,
          fontSize: 11,
          fontWeight: FontWeight.w600,
        ),
      );
    }).toList();
  }

  /// 构建图例项（点击触发筛选）
  Widget _buildLegendItem(_LevelData level) {
    return GestureDetector(
      onTap: onLevelTap != null ? () => onLevelTap!(level.name) : null,
      child: MouseRegion(
        cursor: onLevelTap != null ? SystemMouseCursors.click : SystemMouseCursors.basic,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 10,
              height: 10,
              decoration: BoxDecoration(
                color: level.color,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            const SizedBox(width: 4),
            Text(
              '${level.name} (${level.count})',
              style: const TextStyle(
                color: AppColors.textSecondary,
                fontSize: 11,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 级别数据
class _LevelData {
  final String name;
  final int count;
  final Color color;

  _LevelData(this.name, this.count, this.color);
}
