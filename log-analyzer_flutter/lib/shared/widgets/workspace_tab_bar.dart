import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/workspace_tab.dart';
import '../providers/app_provider.dart';
import '../providers/workspace_tab_provider.dart';
import 'workspace_picker_dialog.dart';

/// 工作区标签栏组件
///
/// 显示所有打开的工作区标签，支持点击切换、拖拽重排、关闭按钮
class WorkspaceTabBar extends ConsumerWidget {
  const WorkspaceTabBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tabs = ref.watch(tabManagerProvider);
    final activeTabId = ref.watch(activeTabIdProvider);

    if (tabs.isEmpty) {
      return const SizedBox.shrink();
    }

    return Container(
      height: 40,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        border: Border(
          bottom: BorderSide(
            color: Theme.of(context).dividerColor,
            width: 1,
          ),
        ),
      ),
      child: Row(
        children: [
          // 标签列表
          Expanded(
            child: ReorderableListView.builder(
              scrollDirection: Axis.horizontal,
              buildDefaultDragHandles: false,
              itemCount: tabs.length,
              onReorder: (oldIndex, newIndex) {
                ref.read(tabManagerProvider.notifier).reorderTabs(oldIndex, newIndex);
              },
              itemBuilder: (context, index) {
                final tab = tabs[index];
                final isActive = tab.id == activeTabId;

                return ReorderableDragStartListener(
                  key: ValueKey(tab.id),
                  index: index,
                  child: _WorkspaceTabItem(
                    tab: tab,
                    isActive: isActive,
                    onTap: () {
                      ref.read(activeTabIdProvider.notifier).setActive(tab.id);
                      // 同时切换活动工作区
                      ref.read(appStateProvider.notifier).setActiveWorkspace(tab.workspaceId);
                    },
                    onClose: () {
                      ref.read(tabManagerProvider.notifier).closeTab(tab.id);
                    },
                    onPin: () {
                      ref.read(tabManagerProvider.notifier).togglePin(tab.id);
                    },
                  ),
                );
              },
            ),
          ),
          // 添加新标签按钮
          _AddTabButton(
            onPressed: () => _showWorkspacePicker(context, ref),
          ),
        ],
      ),
    );
  }

  Future<void> _showWorkspacePicker(BuildContext context, WidgetRef ref) async {
    final result = await showDialog<WorkspacePickerResult>(
      context: context,
      builder: (context) => const WorkspacePickerDialog(),
    );

    if (result != null) {
      ref.read(tabManagerProvider.notifier).openTab(result.workspaceId, result.workspaceName);
    }
  }
}

/// 单个标签页项目组件
class _WorkspaceTabItem extends StatelessWidget {
  final WorkspaceTab tab;
  final bool isActive;
  final VoidCallback onTap;
  final VoidCallback onClose;
  final VoidCallback onPin;

  const _WorkspaceTabItem({
    required this.tab,
    required this.isActive,
    required this.onTap,
    required this.onClose,
    required this.onPin,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return GestureDetector(
      onTap: onTap,
      onSecondaryTap: () => _showContextMenu(context),
      child: Container(
        constraints: const BoxConstraints(maxWidth: 200, minWidth: 100),
        padding: const EdgeInsets.symmetric(horizontal: 12),
        decoration: BoxDecoration(
          color: isActive
            ? theme.colorScheme.surface
            : theme.colorScheme.surfaceContainerHighest,
          border: Border(
            bottom: BorderSide(
              color: isActive
                ? theme.colorScheme.primary
                : Colors.transparent,
              width: 2,
            ),
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            // 固定图标
            if (tab.isPinned)
              Icon(
                Icons.push_pin,
                size: 14,
                color: theme.colorScheme.primary,
              ),
            const SizedBox(width: 4),
            // 标题
            Flexible(
              child: Text(
                tab.title,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: isActive ? FontWeight.w600 : FontWeight.normal,
                  color: isActive
                    ? theme.colorScheme.onSurface
                    : theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            const SizedBox(width: 4),
            // 关闭按钮
            InkWell(
              onTap: onClose,
              borderRadius: BorderRadius.circular(10),
              child: Padding(
                padding: const EdgeInsets.all(2),
                child: Icon(
                  Icons.close,
                  size: 16,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showContextMenu(BuildContext context) {
    final RenderBox overlay = Overlay.of(context).context.findRenderObject() as RenderBox;
    final RenderBox button = context.findRenderObject() as RenderBox;
    final RelativeRect position = RelativeRect.fromRect(
      Rect.fromPoints(
        button.localToGlobal(Offset.zero, ancestor: overlay),
        button.localToGlobal(button.size.bottomRight(Offset.zero), ancestor: overlay),
      ),
      Offset.zero & overlay.size,
    );

    showMenu<String>(
      context: context,
      position: position,
      items: [
        PopupMenuItem(
          value: 'pin',
          child: Row(
            children: [
              Icon(tab.isPinned ? Icons.push_pin_outlined : Icons.push_pin),
              const SizedBox(width: 8),
              Text(tab.isPinned ? '取消固定' : '固定标签页'),
            ],
          ),
        ),
        const PopupMenuItem(
          value: 'close_others',
          child: Row(
            children: [
              Icon(Icons.tab_unselected),
              SizedBox(width: 8),
              Text('关闭其他标签页'),
            ],
          ),
        ),
        const PopupMenuItem(
          value: 'close_all',
          child: Row(
            children: [
              const Icon(Icons.close),
              SizedBox(width: 8),
              Text('关闭所有标签页'),
            ],
          ),
        ),
      ],
    ).then((value) {
      if (value == 'pin') {
        onPin();
      } else if (value == 'close_others') {
        // TODO: 实现关闭其他标签页
      } else if (value == 'close_all') {
        // TODO: 实现关闭所有标签页
      }
    });
  }
}

/// 添加标签按钮
class _AddTabButton extends StatelessWidget {
  final VoidCallback onPressed;

  const _AddTabButton({required this.onPressed});

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.add, size: 20),
      onPressed: onPressed,
      tooltip: '打开新标签页',
      padding: const EdgeInsets.all(8),
      constraints: const BoxConstraints(minWidth: 36, minHeight: 36),
    );
  }
}
