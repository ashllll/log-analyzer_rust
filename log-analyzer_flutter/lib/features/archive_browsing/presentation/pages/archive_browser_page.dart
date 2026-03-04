import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../models/archive_node.dart';
import '../../providers/archive_browser_provider.dart';
import '../../../../shared/services/api_service.dart';
import '../widgets/archive_tree_view.dart';
import '../widgets/archive_preview_panel.dart';
import '../widgets/archive_search_bar.dart';

/// 压缩包浏览页面（Split Pane 布局）
class ArchiveBrowserPage extends ConsumerStatefulWidget {
  final String archivePath;

  const ArchiveBrowserPage({
    super.key,
    required this.archivePath,
  });

  @override
  ConsumerState<ArchiveBrowserPage> createState() => _ArchiveBrowserPageState();
}

class _ArchiveBrowserPageState extends ConsumerState<ArchiveBrowserPage> {
  ArchiveNode? _selectedNode;
  String? _previewContent;
  bool _isLoadingPreview = false;
  String? _previewError;
  bool _isTruncated = false;

  @override
  void initState() {
    super.initState();
    // 加载文件列表
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(archivePathProvider.notifier).setPath(widget.archivePath);
    });
  }

  Future<void> _loadPreview(ArchiveNode node) async {
    if (node.isDirectory) return;

    setState(() {
      _selectedNode = node;
      _isLoadingPreview = true;
      _previewError = null;
      _previewContent = null;
    });

    try {
      final api = ApiService();
      final result = await api.readArchiveFile(widget.archivePath, node.path);

      setState(() {
        _previewContent = result.content;
        _isTruncated = result.truncated;
        _isLoadingPreview = false;
      });
    } catch (e) {
      setState(() {
        _previewError = e.toString();
        _isLoadingPreview = false;
      });
    }
  }

  void _toggleExpand(ArchiveNode node) {
    setState(() {
      node.isExpanded = !node.isExpanded;
    });
    // 触发重建
    ref.invalidate(archiveTreeProvider(widget.archivePath));
  }

  @override
  Widget build(BuildContext context) {
    // 监听搜索关键词变化
    final searchKeyword = ref.watch(searchKeywordProvider);

    // 加载文件树
    final treeAsync = ref.watch(archiveTreeProvider(widget.archivePath));

    return Scaffold(
      appBar: AppBar(
        title: Text(
          '浏览: ${Uri.decodeComponent(widget.archivePath.split('/').last)}',
        ),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.pop(),
        ),
      ),
      body: treeAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, stack) => Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(Icons.error_outline, size: 48, color: Colors.red.shade300),
              const SizedBox(height: 16),
              Text('加载失败: $error'),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: () =>
                    ref.refresh(archiveTreeProvider(widget.archivePath)),
                child: const Text('重试'),
              ),
            ],
          ),
        ),
        data: (nodes) {
          // 根据搜索关键词过滤
          final filteredNodes = _filterNodes(nodes, searchKeyword);

          return Column(
            children: [
              // 搜索栏
              const ArchiveSearchBar(),
              // Split Pane 布局
              Expanded(
                child: Row(
                  children: [
                    // 左侧：树形视图 (30% 宽度)
                    SizedBox(
                      width: MediaQuery.of(context).size.width * 0.3,
                      child: ArchiveTreeView(
                        nodes: filteredNodes,
                        selectedPath: _selectedNode?.path,
                        onSelect: _loadPreview,
                        onToggleExpand: _toggleExpand,
                      ),
                    ),
                    // 分隔线
                    const VerticalDivider(width: 1),
                    // 右侧：预览面板 (70% 宽度)
                    Expanded(
                      child: ArchivePreviewPanel(
                        content: _previewContent,
                        searchKeyword: searchKeyword,
                        isLoading: _isLoadingPreview,
                        error: _previewError,
                        truncated: _isTruncated,
                        selectedFileName: _selectedNode?.name,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  /// 根据关键词过滤节点
  List<ArchiveNode> _filterNodes(List<ArchiveNode> nodes, String keyword) {
    if (keyword.isEmpty) return nodes;

    final result = <ArchiveNode>[];
    final lowerKeyword = keyword.toLowerCase();

    for (final node in nodes) {
      if (node.name.toLowerCase().contains(lowerKeyword) ||
          node.path.toLowerCase().contains(lowerKeyword)) {
        result.add(node);
      } else if (node.isDirectory && node.children.isNotEmpty) {
        // 递归过滤子目录
        final filteredChildren = _filterNodes(node.children, keyword);
        if (filteredChildren.isNotEmpty) {
          result.add(ArchiveNode(
            name: node.name,
            path: node.path,
            isDirectory: node.isDirectory,
            size: node.size,
            compressedSize: node.compressedSize,
            children: filteredChildren,
            isExpanded: true,
          ));
        }
      }
    }
    return result;
  }
}
