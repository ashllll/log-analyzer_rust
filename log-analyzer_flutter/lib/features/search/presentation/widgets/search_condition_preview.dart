import 'package:flutter/material.dart';

import '../../../../core/theme/app_theme.dart';
import '../../../../shared/services/generated/ffi/types.dart' as ffi_types;
import '../../providers/search_query_provider.dart';

/// 搜索条件预览组件
///
/// 显示组合后的查询条件，如 "keyword1 AND keyword2 AND keyword3"
/// 用于在执行搜索前确认搜索条件
class SearchConditionPreview extends StatelessWidget {
  /// 关键词列表
  final List<SearchTerm> terms;

  /// 全局操作符
  final ffi_types.QueryOperatorData globalOperator;

  const SearchConditionPreview({
    super.key,
    required this.terms,
    required this.globalOperator,
  });

  @override
  Widget build(BuildContext context) {
    final previewText = _buildPreviewText();
    final hasKeywords = terms.where((t) => t.enabled).isNotEmpty;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: hasKeywords
            ? AppColors.primary.withOpacity(0.05)
            : AppColors.bgInput.withOpacity(0.3),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: hasKeywords
              ? AppColors.primary.withOpacity(0.2)
              : AppColors.border.withOpacity(0.5),
        ),
      ),
      child: Row(
        children: [
          // 图标
          Icon(
            hasKeywords ? Icons.manage_search : Icons.search_off,
            size: 18,
            color: hasKeywords ? AppColors.primary : AppColors.textMuted,
          ),
          const SizedBox(width: 8),
          // 预览文本
          Expanded(
            child: hasKeywords
                ? _buildRichPreview(previewText)
                : Text(
                    '无搜索条件',
                    style: TextStyle(
                      color: AppColors.textMuted,
                      fontSize: 13,
                      fontStyle: FontStyle.italic,
                    ),
                  ),
          ),
          // 条件数量标签
          if (hasKeywords)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
              decoration: BoxDecoration(
                color: AppColors.primary.withOpacity(0.15),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Text(
                '${terms.where((t) => t.enabled).length}',
                style: TextStyle(
                  color: AppColors.primary,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
        ],
      ),
    );
  }

  /// 构建预览文本
  String _buildPreviewText() {
    final enabledTerms = terms.where((t) => t.enabled).toList();
    if (enabledTerms.isEmpty) {
      return '无搜索条件';
    }

    final opStr = _getOperatorString(globalOperator);
    return enabledTerms.map((t) => t.value).join(opStr);
  }

  /// 获取操作符显示字符串
  String _getOperatorString(ffi_types.QueryOperatorData op) {
    switch (op) {
      case ffi_types.QueryOperatorData.and:
        return ' AND ';
      case ffi_types.QueryOperatorData.or:
        return ' OR ';
      case ffi_types.QueryOperatorData.not:
        return ' NOT ';
    }
  }

  /// 获取操作符颜色
  Color _getOperatorColor() {
    switch (globalOperator) {
      case ffi_types.QueryOperatorData.and:
        return AppColors.success;
      case ffi_types.QueryOperatorData.or:
        return AppColors.warning;
      case ffi_types.QueryOperatorData.not:
        return AppColors.error;
    }
  }

  /// 构建富文本预览
  ///
  /// 关键词使用主题色，操作符使用次要色
  Widget _buildRichPreview(String previewText) {
    final enabledTerms = terms.where((t) => t.enabled).toList();
    final opStr = _getOperatorString(globalOperator);
    final opColor = _getOperatorColor();

    final spans = <InlineSpan>[];
    for (int i = 0; i < enabledTerms.length; i++) {
      // 添加关键词
      spans.add(
        TextSpan(
          text: enabledTerms[i].value,
          style: TextStyle(
            color: AppColors.primary,
            fontSize: 13,
            fontWeight: FontWeight.w500,
          ),
        ),
      );

      // 添加操作符（除了最后一个）
      if (i < enabledTerms.length - 1) {
        spans.add(
          TextSpan(
            text: opStr,
            style: TextStyle(
              color: opColor,
              fontSize: 13,
              fontWeight: FontWeight.w600,
            ),
          ),
        );
      }
    }

    return RichText(
      text: TextSpan(children: spans),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }
}
