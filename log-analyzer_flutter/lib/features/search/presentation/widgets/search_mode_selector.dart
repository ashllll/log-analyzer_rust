import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';
import '../../models/search_mode.dart';

/// 搜索模式选择器组件
///
/// 使用 Material Design 3 的 SegmentedButton 实现
/// 支持三种搜索模式：普通、正则、组合
///
/// 用法：
/// ```dart
/// SearchModeSelector(
///   currentMode: _searchMode,
///   onModeChanged: (mode) => setState(() => _searchMode = mode),
/// )
/// ```
class SearchModeSelector extends StatelessWidget {
  /// 当前选中的搜索模式
  final SearchMode currentMode;

  /// 模式变化回调
  final ValueChanged<SearchMode> onModeChanged;

  const SearchModeSelector({
    super.key,
    required this.currentMode,
    required this.onModeChanged,
  });

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<SearchMode>(
      segments: _buildSegments(),
      selected: {currentMode},
      onSelectionChanged: _handleSelectionChanged,
      style: _buildButtonStyle(),
    );
  }

  /// 构建分段按钮选项
  List<ButtonSegment<SearchMode>> _buildSegments() {
    return const [
      ButtonSegment<SearchMode>(
        value: SearchMode.normal,
        label: Text('普通'),
        icon: Icon(Icons.search, size: 18),
        tooltip: '简单文本搜索',
      ),
      ButtonSegment<SearchMode>(
        value: SearchMode.regex,
        label: Text('正则'),
        icon: Icon(Icons.code, size: 18),
        tooltip: '正则表达式搜索',
      ),
      ButtonSegment<SearchMode>(
        value: SearchMode.combined,
        label: Text('组合'),
        icon: Icon(Icons.manage_search, size: 18),
        tooltip: '组合搜索（正则 + 关键词）',
      ),
    ];
  }

  /// 处理选择变化
  void _handleSelectionChanged(Set<SearchMode> selection) {
    if (selection.isNotEmpty) {
      onModeChanged(selection.first);
    }
  }

  /// 构建按钮样式
  ///
  /// 使用 AppTheme 统一样式，不硬编码颜色
  ButtonStyle _buildButtonStyle() {
    return SegmentedButton.styleFrom(
      // 选中状态背景色
      selectedBackgroundColor: AppColors.primary,
      selectedForegroundColor: Colors.white,
      // 未选中状态
      backgroundColor: AppColors.bgCard,
      foregroundColor: AppColors.textSecondary,
      // 边框
      side: const BorderSide(color: AppColors.border, width: 1),
      // 形状
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
      ),
      // 内边距
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      // 文字样式
      textStyle: const TextStyle(
        fontSize: 13,
        fontWeight: FontWeight.w500,
      ),
    );
  }
}
