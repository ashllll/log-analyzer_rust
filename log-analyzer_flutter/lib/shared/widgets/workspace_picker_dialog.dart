import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/common.dart';
import '../providers/workspace_provider.dart';

/// 工作区选择结果
class WorkspacePickerResult {
  final String workspaceId;
  final String workspaceName;

  WorkspacePickerResult({
    required this.workspaceId,
    required this.workspaceName,
  });
}

/// 工作区选择对话框
class WorkspacePickerDialog extends ConsumerWidget {
  const WorkspacePickerDialog({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final workspaces = ref.watch(workspaceStateProvider);
    final theme = Theme.of(context);

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.tab, color: theme.colorScheme.primary),
          const SizedBox(width: 8),
          const Text('打开工作区'),
        ],
      ),
      content: SizedBox(
        width: 400,
        height: 300,
        child: workspaces.isEmpty
            ? _buildEmptyState()
            : ListView.builder(
                itemCount: workspaces.length,
                itemBuilder: (context, index) {
                  final workspace = workspaces[index];
                  return _WorkspaceListItem(
                    workspace: workspace,
                    onTap: () {
                      Navigator.of(context).pop(
                        WorkspacePickerResult(
                          workspaceId: workspace.id,
                          workspaceName: workspace.name,
                        ),
                      );
                    },
                  );
                },
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
      ],
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.folder_off,
            size: 64,
            color: Colors.grey[400],
          ),
          const SizedBox(height: 16),
          Text(
            '暂无工作区',
            style: TextStyle(
              fontSize: 16,
              color: Colors.grey[600],
            ),
          ),
          const SizedBox(height: 8),
          Text(
            '请先创建一个工作区',
            style: TextStyle(
              fontSize: 14,
              color: Colors.grey[500],
            ),
          ),
        ],
      ),
    );
  }
}

/// 工作区列表项组件
class _WorkspaceListItem extends StatelessWidget {
  final Workspace workspace;
  final VoidCallback onTap;

  const _WorkspaceListItem({
    required this.workspace,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: ListTile(
        leading: Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: theme.colorScheme.primaryContainer,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Icon(
            Icons.folder,
            color: theme.colorScheme.onPrimaryContainer,
          ),
        ),
        title: Text(
          workspace.name,
          style: const TextStyle(fontWeight: FontWeight.w500),
        ),
        subtitle: Text(
          workspace.path,
          overflow: TextOverflow.ellipsis,
          maxLines: 1,
          style: TextStyle(
            fontSize: 12,
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        trailing: _buildStatusIndicator(workspace.status, theme),
        onTap: onTap,
      ),
    );
  }

  Widget _buildStatusIndicator(WorkspaceStatusData status, ThemeData theme) {
    IconData icon;
    Color color;

    switch (status.value) {
      case 'ready':
        icon = Icons.check_circle;
        color = Colors.green;
        break;
      case 'indexing':
        icon = Icons.sync;
        color = Colors.orange;
        break;
      case 'error':
        icon = Icons.error;
        color = Colors.red;
        break;
      default:
        icon = Icons.hourglass_empty;
        color = Colors.grey;
    }

    return Icon(icon, size: 20, color: color);
  }
}
