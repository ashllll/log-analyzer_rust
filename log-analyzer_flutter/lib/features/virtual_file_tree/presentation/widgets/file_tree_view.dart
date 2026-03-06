import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../shared/providers/virtual_file_tree_provider.dart';
import '../../providers/file_tree_ui_provider.dart';
import 'file_tree_node.dart';

/// 文件树视图组件
///
/// 使用 CustomScrollView + SliverList 实现树形结构
/// 支持懒加载子节点和键盘导航
class FileTreeView extends ConsumerStatefulWidget {
  /// 根节点列表
  final List<VirtualTreeNode> nodes;

  /// 节点点击回调
  final void Function(VirtualTreeNode)? onNodeTap;

  /// 构造函数
  const FileTreeView({
    super.key,
    required this.nodes,
    this.onNodeTap,
  });

  @override
  ConsumerState<FileTreeView> createState() => _FileTreeViewState();
}

class _FileTreeViewState extends ConsumerState<FileTreeView> {
  /// 焦点节点路径
  String? _focusedNodePath;

  /// 滚动控制器
  final ScrollController _scrollController = ScrollController();

  /// 焦点控制器
  final FocusNode _focusNode = FocusNode();

  /// 展平的节点列表（用于键盘导航）
  List<_FlatNode> _flattenedNodes = [];

  /// 当前焦点索引
  int _focusedIndex = -1;

  @override
  void initState() {
    super.initState();
    // 首次加载时自动选中第一个节点
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _flattenNodes();
      if (_flattenedNodes.isNotEmpty) {
        setState(() {
          _focusedIndex = 0;
          _focusedNodePath = _flattenedNodes[0].node.nodePath;
        });
      }
    });
  }

  @override
  void didUpdateWidget(FileTreeView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.nodes != widget.nodes) {
      _flattenNodes();
    }
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  /// 将树形结构展平为列表（用于键盘导航）
  void _flattenNodes() {
    _flattenedNodes = [];
    _flattenNodesRecursive(widget.nodes, 0);
  }

  void _flattenNodesRecursive(List<VirtualTreeNode> nodes, int depth) {
    final expandedPaths = ref.read(fileTreeUIProvider).expandedPaths;

    for (final node in nodes) {
      _flattenedNodes.add(_FlatNode(node: node, depth: depth));

      // 如果节点展开且有子节点，递归添加
      if (node.isArchive && expandedPaths.contains(node.nodePath)) {
        if (node.children.isNotEmpty) {
          _flattenNodesRecursive(node.children, depth + 1);
        }
      }
    }
  }

  /// 处理键盘事件
  KeyEventResult _handleKeyEvent(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }

    final expandedPaths = ref.read(fileTreeUIProvider).expandedPaths;

    switch (event.logicalKey) {
      // 上箭头：选中上一个节点
      case LogicalKeyboardKey.arrowUp:
        _moveFocus(-1);
        return KeyEventResult.handled;

      // 下箭头：选中下一个节点
      case LogicalKeyboardKey.arrowDown:
        _moveFocus(1);
        return KeyEventResult.handled;

      // 左箭头：折叠当前目录
      case LogicalKeyboardKey.arrowLeft:
        if (_focusedNodePath != null && expandedPaths.contains(_focusedNodePath)) {
          ref.read(fileTreeUIProvider.notifier).toggleExpand(_focusedNodePath!);
          _flattenNodes();
        }
        return KeyEventResult.handled;

      // 右箭头：展开当前目录
      case LogicalKeyboardKey.arrowRight:
        if (_focusedNodePath != null) {
          final node = _flattenedNodes
              .where((n) => n.node.nodePath == _focusedNodePath)
              .firstOrNull;
          if (node != null && node.node.isArchive && !expandedPaths.contains(_focusedNodePath)) {
            ref.read(fileTreeUIProvider.notifier).toggleExpand(_focusedNodePath!);
            _flattenNodes();
          }
        }
        return KeyEventResult.handled;

      // 回车：打开预览
      case LogicalKeyboardKey.enter:
        if (_focusedNodePath != null) {
          final node = _flattenedNodes
              .where((n) => n.node.nodePath == _focusedNodePath)
              .firstOrNull;
          if (node != null) {
            widget.onNodeTap?.call(node.node);
          }
        }
        return KeyEventResult.handled;

      default:
        return KeyEventResult.ignored;
    }
  }

  /// 移动焦点
  void _moveFocus(int direction) {
    if (_flattenedNodes.isEmpty) return;

    final newIndex = (_focusedIndex + direction).clamp(0, _flattenedNodes.length - 1);
    if (newIndex != _focusedIndex) {
      setState(() {
        _focusedIndex = newIndex;
        _focusedNodePath = _flattenedNodes[newIndex].node.nodePath;
      });

      // 确保选中的节点滚动到可见区域
      _scrollToIndex(newIndex);
    }
  }

  /// 滚动到指定索引
  void _scrollToIndex(int index) {
    // 计算大概的滚动位置（每行 28px + 一些边距）
    final targetOffset = index * 28.0;
    _scrollController.animateTo(
      targetOffset,
      duration: const Duration(milliseconds: 100),
      curve: Curves.easeOut,
    );
  }

  /// 构建树形节点
  Widget _buildNode(BuildContext context, _FlatNode flatNode, int index) {
    final node = flatNode.node;
    final depth = flatNode.depth;
    final expandedPaths = ref.watch(fileTreeUIProvider).expandedPaths;
    final selectedPath = ref.watch(fileTreeUIProvider).selectedPath;

    final isExpanded = node.isArchive && expandedPaths.contains(node.nodePath);
    final isSelected = node.nodePath == selectedPath;

    return FileTreeNode(
      node: node,
      depth: depth,
      isExpanded: isExpanded,
      isSelected: isSelected || node.nodePath == _focusedNodePath,
      onTap: () {
        // 更新选中状态
        ref.read(fileTreeUIProvider.notifier).selectNode(node.nodePath);
        widget.onNodeTap?.call(node);
      },
      onExpand: node.isArchive
          ? () {
              ref.read(fileTreeUIProvider.notifier).toggleExpand(node.nodePath);
              _flattenNodes();
            }
          : null,
    );
  }

  @override
  Widget build(BuildContext context) {
    // 监听展开状态变化，重新计算展平节点
    ref.listen(fileTreeUIProvider, (previous, next) {
      _flattenNodes();
    });

    // 重新计算展平节点
    _flattenNodes();

    if (widget.nodes.isEmpty) {
      return const _EmptyState();
    }

    return Focus(
      focusNode: _focusNode,
      autofocus: true,
      onKeyEvent: _handleKeyEvent,
      child: ListView.builder(
        controller: _scrollController,
        itemCount: _flattenedNodes.length,
        itemBuilder: (context, index) => _buildNode(context, _flattenedNodes[index], index),
      ),
    );
  }
}

/// 展平节点数据结构
class _FlatNode {
  final VirtualTreeNode node;
  final int depth;

  _FlatNode({required this.node, required this.depth});
}

/// 空状态组件
class _EmptyState extends StatelessWidget {
  const _EmptyState();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.folder_open,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            '工作区为空',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
              color: Theme.of(context).colorScheme.outline,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            '导入文件开始分析',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }
}
