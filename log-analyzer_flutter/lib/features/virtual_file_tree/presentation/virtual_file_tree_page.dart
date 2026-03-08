import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

import '../../../shared/providers/app_provider.dart';
import '../../../shared/providers/virtual_file_tree_provider.dart';
import 'widgets/file_tree_sidebar.dart';

/// 虚拟文件树页面
///
/// 提供虚拟文件树浏览功能的入口页面
/// 左侧为可调宽度的文件树侧边栏，右侧为标签页切换
class VirtualFileTreePage extends ConsumerStatefulWidget {
  /// 构造函数
  const VirtualFileTreePage({super.key});

  @override
  ConsumerState<VirtualFileTreePage> createState() =>
      _VirtualFileTreePageState();
}

class _VirtualFileTreePageState extends ConsumerState<VirtualFileTreePage>
    with SingleTickerProviderStateMixin {
  /// 标签页控制器
  late TabController _tabController;

  /// 当前选中的文件哈希（用于预览）
  String? _selectedFileHash;

  /// 文件内容
  String? _fileContent;

  /// 是否正在加载文件内容
  bool _isLoadingContent = false;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  /// 处理节点点击
  Future<void> _handleNodeTap(VirtualTreeNode node) async {
    if (node.isFile) {
      // 读取文件内容
      final workspaceId = ref.read(appStateProvider).activeWorkspaceId;
      if (workspaceId == null) return;

      setState(() {
        _selectedFileHash = node.nodeHash;
        _isLoadingContent = true;
      });

      try {
        final content = await ref
            .read(virtualFileTreeProvider(workspaceId).notifier)
            .readFileByHash(node.nodeHash);

        setState(() {
          _fileContent = content?.content;
          _isLoadingContent = false;
        });
      } catch (e) {
        setState(() {
          _fileContent = null;
          _isLoadingContent = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final workspaceId = ref.watch(appStateProvider).activeWorkspaceId;
    final theme = Theme.of(context);

    // 如果没有选择工作区，显示提示
    if (workspaceId == null) {
      return _buildNoWorkspaceState(theme);
    }

    // 监听文件树数据
    final treeAsync = ref.watch(virtualFileTreeProvider(workspaceId));

    return Row(
      children: [
        // 左侧：文件树侧边栏
        FileTreeSidebar(
          nodes: treeAsync.value ?? [],
          onNodeTap: _handleNodeTap,
        ),
        // 右侧：主内容区（标签页）
        Expanded(
          child: Column(
            children: [
              // 标签栏
              Container(
                color: theme.colorScheme.surfaceContainerLow,
                child: TabBar(
                  controller: _tabController,
                  tabs: const [
                    Tab(text: '日志列表'),
                    Tab(text: '文件树'),
                  ],
                  labelColor: theme.colorScheme.primary,
                  unselectedLabelColor: theme.colorScheme.onSurfaceVariant,
                  indicatorColor: theme.colorScheme.primary,
                ),
              ),
              // 标签内容
              Expanded(
                child: TabBarView(
                  controller: _tabController,
                  children: [
                    // 日志列表（搜索结果）- 这里显示文件预览
                    _buildFilePreview(theme),
                    // 文件树 - 已在侧边栏显示
                    Center(
                      child: Text(
                        '文件树在左侧侧边栏',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.outline,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  /// 构建没有工作区状态
  Widget _buildNoWorkspaceState(ThemeData theme) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(LucideIcons.folderX, size: 64, color: theme.colorScheme.outline),
          const SizedBox(height: 16),
          Text(
            '请先选择工作区',
            style: theme.textTheme.titleMedium?.copyWith(
              color: theme.colorScheme.outline,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            '在工作区管理页面选择或创建工作区',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }

  /// 构建文件预览区
  Widget _buildFilePreview(ThemeData theme) {
    if (_selectedFileHash == null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(LucideIcons.file, size: 48, color: theme.colorScheme.outline),
            const SizedBox(height: 16),
            Text(
              '选择文件预览内容',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      );
    }

    if (_isLoadingContent) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_fileContent == null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(LucideIcons.fileX, size: 48, color: theme.colorScheme.error),
            const SizedBox(height: 16),
            Text(
              '无法加载文件内容',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
          ],
        ),
      );
    }

    return Container(
      padding: const EdgeInsets.all(16),
      child: SelectableText(
        _fileContent!,
        style: theme.textTheme.bodySmall?.copyWith(fontFamily: 'monospace'),
      ),
    );
  }
}
