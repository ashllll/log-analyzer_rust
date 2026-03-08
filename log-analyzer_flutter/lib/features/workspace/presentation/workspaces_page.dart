import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';
import 'dart:async';
import 'dart:io';

import '../../../shared/models/common.dart';
import '../../../shared/providers/workspace_provider.dart';
import '../../../shared/providers/app_provider.dart';
import '../../../shared/providers/import_progress_provider.dart';
import '../../../shared/services/api_service.dart';
import '../../../shared/widgets/drop_zone.dart';
import '../../../shared/widgets/archive_import_dialog.dart';
import '../../../shared/widgets/import_progress_dialog.dart';
import '../../../shared/widgets/skeleton_loading.dart';
import '../../../shared/widgets/empty_state_widget.dart';
import '../../../core/theme/app_theme.dart';

/// 工作区管理页面
///
/// 对应 React 版本的 WorkspacesPage.tsx
/// 功能：
/// - 工作区列表展示
/// - 导入文件夹/文件
/// - 工作区删除
/// - 刷新工作区
/// - 文件监听开关
class WorkspacesPage extends ConsumerStatefulWidget {
  const WorkspacesPage({super.key});

  @override
  ConsumerState<WorkspacesPage> createState() => _WorkspacesPageState();
}

class _WorkspacesPageState extends ConsumerState<WorkspacesPage> {
  final FocusNode _listFocusNode = FocusNode();
  int _selectedIndex = -1;
  Timer? _statusPollingTimer;

  @override
  void initState() {
    super.initState();
    // 启动状态轮询
    _startStatusPolling();
  }

  @override
  void dispose() {
    _listFocusNode.dispose();
    _statusPollingTimer?.cancel();
    super.dispose();
  }

  /// 启动状态轮询
  void _startStatusPolling() {
    _statusPollingTimer?.cancel();
    _statusPollingTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => _pollWorkspaceStatus(),
    );
  }

  /// 轮询工作区状态
  Future<void> _pollWorkspaceStatus() async {
    final workspaces = ref.read(workspaceStateProvider);
    if (workspaces.isEmpty) return;

    // 检查是否有正在处理的工作区
    final hasProcessing = workspaces.any(
      (w) =>
          w.status.value == 'SCANNING' ||
          w.status.value == 'PROCESSING' ||
          w.status.value == 'INDEXING',
    );

    if (!hasProcessing) return;

    // 刷新工作区列表以获取最新状态
    try {
      await ref.read(workspaceStateProvider.notifier).loadWorkspaces();
    } catch (e) {
      debugPrint('Status polling error: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final workspaces = ref.watch(workspaceStateProvider);
    final activeWorkspaceId = ref.watch(appStateProvider).activeWorkspaceId;
    final importState = ref.watch(importProgressProvider);

    return Scaffold(
      appBar: _buildAppBar(context),
      body: DropZoneWidget(
        onFilesDropped: (paths) => _handleFilesDropped(context, paths),
        onArchiveDropped: (archivePath) =>
            _handleArchiveDropped(context, archivePath),
        foldersOnly: true,
        archiveEnabled: true,
        child: workspaces.isEmpty
            ? _buildEmptyState(context)
            : _buildWorkspaceList(workspaces, activeWorkspaceId),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _showAddWorkspaceDialog(context),
        backgroundColor: AppColors.primary,
        child: const Icon(Icons.add),
      ),
    );
  }

  /// 处理拖放的压缩包
  Future<void> _handleArchiveDropped(
    BuildContext context,
    String archivePath,
  ) async {
    if (!context.mounted) return;

    // 获取工作区列表
    final workspaces = ref.read(workspaceStateProvider);

    if (workspaces.isEmpty) {
      // 没有工作区，提示创建
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('请先创建工作区'),
          backgroundColor: AppColors.warning,
        ),
      );
    } else {
      // 有工作区，让用户选择
      _showArchiveImportDestinationDialog(
        context,
        archivePath,
        workspaces.map((w) => w.id).toList(),
      );
    }
  }

  /// 显示压缩包导入目标选择对话框
  void _showArchiveImportDestinationDialog(
    BuildContext context,
    String archivePath,
    List<String> workspaceIds,
  ) {
    final workspaces = ref.read(workspaceStateProvider);

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        backgroundColor: AppColors.bgCard,
        title: const Text('导入压缩包到工作区'),
        content: SizedBox(
          width: 300,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('文件: ${archivePath.split(RegExp(r'[/\\]')).last}'),
              const SizedBox(height: 16),
              const Text(
                '选择目标工作区:',
                style: TextStyle(fontWeight: FontWeight.w500),
              ),
              const SizedBox(height: 8),
              ...workspaces.map(
                (workspace) => ListTile(
                  leading: const Icon(Icons.folder),
                  title: Text(workspace.name),
                  onTap: () {
                    Navigator.pop(dialogContext);
                    _startArchiveImport(context, workspace.id, archivePath);
                  },
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('取消'),
          ),
        ],
      ),
    );
  }

  /// 开始压缩包导入
  Future<void> _startArchiveImport(
    BuildContext context,
    String workspaceId,
    String archivePath,
  ) async {
    try {
      // 显示压缩包预览对话框
      if (!context.mounted) return;

      final taskId = await showArchiveImportDialog(
        context,
        archivePath: archivePath,
        workspaceId: workspaceId,
      );

      if (taskId != null && context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('压缩包导入已开始'),
            backgroundColor: AppColors.success,
          ),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('导入失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }

  /// 处理拖放的文件
  Future<void> _handleFilesDropped(
    BuildContext context,
    List<String> paths,
  ) async {
    if (paths.isEmpty) return;

    // 提示用户选择工作区或创建新工作区
    if (!context.mounted) return;

    // 获取工作区列表
    final workspaces = ref.read(workspaceStateProvider);

    if (workspaces.isEmpty) {
      // 没有工作区，提示创建
      _showAddWorkspaceDialog(context, initialPath: paths.first);
    } else {
      // 有工作区，让用户选择
      _showImportDestinationDialog(context, paths);
    }
  }

  /// 显示导入目标选择对话框
  void _showImportDestinationDialog(BuildContext context, List<String> paths) {
    final workspaces = ref.read(workspaceStateProvider);

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        backgroundColor: AppColors.bgCard,
        title: const Text('导入到工作区'),
        content: SizedBox(
          width: 300,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('已选择 ${paths.length} 个文件夹'),
              const SizedBox(height: 16),
              const Text(
                '选择目标工作区:',
                style: TextStyle(fontWeight: FontWeight.w500),
              ),
              const SizedBox(height: 8),
              ...workspaces.map(
                (workspace) => ListTile(
                  leading: const Icon(Icons.folder),
                  title: Text(workspace.name),
                  subtitle: Text(workspace.path),
                  onTap: () {
                    Navigator.pop(dialogContext);
                    _startImport(context, workspace.id, paths);
                  },
                ),
              ),
              const Divider(),
              ListTile(
                leading: const Icon(Icons.add),
                title: const Text('创建新工作区'),
                onTap: () {
                  Navigator.pop(dialogContext);
                  if (paths.isNotEmpty) {
                    _showAddWorkspaceDialog(context, initialPath: paths.first);
                  }
                },
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('取消'),
          ),
        ],
      ),
    );
  }

  /// 开始导入
  Future<void> _startImport(
    BuildContext context,
    String workspaceId,
    List<String> paths,
  ) async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 显示进度对话框
      if (context.mounted) {
        showDialog(
          context: context,
          barrierDismissible: false,
          builder: (context) => const ImportProgressDialog(),
        );
      }

      // 逐个导入文件夹
      for (final path in paths) {
        // 开始导入
        final taskId = await apiService.importFolder(
          path: path,
          workspaceId: workspaceId,
        );

        // 更新进度
        ref
            .read(importProgressProvider.notifier)
            .startImport(taskId: taskId, totalFiles: 1);

        // 模拟进度更新 (实际应该监听任务进度)
        await Future.delayed(const Duration(milliseconds: 500));
        ref
            .read(importProgressProvider.notifier)
            .updateProgress(
              totalFiles: 1,
              processedFiles: 1,
              currentFile: path,
            );
      }

      // 完成导入
      ref.read(importProgressProvider.notifier).completeImport();

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('导入完成'),
            backgroundColor: AppColors.success,
          ),
        );
      }

      // 刷新工作区
      await ref.read(workspaceStateProvider.notifier).loadWorkspaces();
    } catch (e) {
      ref.read(importProgressProvider.notifier).failImport(e.toString());

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('导入失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }

  /// 构建 AppBar
  PreferredSizeWidget _buildAppBar(BuildContext context) {
    return AppBar(
      backgroundColor: AppColors.bgMain,
      elevation: 0,
      title: const Text(
        '工作区',
        style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
      ),
      actions: [
        // 导入按钮 (支持文件夹和压缩包)
        PopupMenuButton<String>(
          icon: const Icon(Icons.file_upload_outlined),
          tooltip: '导入',
          onSelected: (value) {
            if (value == 'folder') {
              _importFolder(context);
            } else if (value == 'archive') {
              _importArchive(context);
            }
          },
          itemBuilder: (context) => [
            const PopupMenuItem(
              value: 'folder',
              child: Row(
                children: [
                  Icon(Icons.folder, size: 20),
                  SizedBox(width: 8),
                  Text('导入文件夹'),
                ],
              ),
            ),
            const PopupMenuItem(
              value: 'archive',
              child: Row(
                children: [
                  Icon(Icons.archive, size: 20),
                  SizedBox(width: 8),
                  Text('导入压缩包'),
                ],
              ),
            ),
          ],
        ),
        IconButton(
          icon: const Icon(Icons.refresh),
          tooltip: '刷新所有工作区',
          onPressed: () => _refreshAllWorkspaces(context),
        ),
      ],
    );
  }

  /// 导入文件夹 (通过文件选择器)
  Future<void> _importFolder(BuildContext context) async {
    try {
      final result = await FilePicker.platform.getDirectoryPath(
        dialogTitle: '选择要导入的文件夹',
        initialDirectory: Platform.isWindows
            ? 'C:\\'
            : Platform.isMacOS
            ? '/Users'
            : '/home',
      );

      if (result != null) {
        // 获取工作区列表
        final workspaces = ref.read(workspaceStateProvider);

        if (workspaces.isEmpty) {
          // 没有工作区，提示创建
          _showAddWorkspaceDialog(context, initialPath: result);
        } else {
          // 有工作区，让用户选择
          _showImportDestinationDialog(context, [result]);
        }
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('选择文件夹失败: $e'),
            backgroundColor: AppColors.error,
          ),
        );
      }
    }
  }

  /// 导入压缩包 (通过文件选择器)
  Future<void> _importArchive(BuildContext context) async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['zip', 'tar', 'gz', 'rar', '7z'],
        allowMultiple: false,
        dialogTitle: '选择要导入的压缩包',
      );

      if (result != null && result.files.isNotEmpty) {
        final filePath = result.files.first.path;
        if (filePath == null) return;

        // 获取工作区列表
        final workspaces = ref.read(workspaceStateProvider);

        if (workspaces.isEmpty) {
          // 没有工作区，提示创建
          if (context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('请先创建工作区'),
                backgroundColor: AppColors.warning,
              ),
            );
          }
        } else {
          // 有工作区，让用户选择
          _showArchiveImportDestinationDialog(
            context,
            filePath,
            workspaces.map((w) => w.id).toList(),
          );
        }
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('选择压缩包失败: $e'),
            backgroundColor: AppColors.error,
          ),
        );
      }
    }
  }

  /// 构建空状态
  Widget _buildEmptyState(BuildContext context) {
    return EmptyStateWidget(
      icon: Icons.folder_outlined,
      title: '暂无工作区',
      description: '创建工作区来管理您的日志文件，支持导入文件夹或压缩包',
      actionLabel: '添加工作区',
      onAction: () => _showAddWorkspaceDialog(context),
    );
  }

  /// 构建工作区列表
  Widget _buildWorkspaceList(
    List<Workspace> workspaces,
    String? activeWorkspaceId,
  ) {
    return Focus(
      focusNode: _listFocusNode,
      autofocus: true,
      onKeyEvent: (node, event) => _handleKeyEvent(event, workspaces),
      child: ListView.builder(
        itemCount: workspaces.length,
        padding: const EdgeInsets.all(16),
        itemBuilder: (context, index) {
          final workspace = workspaces[index];
          final isActive = workspace.id == activeWorkspaceId;
          final isSelected = index == _selectedIndex;

          return _WorkspaceCard(
            workspace: workspace,
            isActive: isActive,
            isSelected: isSelected,
            onTap: () => _selectWorkspace(workspace.id),
            onDelete: () => _confirmDeleteWorkspace(context, workspace),
            onRefresh: () => _refreshWorkspace(context, workspace),
            onToggleWatch: () => _toggleWatch(context, workspace),
          );
        },
      ),
    );
  }

  /// 处理键盘事件
  KeyEventResult _handleKeyEvent(KeyEvent event, List<Workspace> workspaces) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }

    final key = event.logicalKey;

    // 上箭头：选择上一个工作区
    if (key == LogicalKeyboardKey.arrowUp) {
      setState(() {
        if (_selectedIndex <= 0) {
          _selectedIndex = workspaces.length - 1;
        } else {
          _selectedIndex--;
        }
      });
      return KeyEventResult.handled;
    }

    // 下箭头：选择下一个工作区
    if (key == LogicalKeyboardKey.arrowDown) {
      setState(() {
        if (_selectedIndex >= workspaces.length - 1) {
          _selectedIndex = 0;
        } else {
          _selectedIndex++;
        }
      });
      return KeyEventResult.handled;
    }

    // 回车键：打开选中的工作区
    if (key == LogicalKeyboardKey.enter) {
      if (_selectedIndex >= 0 && _selectedIndex < workspaces.length) {
        _selectWorkspace(workspaces[_selectedIndex].id);
        return KeyEventResult.handled;
      }
    }

    return KeyEventResult.ignored;
  }

  /// 选择工作区
  void _selectWorkspace(String workspaceId) {
    // 找到选中工作区的索引
    final workspaces = ref.read(workspaceStateProvider);
    final index = workspaces.indexWhere((w) => w.id == workspaceId);
    if (index != -1) {
      setState(() {
        _selectedIndex = index;
      });
    }
    ref.read(appStateProvider.notifier).setActiveWorkspace(workspaceId);
  }

  /// 刷新所有工作区
  Future<void> _refreshAllWorkspaces(BuildContext context) async {
    try {
      await ref.read(workspaceStateProvider.notifier).loadWorkspaces();
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('工作区已刷新'),
            backgroundColor: AppColors.success,
            duration: Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('刷新失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }

  /// 显示添加工作区对话框
  void _showAddWorkspaceDialog(BuildContext context, {String? initialPath}) {
    showDialog(
      context: context,
      builder: (context) => _AddWorkspaceDialog(initialPath: initialPath),
    );
  }

  /// 确认删除工作区
  void _confirmDeleteWorkspace(BuildContext context, Workspace workspace) {
    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        backgroundColor: AppColors.bgCard,
        title: const Text('删除工作区'),
        content: Text(
          '确定要删除工作区 "${workspace.name}" 吗？\n'
          '此操作不会删除实际文件，仅删除工作区配置。',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('取消'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(dialogContext);
              _deleteWorkspace(context, workspace.id);
            },
            style: ElevatedButton.styleFrom(backgroundColor: AppColors.error),
            child: const Text('删除'),
          ),
        ],
      ),
    );
  }

  /// 删除工作区
  Future<void> _deleteWorkspace(
    BuildContext context,
    String workspaceId,
  ) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      await apiService.deleteWorkspace(workspaceId);

      if (context.mounted) {
        ref.read(workspaceStateProvider.notifier).removeWorkspace(workspaceId);
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('工作区已删除'),
            backgroundColor: AppColors.success,
            duration: Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('删除失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }

  /// 刷新工作区
  Future<void> _refreshWorkspace(
    BuildContext context,
    Workspace workspace,
  ) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      await apiService.refreshWorkspace(workspace.id, workspace.path);

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('正在刷新工作区: ${workspace.name}'),
            duration: const Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('刷新失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }

  /// 切换文件监听
  Future<void> _toggleWatch(BuildContext context, Workspace workspace) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      final newValue = !(workspace.watching ?? false);

      if (newValue) {
        await apiService.startWatch(
          workspaceId: workspace.id,
          paths: [workspace.path],
          recursive: true,
        );
      } else {
        await apiService.stopWatch(workspace.id);
      }

      if (context.mounted) {
        ref
            .read(workspaceStateProvider.notifier)
            .updateWorkspace(
              workspace.id,
              workspace.copyWith(watching: newValue),
            );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('操作失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }
}

/// 工作区卡片组件
class _WorkspaceCard extends StatelessWidget {
  final Workspace workspace;
  final bool isActive;
  final bool isSelected;
  final VoidCallback onTap;
  final VoidCallback onDelete;
  final VoidCallback onRefresh;
  final VoidCallback onToggleWatch;

  const _WorkspaceCard({
    required this.workspace,
    required this.isActive,
    this.isSelected = false,
    required this.onTap,
    required this.onDelete,
    required this.onRefresh,
    required this.onToggleWatch,
  });

  @override
  Widget build(BuildContext context) {
    final statusColor = _getStatusColor(workspace.status.value);
    final statusText = _getStatusText(workspace.status.value);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      color: isSelected ? AppColors.primary.withValues(alpha: 0.1) : null,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            border: Border.all(
              color: isActive
                  ? AppColors.primary
                  : isSelected
                  ? AppColors.primary.withValues(alpha: 0.5)
                  : Colors.transparent,
              width: isActive || isSelected ? 2 : 1,
            ),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 标题行
              Row(
                children: [
                  Expanded(
                    child: Text(
                      workspace.name,
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: isActive
                            ? AppColors.primary
                            : AppColors.textPrimary,
                      ),
                    ),
                  ),
                  // 状态标签
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: statusColor.withValues(alpha: 0.15),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: statusColor.withValues(alpha: 0.3),
                        width: 1,
                      ),
                    ),
                    child: Row(
                      children: [
                        Icon(
                          _getStatusIcon(workspace.status.value),
                          size: 12,
                          color: statusColor,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          statusText,
                          style: TextStyle(
                            color: statusColor,
                            fontSize: 11,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ),
                  // 监听状态
                  if (workspace.watching ?? false)
                    const Icon(
                      Icons.visibility,
                      size: 16,
                      color: AppColors.success,
                    ),
                ],
              ),
              const SizedBox(height: 8),
              // 路径
              Text(
                workspace.path,
                style: const TextStyle(
                  fontSize: 13,
                  color: AppColors.textSecondary,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 4),
              // 统计信息
              Row(
                children: [
                  const Icon(
                    Icons.insert_drive_file_outlined,
                    size: 14,
                    color: AppColors.textMuted,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    '${workspace.files} 文件',
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textSecondary,
                    ),
                  ),
                  const SizedBox(width: 16),
                  const Icon(
                    Icons.storage_outlined,
                    size: 14,
                    color: AppColors.textMuted,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    workspace.size,
                    style: const TextStyle(
                      fontSize: 12,
                      color: AppColors.textSecondary,
                    ),
                  ),
                  // 最近打开时间
                  if (workspace.lastOpenedAt != null) ...[
                    const SizedBox(width: 16),
                    const Icon(
                      Icons.access_time,
                      size: 14,
                      color: AppColors.textMuted,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      _formatDateTime(workspace.lastOpenedAt),
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textSecondary,
                      ),
                    ),
                  ],
                ],
              ),
              // 创建时间
              if (workspace.createdAt != null) ...[
                const SizedBox(height: 4),
                Row(
                  children: [
                    const Icon(
                      Icons.calendar_today,
                      size: 14,
                      color: AppColors.textMuted,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '创建于 ${_formatDateTime(workspace.createdAt)}',
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppColors.textMuted,
                      ),
                    ),
                  ],
                ),
              ],
              const SizedBox(height: 12),
              // 操作按钮
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  if (workspace.watching ?? false)
                    IconButton(
                      icon: const Icon(Icons.visibility_off, size: 18),
                      tooltip: '停止监听',
                      onPressed: onToggleWatch,
                      padding: const EdgeInsets.all(8),
                    ),
                  IconButton(
                    icon: const Icon(Icons.refresh, size: 18),
                    tooltip: '刷新',
                    onPressed: onRefresh,
                    padding: const EdgeInsets.all(8),
                  ),
                  IconButton(
                    icon: const Icon(Icons.delete_outline, size: 18),
                    tooltip: '删除',
                    color: AppColors.error,
                    onPressed: onDelete,
                    padding: const EdgeInsets.all(8),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Color _getStatusColor(String status) {
    switch (status) {
      case 'READY':
        return AppColors.success;
      case 'SCANNING':
      case 'PROCESSING':
        return AppColors.warning;
      case 'OFFLINE':
        return AppColors.error;
      case 'INDEXING':
        return AppColors.primary;
      case 'ERROR':
        return AppColors.error;
      default:
        return AppColors.textMuted;
    }
  }

  IconData _getStatusIcon(String status) {
    switch (status) {
      case 'READY':
        return Icons.check_circle;
      case 'SCANNING':
      case 'PROCESSING':
      case 'INDEXING':
        return Icons.sync;
      case 'OFFLINE':
        return Icons.cloud_off;
      case 'ERROR':
        return Icons.error;
      default:
        return Icons.help_outline;
    }
  }

  String _getStatusText(String status) {
    switch (status) {
      case 'READY':
        return '就绪';
      case 'SCANNING':
        return '扫描中';
      case 'PROCESSING':
        return '处理中';
      case 'INDEXING':
        return '索引中';
      case 'OFFLINE':
        return '离线';
      case 'ERROR':
        return '错误';
      default:
        return '未知';
    }
  }

  /// 格式化日期时间显示
  String _formatDateTime(DateTime? dateTime) {
    if (dateTime == null) return '';
    final now = DateTime.now();
    final diff = now.difference(dateTime);

    if (diff.inDays == 0) {
      if (diff.inHours == 0) {
        if (diff.inMinutes == 0) {
          return '刚刚';
        }
        return '${diff.inMinutes} 分钟前';
      }
      return '${diff.inHours} 小时前';
    } else if (diff.inDays == 1) {
      return '昨天';
    } else if (diff.inDays < 7) {
      return '${diff.inDays} 天前';
    } else {
      return '${dateTime.year}-${dateTime.month.toString().padLeft(2, '0')}-${dateTime.day.toString().padLeft(2, '0')}';
    }
  }
}

/// 添加工作区对话框
class _AddWorkspaceDialog extends ConsumerStatefulWidget {
  final String? initialPath;

  const _AddWorkspaceDialog({super.key, this.initialPath});

  @override
  ConsumerState<_AddWorkspaceDialog> createState() =>
      _AddWorkspaceDialogState();
}

class _AddWorkspaceDialogState extends ConsumerState<_AddWorkspaceDialog> {
  final _nameController = TextEditingController();
  final _pathController = TextEditingController();
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    // 如果有初始路径，自动填充
    if (widget.initialPath != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        setState(() {
          _pathController.text = widget.initialPath!;
          // 自动使用文件夹名作为名称
          final parts = widget.initialPath!.split(Platform.pathSeparator);
          final folderName = parts.isNotEmpty ? parts.last : '新工作区';
          _nameController.text = folderName;
        });
      });
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _pathController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppColors.bgCard,
      title: const Text('添加工作区'),
      content: SizedBox(
        width: 400,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // 名称输入
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(
                labelText: '工作区名称',
                hintText: '例如: 生产环境日志',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 16),
            // 路径输入
            TextField(
              controller: _pathController,
              decoration: InputDecoration(
                labelText: '日志路径',
                hintText: '选择包含日志的文件夹',
                border: const OutlineInputBorder(),
                suffixIcon: IconButton(
                  icon: const Icon(Icons.folder_open),
                  tooltip: '浏览文件夹',
                  onPressed: _selectFolder,
                ),
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        ElevatedButton(
          onPressed: _isLoading ? null : _createWorkspace,
          child: _isLoading
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('创建'),
        ),
      ],
    );
  }

  /// 选择文件夹
  Future<void> _selectFolder() async {
    try {
      final result = await FilePicker.platform.getDirectoryPath(
        dialogTitle: '选择日志文件夹',
        // 初始目录（可选）
        initialDirectory: Platform.isWindows
            ? 'C:\\'
            : Platform.isMacOS
            ? '/Users'
            : '/home',
      );

      if (result != null) {
        setState(() {
          _pathController.text = result;
          // 如果名称为空，自动使用文件夹名作为名称
          if (_nameController.text.isEmpty) {
            final parts = result.split(Platform.pathSeparator);
            final folderName = parts.isNotEmpty ? parts.last : '新工作区';
            _nameController.text = folderName;
          }
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('选择文件夹失败: $e'),
            backgroundColor: AppColors.error,
          ),
        );
      }
    }
  }

  /// 创建工作区
  Future<void> _createWorkspace() async {
    final name = _nameController.text.trim();
    final path = _pathController.text.trim();

    if (name.isEmpty || path.isEmpty) {
      return;
    }

    setState(() {
      _isLoading = true;
    });

    try {
      final apiService = ref.read(apiServiceProvider);
      final workspaceId = await apiService.createWorkspace(
        name: name,
        path: path,
      );

      // 创建新的工作区对象
      final newWorkspace = Workspace(
        id: workspaceId,
        name: name,
        path: path,
        status: const WorkspaceStatusData(value: 'SCANNING'),
        size: '0 MB',
        files: 0,
        watching: false,
      );

      ref.read(workspaceStateProvider.notifier).addWorkspace(newWorkspace);

      if (mounted) {
        Navigator.pop(context);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('正在导入: $name'),
            backgroundColor: AppColors.primary,
            duration: const Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      setState(() {
        _isLoading = false;
      });

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('创建失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }
}
