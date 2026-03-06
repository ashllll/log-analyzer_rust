import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

import '../../../../shared/providers/virtual_file_tree_provider.dart';
import 'empty_state.dart';
import 'loading_skeleton.dart';

/// 文件预览面板组件
///
/// 显示选中文件的内容，支持加载状态、错误状态和内容展示
class FilePreviewPanel extends ConsumerStatefulWidget {
  /// 选中的文件节点
  final VirtualTreeNode? selectedFile;

  /// 工作区 ID
  final String workspaceId;

  /// 构造函数
  const FilePreviewPanel({
    super.key,
    this.selectedFile,
    required this.workspaceId,
  });

  @override
  ConsumerState<FilePreviewPanel> createState() => _FilePreviewPanelState();
}

class _FilePreviewPanelState extends ConsumerState<FilePreviewPanel> {
  /// 文件内容
  String? _content;

  /// 是否正在加载
  bool _isLoading = false;

  /// 错误信息
  String? _error;

  @override
  void didUpdateWidget(covariant FilePreviewPanel oldWidget) {
    super.didUpdateWidget(oldWidget);

    // 当选中的文件变化时，重新加载内容
    if (widget.selectedFile != oldWidget.selectedFile) {
      _loadContent();
    }
  }

  @override
  void initState() {
    super.initState();
    // 初始加载
    if (widget.selectedFile != null) {
      _loadContent();
    }
  }

  /// 加载文件内容
  Future<void> _loadContent() async {
    if (widget.selectedFile == null) {
      setState(() {
        _content = null;
        _error = null;
        _isLoading = false;
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      final result = await ref
          .read(virtualFileTreeProvider(widget.workspaceId).notifier)
          .readFileByHash(widget.selectedFile!.nodeHash);

      setState(() {
        _content = result?.content;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _content = null;
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  /// 重试加载
  void _retry() {
    _loadContent();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    // 没有选中文件
    if (widget.selectedFile == null) {
      return const FilePreviewEmptyState();
    }

    // 加载中
    if (_isLoading) {
      return const FilePreviewLoadingSkeleton();
    }

    // 加载错误
    if (_error != null) {
      return _buildErrorView(theme);
    }

    // 内容为空
    if (_content == null || _content!.isEmpty) {
      return _buildEmptyContentView(theme);
    }

    // 显示文件内容
    return _buildContentView(theme);
  }

  /// 构建错误视图
  Widget _buildErrorView(ThemeData theme) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              LucideIcons.fileX,
              size: 48,
              color: theme.colorScheme.error,
            ),
            const SizedBox(height: 16),
            Text(
              '无法加载文件内容',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              _error ?? '未知错误',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: _retry,
              icon: const Icon(LucideIcons.refreshCw),
              label: const Text('重试'),
            ),
          ],
        ),
      ),
    );
  }

  /// 构建空内容视图
  Widget _buildEmptyContentView(ThemeData theme) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            LucideIcons.fileX,
            size: 48,
            color: theme.colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            '文件内容为空',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建内容视图
  Widget _buildContentView(ThemeData theme) {
    return Container(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 文件名标题
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Row(
              children: [
                Icon(
                  LucideIcons.fileText,
                  size: 16,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    widget.selectedFile!.nodeName,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.onSurface,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          // 文件内容
          Expanded(
            child: Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(4),
              ),
              child: SelectableText(
                _content!,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  fontSize: 13,
                  height: 1.5,
                  color: theme.colorScheme.onSurface,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
