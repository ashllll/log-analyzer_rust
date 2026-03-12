import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../core/constants/app_constants.dart';
import '../../../../shared/providers/app_provider.dart';
import '../../../../shared/providers/workspace_provider.dart';
import '../../providers/settings_provider.dart';

/// 工作区设置 Tab
///
/// 显示最近工作区列表，提供清空历史功能
class WorkspaceSettingsTab extends ConsumerWidget {
  const WorkspaceSettingsTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settingsState = ref.watch(settingsProvider);
    final workspaces = ref.watch(workspaceStateProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '工作区设置',
            style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 32),

          // 最近工作区
          Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.folder_outlined,
                        color: Theme.of(context).colorScheme.primary,
                      ),
                      const SizedBox(width: 12),
                      const Text(
                        '最近工作区',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const Spacer(),
                      if (settingsState.recentWorkspaces.isNotEmpty)
                        TextButton.icon(
                          onPressed: () =>
                              _showClearHistoryDialog(context, ref),
                          icon: const Icon(Icons.delete_outline, size: 18),
                          label: const Text('清空历史'),
                        ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    '显示最近打开的 5 个工作区',
                    style: TextStyle(color: Colors.grey),
                  ),
                  const SizedBox(height: 20),

                  if (settingsState.recentWorkspaces.isEmpty)
                    _buildEmptyState()
                  else
                    _buildRecentWorkspacesList(
                      context,
                      ref,
                      settingsState.recentWorkspaces,
                      workspaces,
                    ),
                ],
              ),
            ),
          ),

          const SizedBox(height: 16),

          // 自动恢复
          Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.refresh,
                        color: Theme.of(context).colorScheme.primary,
                      ),
                      const SizedBox(width: 12),
                      const Text(
                        '启动恢复',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    '应用启动时自动加载上次的工作区',
                    style: TextStyle(color: Colors.grey),
                  ),
                  const SizedBox(height: 16),

                  if (settingsState.lastWorkspaceId != null) ...[
                    Container(
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.primaryContainer,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        children: [
                          Icon(
                            Icons.check_circle,
                            color: Theme.of(context).colorScheme.primary,
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Text(
                              '已启用自动恢复',
                              style: TextStyle(
                                color: Theme.of(
                                  context,
                                ).colorScheme.onPrimaryContainer,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ] else ...[
                    Container(
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: Colors.grey[200],
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Row(
                        children: [
                          Icon(Icons.info_outline, color: Colors.grey),
                          SizedBox(width: 12),
                          Expanded(
                            child: Text(
                              '未设置自动恢复',
                              style: TextStyle(color: Colors.grey),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 32),
      child: const Center(
        child: Column(
          children: [
            Icon(Icons.folder_off_outlined, size: 48, color: Colors.grey),
            SizedBox(height: 16),
            Text('暂无最近工作区', style: TextStyle(color: Colors.grey, fontSize: 16)),
            SizedBox(height: 8),
            Text(
              '打开工作区后会显示在这里',
              style: TextStyle(color: Colors.grey, fontSize: 14),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildRecentWorkspacesList(
    BuildContext context,
    WidgetRef ref,
    List<String> recentWorkspaceIds,
    List workspaces,
  ) {
    return ListView.separated(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: recentWorkspaceIds.length,
      separatorBuilder: (context, index) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final workspaceId = recentWorkspaceIds[index];
        // 查找对应的工作区
        dynamic workspace;
        try {
          workspace = workspaces.firstWhere((w) => w.id == workspaceId);
        } catch (e) {
          workspace = null;
        }

        return ListTile(
          leading: const Icon(Icons.folder),
          title: Text(workspace?.name ?? '未知工作区'),
          subtitle: Text(workspace?.path ?? workspaceId),
          trailing: IconButton(
            icon: const Icon(Icons.close, size: 18),
            onPressed: () {
              removeRecentWorkspace(ref, workspaceId);
              ref
                  .read(appStateProvider.notifier)
                  .addToast(ToastType.info, '已从最近列表中移除');
            },
            tooltip: '移除',
          ),
        );
      },
    );
  }

  void _showClearHistoryDialog(BuildContext context, WidgetRef ref) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('清空历史记录'),
        content: const Text('确定要清空所有最近工作区记录吗？'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('取消'),
          ),
          TextButton(
            onPressed: () {
              clearRecentWorkspaces(ref);
              ref
                  .read(appStateProvider.notifier)
                  .addToast(ToastType.success, '已清空历史记录');
              Navigator.of(context).pop();
            },
            child: const Text('确定'),
          ),
        ],
      ),
    );
  }
}
