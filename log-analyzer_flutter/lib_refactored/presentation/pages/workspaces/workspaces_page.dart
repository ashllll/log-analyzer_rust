/// 工作区列表页面
/// 
/// 精简架构示例，遵循单一职责原则
/// - 只负责页面布局和导航
/// - 业务逻辑交给 AsyncNotifier
/// - UI 组件拆分到 widgets 目录

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/errors/error_handler.dart';
import '../../../domain/entities/workspace.dart';
import '../../providers/workspace_provider.dart';
import '../../widgets/common/async_value_widget.dart';
import '../../widgets/common/error_view.dart';
import '../../widgets/workspace/workspace_list_view.dart';
import '../../widgets/workspace/create_workspace_dialog.dart';

/// 工作区页面
class WorkspacesPage extends ConsumerWidget {
  const WorkspacesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // 监听工作区列表状态
    final workspacesAsync = ref.watch(workspaceListProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('工作区'),
        actions: [
          // 刷新按钮
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => ref.read(workspaceListProvider.notifier).refresh(),
          ),
        ],
      ),
      body: workspacesAsync.when(
        // 加载中显示骨架屏
        loading: () => const WorkspaceListSkeleton(),
        
        // 错误时显示错误界面
        error: (error, stack) => ErrorView(
          error: error,
          onRetry: () => ref.read(workspaceListProvider.notifier).refresh(),
        ),
        
        // 数据就绪时显示列表
        data: (workspaces) => WorkspaceListView(
          workspaces: workspaces,
          onWorkspaceSelected: (workspace) => _onWorkspaceSelected(
            context,
            ref,
            workspace,
          ),
          onWorkspaceDeleted: (workspace) => _onWorkspaceDeleted(
            context,
            ref,
            workspace,
          ),
          onRefresh: () => ref.read(workspaceListProvider.notifier).refresh(),
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => _showCreateDialog(context, ref),
        icon: const Icon(Icons.add),
        label: const Text('新建工作区'),
      ),
    );
  }

  void _onWorkspaceSelected(
    BuildContext context,
    WidgetRef ref,
    Workspace workspace,
  ) {
    // 更新选中的工作区
    ref.read(selectedWorkspaceProvider.notifier).select(workspace);
    
    // 导航到工作区详情
    // context.go('/workspaces/${workspace.id}');
  }

  Future<void> _onWorkspaceDeleted(
    BuildContext context,
    WidgetRef ref,
    Workspace workspace,
  ) async {
    // 显示确认对话框
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('确认删除'),
        content: Text('确定要删除工作区 "${workspace.name}" 吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('删除'),
          ),
        ],
      ),
    );

    if (confirmed == true) {
      try {
        await ref.read(workspaceListProvider.notifier).deleteWorkspace(
          workspace.id,
        );
        
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('工作区已删除')),
          );
        }
      } catch (e) {
        if (context.mounted) {
          ErrorHandler.report(e, context: 'DeleteWorkspace');
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('删除失败: $e')),
          );
        }
      }
    }
  }

  Future<void> _showCreateDialog(BuildContext context, WidgetRef ref) async {
    final result = await showDialog<CreateWorkspaceParams>(
      context: context,
      builder: (context) => const CreateWorkspaceDialog(),
    );

    if (result != null) {
      try {
        await ref.read(workspaceListProvider.notifier).createWorkspace(result);
        
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('工作区创建成功')),
          );
        }
      } catch (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('创建失败: $e')),
          );
        }
      }
    }
  }
}

/// 骨架屏
class WorkspaceListSkeleton extends StatelessWidget {
  const WorkspaceListSkeleton({super.key});

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: 5,
      itemBuilder: (context, index) => const Card(
        child: SizedBox(height: 80),
      ),
    );
  }
}
