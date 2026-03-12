import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

import '../../../../shared/providers/virtual_file_tree_provider.dart';
import '../../providers/file_tree_ui_provider.dart';
import 'file_tree_view.dart';

/// 文件树侧边栏组件
///
/// 左侧可调宽度的文件树侧边栏
class FileTreeSidebar extends ConsumerStatefulWidget {
  /// 文件树节点列表
  final List<VirtualTreeNode> nodes;

  /// 节点点击回调
  final void Function(VirtualTreeNode)? onNodeTap;

  /// 构造函数
  const FileTreeSidebar({super.key, required this.nodes, this.onNodeTap});

  @override
  ConsumerState<FileTreeSidebar> createState() => _FileTreeSidebarState();
}

class _FileTreeSidebarState extends ConsumerState<FileTreeSidebar> {
  /// 是否正在拖动
  bool _isDragging = false;

  /// 拖动起始位置
  double _dragStartX = 0;

  /// 拖动起始宽度
  double _startWidth = 0;

  /// 最小宽度
  static const double minWidth = 200.0;

  /// 最大宽度
  static const double maxWidth = 500.0;

  /// 拖动区域宽度
  static const double dragAreaWidth = 4.0;

  /// 处理拖动开始
  void _onDragStart(DragStartDetails details) {
    final currentWidth = ref.read(fileTreeUIProvider).sidebarWidth;
    setState(() {
      _isDragging = true;
      _dragStartX = details.globalPosition.dx;
      _startWidth = currentWidth;
    });
  }

  /// 处理拖动更新
  void _onDragUpdate(DragUpdateDetails details) {
    final delta = details.globalPosition.dx - _dragStartX;
    final newWidth = (_startWidth + delta).clamp(minWidth, maxWidth);
    ref.read(fileTreeUIProvider.notifier).setSidebarWidth(newWidth);
  }

  /// 处理拖动结束
  void _onDragEnd(DragEndDetails details) {
    setState(() {
      _isDragging = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final uiState = ref.watch(fileTreeUIProvider);
    final theme = Theme.of(context);

    // 如果侧边栏折叠，返回折叠按钮
    if (uiState.isSidebarCollapsed) {
      return _buildCollapsedSidebar(theme);
    }

    return Row(
      children: [
        // 文件树主体
        SizedBox(
          width: uiState.sidebarWidth,
          child: Column(
            children: [
              // 标题栏
              _buildHeader(theme),
              // 分割线
              Divider(height: 1, color: theme.colorScheme.outlineVariant),
              // 文件树视图
              Expanded(
                child: FileTreeView(
                  nodes: widget.nodes,
                  onNodeTap: widget.onNodeTap,
                ),
              ),
            ],
          ),
        ),
        // 拖动区域
        GestureDetector(
          onHorizontalDragStart: _onDragStart,
          onHorizontalDragUpdate: _onDragUpdate,
          onHorizontalDragEnd: _onDragEnd,
          child: MouseRegion(
            cursor: SystemMouseCursors.resizeColumn,
            child: Container(
              width: dragAreaWidth,
              color: _isDragging
                  ? theme.colorScheme.primary.withOpacity(0.3)
                  : Colors.transparent,
              child: Center(
                child: Container(
                  width: 1,
                  color: _isDragging
                      ? theme.colorScheme.primary
                      : theme.colorScheme.outlineVariant,
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }

  /// 构建折叠侧边栏
  Widget _buildCollapsedSidebar(ThemeData theme) {
    return Container(
      width: 48,
      color: theme.colorScheme.surfaceContainerLow,
      child: Column(
        children: [
          const SizedBox(height: 8),
          // 展开按钮
          IconButton(
            icon: const Icon(LucideIcons.panelRight),
            tooltip: '展开文件树',
            onPressed: () {
              ref.read(fileTreeUIProvider.notifier).expandSidebar();
            },
          ),
          const Divider(),
          // 文件树图标
          IconButton(
            icon: const Icon(LucideIcons.files),
            tooltip: '文件树',
            onPressed: () {
              ref.read(fileTreeUIProvider.notifier).expandSidebar();
            },
          ),
        ],
      ),
    );
  }

  /// 构建标题栏
  Widget _buildHeader(ThemeData theme) {
    return Container(
      height: 40,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      color: theme.colorScheme.surfaceContainerLow,
      child: Row(
        children: [
          Icon(
            LucideIcons.files,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              '文件树',
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          // 折叠按钮
          IconButton(
            icon: const Icon(LucideIcons.panelLeftClose, size: 16),
            tooltip: '折叠侧边栏',
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
            onPressed: () {
              ref.read(fileTreeUIProvider.notifier).collapseSidebar();
            },
          ),
        ],
      ),
    );
  }
}
