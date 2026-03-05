import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/providers/search_history_provider.dart';
import '../../../../core/theme/app_theme.dart';

/// 搜索历史下拉组件
///
/// 显示历史搜索记录列表，支持点击快速填充和删除单条记录
/// 使用 PopupMenuButton 实现下拉交互
class SearchHistoryDropdown extends ConsumerWidget {
  /// 当前工作区 ID
  final String workspaceId;

  /// 选择历史记录回调（填充搜索框）
  final void Function(String query) onSelect;

  /// 删除单条历史记录回调（可选）
  final void Function(String query)? onDelete;

  const SearchHistoryDropdown({
    super.key,
    required this.workspaceId,
    required this.onSelect,
    this.onDelete,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // 监听搜索历史状态
    final historyAsync = ref.watch(searchHistoryProvider(workspaceId));

    return historyAsync.when(
      data: (history) => _buildDropdown(history),
      loading: () => _buildDisabledButton(),
      error: (_, __) => _buildDisabledButton(),
    );
  }

  /// 构建下拉按钮
  Widget _buildDropdown(List<SearchHistoryItem> history) {
    // 历史为空时显示禁用按钮
    if (history.isEmpty) {
      return _buildDisabledButton();
    }

    return PopupMenuButton<String>(
      icon: const Icon(
        Icons.history,
        size: 20,
        color: AppColors.textMuted,
      ),
      tooltip: '搜索历史',
      offset: const Offset(0, 40),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
      ),
      color: AppColors.bgCard,
      elevation: 8,
      constraints: const BoxConstraints(
        minWidth: 280,
        maxWidth: 400,
        maxHeight: 400,
      ),
      itemBuilder: (context) => history
          .map((item) => _buildHistoryItem(item, context))
          .toList(),
      onSelected: (query) => onSelect(query),
    );
  }

  /// 构建单条历史记录项
  PopupMenuItem<String> _buildHistoryItem(
    SearchHistoryItem item,
    BuildContext context,
  ) {
    return PopupMenuItem<String>(
      value: item.query,
      height: 48,
      padding: EdgeInsets.zero,
      child: StatefulBuilder(
        builder: (context, setState) {
          return MouseRegion(
            cursor: SystemMouseCursors.click,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Row(
                children: [
                  // 历史图标
                  const Icon(
                    Icons.history,
                    size: 16,
                    color: AppColors.textMuted,
                  ),
                  const SizedBox(width: 12),
                  // 查询文本
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text(
                          item.query,
                          style: const TextStyle(
                            color: AppColors.textPrimary,
                            fontSize: 14,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          _formatHistoryMeta(item),
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
                  // 删除按钮
                  if (onDelete != null)
                    InkWell(
                      onTap: () {
                        // 关闭下拉菜单
                        Navigator.of(context).pop();
                        // 调用删除回调
                        onDelete?.call(item.query);
                      },
                      borderRadius: BorderRadius.circular(4),
                      child: const Padding(
                        padding: EdgeInsets.all(4),
                        child: Icon(
                          Icons.close,
                          size: 16,
                          color: AppColors.textMuted,
                        ),
                      ),
                    ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  /// 格式化历史记录元数据
  String _formatHistoryMeta(SearchHistoryItem item) {
    // 格式化时间
    final timeStr = _formatTime(item.searchedAt);
    // 结果数量
    final countStr = '${item.resultCount} 条结果';
    return '$timeStr · $countStr';
  }

  /// 格式化时间为相对时间
  String _formatTime(String isoTime) {
    try {
      final dateTime = DateTime.parse(isoTime);
      final now = DateTime.now();
      final difference = now.difference(dateTime);

      // 1分钟内
      if (difference.inSeconds < 60) {
        return '刚刚';
      }
      // 1小时内
      if (difference.inMinutes < 60) {
        return '${difference.inMinutes} 分钟前';
      }
      // 今天内
      if (difference.inHours < 24 && dateTime.day == now.day) {
        return '${difference.inHours} 小时前';
      }
      // 昨天内
      if (difference.inDays == 1) {
        return '昨天';
      }
      // 一周内
      if (difference.inDays < 7) {
        return '${difference.inDays} 天前';
      }
      // 超过一周，显示日期
      return '${dateTime.month}/${dateTime.day}';
    } catch (_) {
      return '';
    }
  }

  /// 构建禁用状态的按钮
  Widget _buildDisabledButton() {
    return Tooltip(
      message: '暂无搜索历史',
      child: Icon(
        Icons.history,
        size: 20,
        color: AppColors.textMuted.withValues(alpha: 0.5),
      ),
    );
  }
}
