import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/providers/search_history_provider.dart';
import '../../../../core/theme/app_theme.dart';

/// 搜索历史下拉组件
///
/// 显示历史搜索记录列表，支持点击快速填充、删除单条记录和清空全部历史
/// 使用 PopupMenuButton 实现下拉交互
class SearchHistoryDropdown extends ConsumerWidget {
  /// 当前工作区 ID
  final String workspaceId;

  /// 选择历史记录回调（填充搜索框）
  final void Function(String query) onSelect;

  /// 删除单条历史记录回调（可选）
  final void Function(String query)? onDelete;

  /// 清空所有历史记录回调（可选）
  final VoidCallback? onClearAll;

  const SearchHistoryDropdown({
    super.key,
    required this.workspaceId,
    required this.onSelect,
    this.onDelete,
    this.onClearAll,
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

    return PopupMenuButton<_HistoryMenuValue>(
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
      itemBuilder: (context) => [
        // 历史记录列表
        ...history.map((item) => _buildHistoryItem(item, context)),
        // 分隔线
        if (history.isNotEmpty) const PopupMenuDivider(),
        // 清空全部按钮
        if (history.isNotEmpty && onClearAll != null)
          PopupMenuItem<_HistoryMenuValue>(
            value: _HistoryMenuValueClearAll(),
            height: 40,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                const Icon(
                  Icons.delete_sweep,
                  size: 18,
                  color: AppColors.error,
                ),
                const SizedBox(width: 8),
                const Text(
                  '清空全部历史',
                  style: TextStyle(
                    color: AppColors.error,
                    fontSize: 14,
                  ),
                ),
              ],
            ),
          ),
      ],
      onSelected: (value) {
        if (value is _HistoryMenuValueSelect) {
          onSelect(value.query);
        } else if (value is _HistoryMenuValueClearAll) {
          onClearAll?.call();
        }
      },
    );
  }

  /// 构建单条历史记录项
  PopupMenuItem<_HistoryMenuValue> _buildHistoryItem(
    SearchHistoryItem item,
    BuildContext context,
  ) {
    return PopupMenuItem<_HistoryMenuValue>(
      value: _HistoryMenuValueSelect(item.query),
      height: 48,
      padding: EdgeInsets.zero,
      child: StatefulBuilder(
        builder: (context, setState) {
          bool isHovering = false;
          return StatefulBuilder(
            builder: (context, setInnerState) {
              return MouseRegion(
                cursor: SystemMouseCursors.click,
                onEnter: (_) => setInnerState(() => isHovering = true),
                onExit: (_) => setInnerState(() => isHovering = false),
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
                        MouseRegion(
                          cursor: SystemMouseCursors.click,
                          child: GestureDetector(
                            onTap: () {
                              // 关闭下拉菜单
                              Navigator.of(context).pop();
                              // 调用删除回调
                              onDelete?.call(item.query);
                            },
                            child: Padding(
                              padding: const EdgeInsets.all(8),
                              child: Icon(
                                Icons.close,
                                size: 16,
                                color: isHovering ? AppColors.error : AppColors.textMuted,
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
              );
            },
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

/// 历史菜单值的基类
///
/// 使用 sealed class 实现类型安全的菜单值
sealed class _HistoryMenuValue {
  const _HistoryMenuValue();
}

/// 选择历史记录
class _HistoryMenuValueSelect extends _HistoryMenuValue {
  final String query;
  const _HistoryMenuValueSelect(this.query);
}

/// 清空全部历史
class _HistoryMenuValueClearAll extends _HistoryMenuValue {
  const _HistoryMenuValueClearAll();
}
