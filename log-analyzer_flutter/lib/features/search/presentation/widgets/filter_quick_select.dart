import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/models/saved_filter.dart';
import '../../../../shared/providers/saved_filters_provider.dart';
import '../../../../core/theme/app_theme.dart';

/// 过滤器快速选择回调
typedef FilterSelectCallback = void Function(SavedFilter filter);

/// 过滤器快速选择组件
///
/// 搜索栏右侧添加过滤器图标按钮，点击显示过滤器下拉列表
class FilterQuickSelect extends ConsumerWidget {
  /// 工作区ID
  final String workspaceId;

  /// 过滤器选择回调
  final FilterSelectCallback? onSelect;

  /// 打开过滤器编辑器回调
  final VoidCallback? onOpenEditor;

  const FilterQuickSelect({
    super.key,
    required this.workspaceId,
    this.onSelect,
    this.onOpenEditor,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final filtersAsync = ref.watch(savedFiltersProvider(workspaceId));

    return filtersAsync.when(
      data: (filters) => _buildFilterButton(context, filters),
      loading: () => _buildLoadingButton(),
      error: (_, __) => _buildErrorButton(),
    );
  }

  /// 构建过滤器按钮
  Widget _buildFilterButton(BuildContext context, List<SavedFilter> filters) {
    return PopupMenuButton<SavedFilter?>(
      tooltip: '选择过滤器',
      offset: const Offset(0, 40),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: AppColors.bgInput,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: AppColors.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              Icons.filter_alt_outlined,
              size: 18,
              color: AppColors.textSecondary,
            ),
            const SizedBox(width: 6),
            const Text(
              '过滤器',
              style: TextStyle(
                color: AppColors.textSecondary,
                fontSize: 13,
              ),
            ),
            if (filters.isNotEmpty) ...[
              const SizedBox(width: 6),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: AppColors.primary.withOpacity(0.2),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  '${filters.length}',
                  style: const TextStyle(
                    color: AppColors.primary,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
      onSelected: (filter) {
        if (filter != null) {
          onSelect?.call(filter);
        }
      },
      itemBuilder: (context) {
        final items = <PopupMenuEntry<SavedFilter?>>[];

        // 添加过滤器列表
        if (filters.isEmpty) {
          items.add(const PopupMenuItem<SavedFilter?>(
            value: null,
            enabled: false,
            child: Text(
              '暂无保存的过滤器',
              style: TextStyle(
                color: AppColors.textMuted,
                fontSize: 13,
              ),
            ),
          ));
        } else {
          // 显示前5个最近使用的过滤器
          final displayFilters = filters.take(5).toList();
          for (final filter in displayFilters) {
            items.add(PopupMenuItem<SavedFilter?>(
              value: filter,
              child: _FilterMenuItem(filter: filter),
            ));
          }
        }

        // 添加分隔线
        items.add(const PopupMenuDivider());

        // 添加保存当前过滤器选项
        items.add(PopupMenuItem<SavedFilter?>(
          value: null,
          onTap: onOpenEditor,
          child: const Row(
            children: [
              Icon(Icons.add, size: 18, color: AppColors.primary),
              SizedBox(width: 8),
              Text(
                '保存当前过滤器',
                style: TextStyle(
                  color: AppColors.primary,
                  fontSize: 13,
                ),
              ),
            ],
          ),
        ));

        return items;
      },
    );
  }

  /// 构建加载状态按钮
  Widget _buildLoadingButton() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: AppColors.bgInput,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.border),
      ),
      child: const SizedBox(
        width: 18,
        height: 18,
        child: CircularProgressIndicator(
          strokeWidth: 2,
          valueColor: AlwaysStoppedAnimation<Color>(AppColors.textMuted),
        ),
      ),
    );
  }

  /// 构建错误状态按钮
  Widget _buildErrorButton() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: AppColors.bgInput,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppColors.border),
      ),
      child: const Icon(
        Icons.filter_alt_outlined,
        size: 18,
        color: AppColors.error,
      ),
    );
  }
}

/// 过滤器菜单项组件
class _FilterMenuItem extends StatelessWidget {
  final SavedFilter filter;

  const _FilterMenuItem({required this.filter});

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        // 默认过滤器图标
        Icon(
          filter.isDefault ? Icons.star : Icons.filter_alt_outlined,
          size: 16,
          color: filter.isDefault ? AppColors.warning : AppColors.textMuted,
        ),
        const SizedBox(width: 8),
        // 过滤器信息
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                filter.name,
                style: const TextStyle(
                  color: AppColors.textPrimary,
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              // 条件摘要
              Text(
                _buildSummary(filter),
                style: const TextStyle(
                  color: AppColors.textMuted,
                  fontSize: 11,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
        // 使用次数
        if (filter.usageCount > 0)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: AppColors.bgMain,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Text(
              '${filter.usageCount}',
              style: const TextStyle(
                color: AppColors.textMuted,
                fontSize: 10,
              ),
            ),
          ),
      ],
    );
  }

  /// 构建条件摘要
  String _buildSummary(SavedFilter filter) {
    final parts = <String>[];

    // 关键词数量
    if (filter.terms.isNotEmpty) {
      parts.add('${filter.terms.length} 关键词');
    }

    // 级别列表
    if (filter.levels.isNotEmpty) {
      parts.add(filter.levels.join(', '));
    }

    // 时间范围
    if (filter.timeRange != null) {
      if (filter.timeRange!.start != null || filter.timeRange!.end != null) {
        parts.add('时间范围');
      }
    }

    // 文件模式
    if (filter.filePattern != null && filter.filePattern!.isNotEmpty) {
      parts.add(filter.filePattern!);
    }

    return parts.isEmpty ? '无过滤条件' : parts.join(' | ');
  }
}
