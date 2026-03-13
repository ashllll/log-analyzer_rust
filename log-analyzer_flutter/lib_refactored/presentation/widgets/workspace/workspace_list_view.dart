/// 工作区列表视图
/// 
/// 纯展示组件，不包含业务逻辑

import 'package:flutter/material.dart';

import '../../../domain/entities/workspace.dart';

/// 工作区列表视图
class WorkspaceListView extends StatelessWidget {
  final List<Workspace> workspaces;
  final Function(Workspace) onWorkspaceSelected;
  final Function(Workspace) onWorkspaceDeleted;
  final VoidCallback onRefresh;

  const WorkspaceListView({
    super.key,
    required this.workspaces,
    required this.onWorkspaceSelected,
    required this.onWorkspaceDeleted,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    if (workspaces.isEmpty) {
      return _EmptyView(onRefresh: onRefresh);
    }

    return RefreshIndicator(
      onRefresh: () async => onRefresh(),
      child: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: workspaces.length,
        itemBuilder: (context, index) => WorkspaceCard(
          workspace: workspaces[index],
          onTap: () => onWorkspaceSelected(workspaces[index]),
          onDelete: () => onWorkspaceDeleted(workspaces[index]),
        ),
      ),
    );
  }
}

/// 工作区卡片
class WorkspaceCard extends StatelessWidget {
  final Workspace workspace;
  final VoidCallback onTap;
  final VoidCallback onDelete;

  const WorkspaceCard({
    super.key,
    required this.workspace,
    required this.onTap,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 标题行
              Row(
                children: [
                  // 状态指示器
                  _StatusIndicator(status: workspace.status),
                  const SizedBox(width: 12),
                  
                  // 名称
                  Expanded(
                    child: Text(
                      workspace.name,
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  
                  // 菜单
                  PopupMenuButton<String>(
                    onSelected: (value) {
                      if (value == 'delete') onDelete();
                    },
                    itemBuilder: (context) => [
                      const PopupMenuItem(
                        value: 'delete',
                        child: Row(
                          children: [
                            Icon(Icons.delete, size: 18),
                            SizedBox(width: 8),
                            Text('删除'),
                          ],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              
              const SizedBox(height: 8),
              
              // 路径
              Text(
                workspace.path,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              
              const SizedBox(height: 12),
              
              // 统计信息
              Row(
                children: [
                  _StatChip(
                    icon: Icons.insert_drive_file,
                    value: workspace.formattedFileCount,
                    label: '文件',
                  ),
                  const SizedBox(width: 16),
                  _StatChip(
                    icon: Icons.format_align_left,
                    value: workspace.formattedLogLines,
                    label: '日志',
                  ),
                  const SizedBox(width: 16),
                  _StatChip(
                    icon: Icons.storage,
                    value: workspace.formattedStorageSize,
                    label: '存储',
                  ),
                ],
              ),
              
              // 进度条（如果正在处理）
              if (workspace.isBusy) ...[
                const SizedBox(height: 12),
                const LinearProgressIndicator(),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// 状态指示器
class _StatusIndicator extends StatelessWidget {
  final WorkspaceStatus status;

  const _StatusIndicator({required this.status});

  @override
  Widget build(BuildContext context) {
    Color color;
    IconData icon;

    switch (status) {
      case WorkspaceStatus.ready:
        color = Colors.green;
        icon = Icons.check_circle;
      case WorkspaceStatus.scanning:
      case WorkspaceStatus.indexing:
        color = Colors.orange;
        icon = Icons.sync;
      case WorkspaceStatus.error:
        color = Colors.red;
        icon = Icons.error;
      case WorkspaceStatus.uninitialized:
        color = Colors.grey;
        icon = Icons.circle_outlined;
    }

    return Icon(icon, color: color, size: 20);
  }
}

/// 统计芯片
class _StatChip extends StatelessWidget {
  final IconData icon;
  final String value;
  final String label;

  const _StatChip({
    required this.icon,
    required this.value,
    required this.label,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 16, color: theme.colorScheme.onSurfaceVariant),
        const SizedBox(width: 4),
        Text(
          '$value $label',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}

/// 空视图
class _EmptyView extends StatelessWidget {
  final VoidCallback onRefresh;

  const _EmptyView({required this.onRefresh});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.folder_open,
            size: 64,
            color: theme.colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            '没有工作区',
            style: theme.textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          Text(
            '点击右下角按钮创建第一个工作区',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 24),
          OutlinedButton.icon(
            onPressed: onRefresh,
            icon: const Icon(Icons.refresh),
            label: const Text('刷新'),
          ),
        ],
      ),
    );
  }
}
