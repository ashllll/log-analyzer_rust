import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';

/// 日志级别卡片组件
///
/// 显示单个日志级别的统计信息，包括：
/// - 级别名称和图标
/// - 日志数量
/// - 百分比进度条
class LogLevelCard extends StatelessWidget {
  /// 日志级别名称 (如 FATAL, ERROR, WARN, INFO, DEBUG, TRACE, UNKNOWN)
  final String level;

  /// 该级别的日志数量
  final int count;

  /// 总日志数量
  final int total;

  /// 级别对应的颜色
  final Color color;

  /// 点击回调（可选）
  final VoidCallback? onTap;

  const LogLevelCard({
    super.key,
    required this.level,
    required this.count,
    required this.total,
    required this.color,
    this.onTap,
  });

  /// 根据级别名称获取对应的颜色
  static Color getColorForLevel(String level) {
    switch (level.toUpperCase()) {
      case 'FATAL':
      case 'ERROR':
        return AppColors.error;
      case 'WARN':
      case 'WARNING':
        return AppColors.warning;
      case 'INFO':
        return AppColors.primary;
      case 'DEBUG':
        return AppColors.keywordPurple;
      case 'TRACE':
      case 'UNKNOWN':
      default:
        return AppColors.textMuted;
    }
  }

  /// 根据级别名称获取对应的图标
  static IconData getIconForLevel(String level) {
    switch (level.toUpperCase()) {
      case 'FATAL':
        return Icons.error;
      case 'ERROR':
        return Icons.error_outline;
      case 'WARN':
      case 'WARNING':
        return Icons.warning_amber;
      case 'INFO':
        return Icons.info_outline;
      case 'DEBUG':
        return Icons.bug_report_outlined;
      case 'TRACE':
        return Icons.timeline;
      case 'UNKNOWN':
      default:
        return Icons.help_outline;
    }
  }

  /// 获取百分比
  double get percentage => total > 0 ? (count / total * 100) : 0;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 100,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: AppColors.bgCard,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: AppColors.border,
            width: 1,
          ),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 级别图标和名称
            Row(
              children: [
                Icon(
                  getIconForLevel(level),
                  color: color,
                  size: 16,
                ),
                const SizedBox(width: 4),
                Expanded(
                  child: Text(
                    level,
                    style: TextStyle(
                      color: color,
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            // 数量
            Text(
              count.toString(),
              style: const TextStyle(
                color: AppColors.textPrimary,
                fontSize: 18,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 4),
            // 百分比
            Text(
              '${percentage.toStringAsFixed(1)}%',
              style: const TextStyle(
                color: AppColors.textSecondary,
                fontSize: 11,
              ),
            ),
            const SizedBox(height: 8),
            // 百分比进度条
            ClipRRect(
              borderRadius: BorderRadius.circular(2),
              child: LinearProgressIndicator(
                value: percentage / 100,
                backgroundColor: AppColors.bgInput,
                valueColor: AlwaysStoppedAnimation<Color>(color),
                minHeight: 4,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
