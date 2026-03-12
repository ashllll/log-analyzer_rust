import 'package:flutter/material.dart';
import 'dart:ui' as ui;

import '../../../../shared/models/common.dart';
import '../../../../core/theme/app_theme.dart';

/// 日志行固定行高配置
///
/// PRD V6.0 4.1 要求：
/// - 强制使用 StrutStyle(forceStrutHeight: true)
/// - 彻底镇压多平台/多语言 Fallback 字体造成的行高突变
/// - 配合 SliverFixedExtentList 实现 O(1) 视口锚点
class LogRowStyle {
  /// 固定行高（必须在所有平台保持一致）
  static const double itemExtent = 36.0;

  /// 字体大小
  static const double fontSize = 13.0;

  /// 行高倍数（height * fontSize = 实际行高）
  /// 1.2 * 13 ≈ 15.6，加上上下 padding 凑成 36
  static const double lineHeight = 1.2;

  /// 等宽字体（用于日志内容）
  static const String monoFontFamily = 'FiraCode';

  /// 默认字体
  static const String defaultFontFamily = 'Roboto';

  /// 强制 StrutStyle - 镇压行高突变
  ///
  /// forceStrutHeight: true 强制所有文本使用统一行高
  /// 即使存在不同字体（如中文、日文、Emoji），也不会影响行高
  static const StrutStyle forcedStrutStyle = StrutStyle(
    forceStrutHeight: true,
    height: lineHeight,
    fontSize: fontSize,
    leadingDistribution: TextLeadingDistribution.even,
  );

  /// 等宽字体 StrutStyle
  static const StrutStyle monoStrutStyle = StrutStyle(
    forceStrutHeight: true,
    height: lineHeight,
    fontSize: fontSize,
    fontFamily: monoFontFamily,
    leadingDistribution: TextLeadingDistribution.even,
  );
}

/// 日志行组件
///
/// 对应 React 版本的 LogRowWidget
/// 功能：
/// - 关键词高亮显示
/// - 强制 StrutStyle 镇压行高突变
/// - 点击展开详情
/// - 配合 SliverFixedExtentList 实现确定性虚拟滚动
///
/// PRD V6.0 4.1 要求：
/// - 必须使用 StrutStyle(forceStrutHeight: true)
/// - itemExtent 必须与 SliverFixedExtentList 一致
class LogRowWidget extends StatelessWidget {
  final LogEntry log;
  final bool isActive;
  final VoidCallback? onTap;
  final List<String>? matchedKeywords;

  /// 固定行高（从父组件传入，确保一致性）
  final double itemExtent;

  const LogRowWidget({
    super.key,
    required this.log,
    required this.isActive,
    this.onTap,
    this.matchedKeywords,
    this.itemExtent = LogRowStyle.itemExtent,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final matchedWords = matchedKeywords ?? log.matchedKeywords ?? [];

    return InkWell(
      onTap: onTap,
      child: Container(
        // 使用固定高度，与 SliverFixedExtentList 的 itemExtent 一致
        height: itemExtent,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        decoration: BoxDecoration(
          color: isActive ? AppColors.bgHover : Colors.transparent,
          border: Border(
            bottom: BorderSide(color: theme.dividerColor, width: 0.5),
          ),
        ),
        // 使用强制 StrutStyle 的文本组件
        child: _buildLogContent(theme, matchedWords),
      ),
    );
  }

  /// 构建日志内容行
  ///
  /// 使用 Row 布局，所有子组件都强制使用 StrutStyle
  Widget _buildLogContent(ThemeData theme, List<String> matchedWords) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        // 时间戳
        SizedBox(
          width: 140,
          child: Text(
            log.timestamp,
            style: const TextStyle(
              fontFamily: LogRowStyle.monoFontFamily,
              fontSize: LogRowStyle.fontSize,
              color: AppColors.textMuted,
              fontFeatures: [ui.FontFeature.tabularFigures()],
            ),
            strutStyle: LogRowStyle.monoStrutStyle,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        const SizedBox(width: 12),
        // 日志级别
        _buildLevelChip(log.level),
        const SizedBox(width: 12),
        // 文件和行号
        SizedBox(
          width: 120,
          child: Text(
            '${log.file}:${log.line}',
            style: const TextStyle(
              fontSize: LogRowStyle.fontSize,
              color: AppColors.textMuted,
            ),
            strutStyle: LogRowStyle.forcedStrutStyle,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        const SizedBox(width: 12),
        // 日志内容（带高亮）
        Expanded(child: _buildHighlightedContent(log.content, matchedWords)),
      ],
    );
  }

  /// 构建日志级别标签
  ///
  /// 使用紧凑布局和强制 StrutStyle 确保行高一致
  Widget _buildLevelChip(String level) {
    Color color;
    switch (level.toUpperCase()) {
      case 'ERROR':
      case 'FATAL':
        color = AppColors.error;
        break;
      case 'WARN':
      case 'WARNING':
        color = AppColors.warning;
        break;
      case 'INFO':
        color = AppColors.primary;
        break;
      case 'DEBUG':
        color = AppColors.keywordPurple;
        break;
      case 'TRACE':
        color = AppColors.textMuted;
        break;
      default:
        color = AppColors.textMuted;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.3), width: 1),
      ),
      child: Text(
        level.toUpperCase(),
        style: TextStyle(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
          height: 1.0,
        ),
        strutStyle: const StrutStyle(
          forceStrutHeight: true,
          fontSize: 11,
          height: 1.0,
        ),
      ),
    );
  }

  /// 构建带高亮的日志内容
  ///
  /// 使用 Text.rich 配合强制 StrutStyle 确保行高一致
  Widget _buildHighlightedContent(
    String content,
    List<String> matchedKeywords,
  ) {
    if (matchedKeywords.isEmpty) {
      return Text(
        content,
        style: const TextStyle(
          fontFamily: LogRowStyle.monoFontFamily,
          fontSize: LogRowStyle.fontSize,
          color: AppColors.textSecondary,
          fontFeatures: [ui.FontFeature.tabularFigures()],
        ),
        strutStyle: LogRowStyle.monoStrutStyle,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      );
    }

    return Text.rich(
      TextSpan(
        children: _buildHighlightedSpans(content, matchedKeywords),
        style: const TextStyle(
          fontFamily: LogRowStyle.monoFontFamily,
          fontSize: LogRowStyle.fontSize,
          fontFeatures: [ui.FontFeature.tabularFigures()],
        ),
      ),
      strutStyle: LogRowStyle.monoStrutStyle,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }

  /// 构建高亮文本片段
  ///
  /// 所有 TextSpan 都使用等宽字体和统一 fontSize
  List<TextSpan> _buildHighlightedSpans(String content, List<String> keywords) {
    final spans = <TextSpan>[];
    var currentIndex = 0;

    // 收集所有匹配位置
    final matches = <_MatchPosition>[];
    for (final keyword in keywords) {
      var index = content.indexOf(keyword);
      while (index != -1) {
        matches.add(
          _MatchPosition(
            keyword: keyword,
            index: index,
            length: keyword.length,
          ),
        );
        index = content.indexOf(keyword, index + keyword.length);
      }
    }

    // 按位置排序
    matches.sort((a, b) => a.index.compareTo(b.index));

    // 构建文本片段
    for (final match in matches) {
      // 添加普通文本
      if (match.index > currentIndex) {
        spans.add(
          TextSpan(
            text: content.substring(currentIndex, match.index),
            style: const TextStyle(
              fontFamily: LogRowStyle.monoFontFamily,
              fontSize: LogRowStyle.fontSize,
              color: AppColors.textSecondary,
              fontFeatures: [ui.FontFeature.tabularFigures()],
            ),
          ),
        );
      }

      // 添加高亮文本
      final color = _getHighlightColor(match.keyword);
      spans.add(
        TextSpan(
          text: content.substring(match.index, match.index + match.length),
          style: TextStyle(
            fontFamily: LogRowStyle.monoFontFamily,
            fontSize: LogRowStyle.fontSize,
            backgroundColor: color.withOpacity(0.3),
            color: color,
            fontWeight: FontWeight.bold,
            fontFeatures: const [ui.FontFeature.tabularFigures()],
          ),
        ),
      );

      currentIndex = match.index + match.length;
    }

    // 添加剩余文本
    if (currentIndex < content.length) {
      spans.add(
        TextSpan(
          text: content.substring(currentIndex),
          style: const TextStyle(
            fontFamily: LogRowStyle.monoFontFamily,
            fontSize: LogRowStyle.fontSize,
            color: AppColors.textSecondary,
            fontFeatures: [ui.FontFeature.tabularFigures()],
          ),
        ),
      );
    }

    return spans;
  }

  /// 根据关键词获取高亮颜色
  Color _getHighlightColor(String keyword) {
    final colors = [
      AppColors.keywordBlue,
      AppColors.keywordGreen,
      AppColors.keywordRed,
      AppColors.keywordOrange,
      AppColors.keywordPurple,
    ];
    final hash = keyword.hashCode.abs();
    return colors[hash % colors.length];
  }
}

/// 匹配位置信息
class _MatchPosition {
  final String keyword;
  final int index;
  final int length;

  _MatchPosition({
    required this.keyword,
    required this.index,
    required this.length,
  });
}
