import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/models/saved_filter.dart';
import '../../../../shared/models/common.dart' as common;
import '../../../../shared/providers/saved_filters_provider.dart';
import '../../../../core/theme/app_theme.dart';
import 'filter_editor_dialog.dart';

/// 过滤器应用回调类型
typedef FilterApplyCallback = void Function(SavedFilter filter);

/// 获取当前过滤器配置的回调类型
typedef GetCurrentFiltersCallback = common.FilterOptions? Function();

/// 侧边栏过滤器列表组件
///
/// 显示已保存的过滤器列表，支持点击应用、编辑和删除操作
class SavedFiltersSidebar extends ConsumerStatefulWidget {
  /// 工作区ID
  final String workspaceId;

  /// 过滤器应用回调
  final FilterApplyCallback? onApply;

  /// 过滤器编辑回调
  final void Function(SavedFilter filter)? onEdit;

  /// 过滤器删除回调
  final void Function(String filterId)? onDelete;

  /// 获取当前过滤器配置的回调（用于预填充新建过滤器的条件）
  final GetCurrentFiltersCallback? getCurrentFilters;

  const SavedFiltersSidebar({
    super.key,
    required this.workspaceId,
    this.onApply,
    this.onEdit,
    this.onDelete,
    this.getCurrentFilters,
  });

  @override
  ConsumerState<SavedFiltersSidebar> createState() => _SavedFiltersSidebarState();
}

class _SavedFiltersSidebarState extends ConsumerState<SavedFiltersSidebar> {
  @override
  Widget build(BuildContext context) {
    final filtersAsync = ref.watch(savedFiltersProvider(widget.workspaceId));

    return Container(
      decoration: const BoxDecoration(
        color: AppColors.bgCard,
        border: Border(
          bottom: BorderSide(color: AppColors.border, width: 1),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          // 标题栏
          _buildHeader(context),
          // 过滤器列表
          filtersAsync.when(
            data: (filters) => _buildFilterList(context, filters),
            loading: _buildLoading,
            error: (error, stack) => _buildError(error.toString()),
          ),
        ],
      ),
    );
  }

  /// 构建标题栏
  Widget _buildHeader(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: const BoxDecoration(
        border: Border(
          bottom: BorderSide(color: AppColors.border, width: 1),
        ),
      ),
      child: Row(
        children: [
          const Icon(
            Icons.filter_alt_outlined,
            size: 18,
            color: AppColors.textSecondary,
          ),
          const SizedBox(width: 8),
          const Text(
            '已保存的过滤器',
            style: TextStyle(
              color: AppColors.textPrimary,
              fontSize: 14,
              fontWeight: FontWeight.w600,
            ),
          ),
          const Spacer(),
          IconButton(
            icon: const Icon(Icons.add, size: 18),
            tooltip: '创建新过滤器',
            onPressed: () => _showCreateDialog(context),
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(
              minWidth: 28,
              minHeight: 28,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建过滤器列表
  Widget _buildFilterList(BuildContext context, List<SavedFilter> filters) {
    if (filters.isEmpty) {
      return _buildEmptyState();
    }

    return ConstrainedBox(
      constraints: const BoxConstraints(maxHeight: 300),
      child: ListView.builder(
        shrinkWrap: true,
        padding: const EdgeInsets.symmetric(vertical: 8),
        itemCount: filters.length,
        itemBuilder: (context, index) {
          final filter = filters[index];
          return _FilterListItem(
            filter: filter,
            onTap: () => widget.onApply?.call(filter),
            onEdit: () => widget.onEdit?.call(filter),
            onDelete: () => widget.onDelete?.call(filter.id),
          );
        },
      ),
    );
  }

  /// 构建空状态
  Widget _buildEmptyState() {
    return const Padding(
      padding: EdgeInsets.all(24),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.filter_alt_off_outlined,
              size: 40,
              color: AppColors.textMuted,
            ),
            SizedBox(height: 12),
            Text(
              '暂无保存的过滤器',
              style: TextStyle(
                color: AppColors.textMuted,
                fontSize: 13,
              ),
            ),
            SizedBox(height: 4),
            Text(
              '点击 + 创建第一个过滤器',
              style: TextStyle(
                color: AppColors.textMuted,
                fontSize: 12,
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// 构建加载状态
  Widget _buildLoading() {
    return const Padding(
      padding: EdgeInsets.all(24),
      child: Center(
        child: SizedBox(
          width: 24,
          height: 24,
          child: CircularProgressIndicator(
            strokeWidth: 2,
            valueColor: AlwaysStoppedAnimation<Color>(AppColors.primary),
          ),
        ),
      ),
    );
  }

  /// 构建错误状态
  Widget _buildError(String error) {
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              Icons.error_outline,
              size: 40,
              color: AppColors.error,
            ),
            const SizedBox(height: 12),
            Text(
              '加载失败: $error',
              style: const TextStyle(
                color: AppColors.error,
                fontSize: 13,
              ),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }

  /// 显示创建对话框
  Future<void> _showCreateDialog(BuildContext context) async {
    // 获取当前过滤器配置用于预填充
    final currentFilters = widget.getCurrentFilters?.call();

    // 调用 FilterEditorDialog 创建新过滤器
    final result = await FilterEditorDialog.show(
      context,
      workspaceId: widget.workspaceId,
      filter: null, // null 表示创建新过滤器
      currentFilters: currentFilters,
    );

    // 如果保存成功，通过 onApply 回调通知父组件
    if (result != null && widget.onApply != null) {
      // FilterEditorDialog 已经保存到 provider，这里只需要通知父组件
      // 可以选择刷新搜索或显示提示信息
    }
  }
}

/// 过滤器列表项组件
class _FilterListItem extends StatelessWidget {
  final SavedFilter filter;
  final VoidCallback onTap;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _FilterListItem({
    required this.filter,
    required this.onTap,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        child: Row(
          children: [
            // 默认过滤器图标
            Icon(
              filter.isDefault ? Icons.star : Icons.filter_alt_outlined,
              size: 16,
              color: filter.isDefault ? AppColors.warning : AppColors.textMuted,
            ),
            const SizedBox(width: 12),
            // 过滤器信息
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  // 名称
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
            // 操作按钮
            PopupMenuButton<String>(
              icon: const Icon(Icons.more_vert, size: 16),
              padding: EdgeInsets.zero,
              tooltip: '更多操作',
              onSelected: (value) {
                switch (value) {
                  case 'edit':
                    onEdit();
                    break;
                  case 'delete':
                    onDelete();
                    break;
                }
              },
              itemBuilder: (context) => [
                const PopupMenuItem(
                  value: 'edit',
                  child: Row(
                    children: [
                      Icon(Icons.edit, size: 16),
                      SizedBox(width: 8),
                      Text('编辑'),
                    ],
                  ),
                ),
                const PopupMenuItem(
                  value: 'delete',
                  child: Row(
                    children: [
                      Icon(Icons.delete, size: 16, color: AppColors.error),
                      SizedBox(width: 8),
                      Text('删除', style: TextStyle(color: AppColors.error)),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// 构建条件摘要
  String _buildSummary(SavedFilter filter) {
    final parts = <String>[];

    // 关键词数量
    if (filter.terms.isNotEmpty) {
      parts.add('${filter.terms.length} 个关键词');
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
