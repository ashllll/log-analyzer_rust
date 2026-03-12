import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../../../../core/theme/app_theme.dart';
import '../../../../shared/services/generated/ffi/types.dart' as ffi_types;
import '../../providers/search_query_provider.dart';

/// 多关键词输入组件
///
/// 支持添加多个关键词并选择 AND/OR/NOT 逻辑组合
/// 包含：
/// - 顶部: SegmentedButton 选择 AND/OR/NOT
/// - 中部: Wrap 显示已添加的关键词 Chip
/// - 底部: TextField 输入新关键词
class MultiKeywordInput extends ConsumerStatefulWidget {
  /// 关键词列表
  final List<SearchTerm> terms;

  /// 全局操作符
  final ffi_types.QueryOperatorData globalOperator;

  /// 关键词变更回调
  final ValueChanged<List<SearchTerm>> onTermsChanged;

  /// 操作符变更回调
  final ValueChanged<ffi_types.QueryOperatorData> onOperatorChanged;

  const MultiKeywordInput({
    super.key,
    required this.terms,
    required this.globalOperator,
    required this.onTermsChanged,
    required this.onOperatorChanged,
  });

  @override
  ConsumerState<MultiKeywordInput> createState() => _MultiKeywordInputState();
}

class _MultiKeywordInputState extends ConsumerState<MultiKeywordInput> {
  final _inputController = TextEditingController();
  final _focusNode = FocusNode();

  @override
  void dispose() {
    _inputController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  /// 添加关键词
  void _addKeyword() {
    final value = _inputController.text.trim();
    if (value.isEmpty) return;

    // 检查重复
    if (widget.terms.any((t) => t.value == value)) {
      _showDuplicateWarning(value);
      return;
    }

    final newTerm = SearchTerm(
      id: const Uuid().v4(),
      value: value,
      operator_: widget.globalOperator,
      isRegex: false,
      priority: widget.terms.length,
      enabled: true,
      caseSensitive: false,
    );

    widget.onTermsChanged([...widget.terms, newTerm]);
    _inputController.clear();
  }

  /// 显示重复提示
  void _showDuplicateWarning(String value) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('关键词 "$value" 已存在'),
        duration: const Duration(seconds: 2),
        backgroundColor: AppColors.warning,
      ),
    );
  }

  /// 删除关键词
  void _removeKeyword(String id) {
    widget.onTermsChanged(widget.terms.where((t) => t.id != id).toList());
  }

  /// 切换关键词启用状态
  void _toggleKeyword(String id) {
    widget.onTermsChanged(
      widget.terms.map((t) {
        if (t.id == id) {
          return t.copyWith(enabled: !t.enabled);
        }
        return t;
      }).toList(),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: AppColors.bgCard,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          // 操作符选择器
          _buildOperatorSelector(),
          const SizedBox(height: 12),
          // 已添加的关键词列表
          if (widget.terms.isNotEmpty) ...[
            _buildKeywordChips(),
            const SizedBox(height: 12),
          ],
          // 输入框
          _buildInputField(),
        ],
      ),
    );
  }

  /// 构建操作符选择器
  Widget _buildOperatorSelector() {
    return SegmentedButton<ffi_types.QueryOperatorData>(
      segments: const [
        ButtonSegment(
          value: ffi_types.QueryOperatorData.and,
          label: Text('AND'),
          icon: Icon(Icons.add_circle_outline, size: 18),
        ),
        ButtonSegment(
          value: ffi_types.QueryOperatorData.or,
          label: Text('OR'),
          icon: Icon(Icons.alt_route_outlined, size: 18),
        ),
        ButtonSegment(
          value: ffi_types.QueryOperatorData.not,
          label: Text('NOT'),
          icon: Icon(Icons.remove_circle_outline, size: 18),
        ),
      ],
      selected: {widget.globalOperator},
      onSelectionChanged: (Set<ffi_types.QueryOperatorData> selection) {
        widget.onOperatorChanged(selection.first);
      },
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) {
            return AppColors.primary.withOpacity(0.2);
          }
          return AppColors.bgInput;
        }),
        foregroundColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) {
            return AppColors.primary;
          }
          return AppColors.textSecondary;
        }),
        side: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) {
            return const BorderSide(color: AppColors.primary, width: 1.5);
          }
          return const BorderSide(color: AppColors.border);
        }),
      ),
    );
  }

  /// 构建关键词 Chip 列表
  Widget _buildKeywordChips() {
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: widget.terms.map((term) => _buildKeywordChip(term)).toList(),
    );
  }

  /// 构建单个关键词 Chip
  Widget _buildKeywordChip(SearchTerm term) {
    final backgroundColor = term.enabled
        ? AppColors.primary.withOpacity(0.1)
        : AppColors.bgInput.withOpacity(0.5);

    final textColor = term.enabled ? AppColors.primary : AppColors.textMuted;

    return Tooltip(
      message: term.enabled ? '点击禁用' : '点击启用',
      child: InputChip(
        label: Text(
          term.value,
          style: TextStyle(
            color: textColor,
            fontWeight: FontWeight.w500,
            decoration: term.enabled ? null : TextDecoration.lineThrough,
          ),
        ),
        backgroundColor: backgroundColor,
        side: BorderSide(
          color: term.enabled
              ? AppColors.primary.withOpacity(0.3)
              : AppColors.border,
        ),
        deleteIcon: const Icon(Icons.close, size: 16),
        deleteIconColor: AppColors.textMuted,
        onDeleted: () => _removeKeyword(term.id),
        onPressed: () => _toggleKeyword(term.id),
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
      ),
    );
  }

  /// 构建输入框
  Widget _buildInputField() {
    return TextField(
      controller: _inputController,
      focusNode: _focusNode,
      decoration: InputDecoration(
        hintText: '输入关键词后按 Enter 或点击添加',
        hintStyle: const TextStyle(color: AppColors.textMuted, fontSize: 14),
        prefixIcon: const Icon(Icons.add, size: 20, color: AppColors.textMuted),
        suffixIcon: IconButton(
          icon: const Icon(Icons.add_circle, color: AppColors.primary),
          tooltip: '添加关键词',
          onPressed: _addKeyword,
        ),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.border),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(8),
          borderSide: const BorderSide(color: AppColors.primary, width: 1.5),
        ),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 12,
          vertical: 10,
        ),
        isDense: true,
      ),
      style: const TextStyle(fontSize: 14),
      onSubmitted: (_) => _addKeyword(),
    );
  }
}
