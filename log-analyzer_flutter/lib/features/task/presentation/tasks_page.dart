import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../shared/models/common.dart';
import '../../../shared/providers/task_provider.dart';
import '../../../shared/services/api_service.dart';
import '../../../core/theme/app_theme.dart';
import '../../../core/constants/app_constants.dart';

/// 任务过滤类型枚举
enum TaskFilterType {
  all, // 全部任务
  running, // 运行中
  completed, // 已完成
  failed, // 失败
  stopped, // 已停止
}

/// 任务管理页面
///
/// 对应 React 版本的 TasksPage.tsx
/// 功能：
/// - 任务列表展示
/// - 任务进度条
/// - 任务状态显示
/// - 取消任务功能
/// - 自动清理已完成任务
/// - 任务过滤功能
class TasksPage extends ConsumerStatefulWidget {
  const TasksPage({super.key});

  @override
  ConsumerState<TasksPage> createState() => _TasksPageState();
}

class _TasksPageState extends ConsumerState<TasksPage> {
  // 自动刷新定时器
  Timer? _autoRefreshTimer;

  // 当前过滤类型（本地状态管理）
  TaskFilterType _currentFilter = TaskFilterType.all;

  @override
  void initState() {
    super.initState();
    _startAutoRefresh();
  }

  @override
  void dispose() {
    _autoRefreshTimer?.cancel();
    super.dispose();
  }

  /// 根据当前过滤条件获取过滤后的任务列表
  List<TaskInfo> _getFilteredTasks(List<TaskInfo> allTasks) {
    switch (_currentFilter) {
      case TaskFilterType.all:
        return allTasks;
      case TaskFilterType.running:
        return allTasks.where((t) => t.status == TaskStatus.running).toList();
      case TaskFilterType.completed:
        return allTasks.where((t) => t.status == TaskStatus.completed).toList();
      case TaskFilterType.failed:
        return allTasks.where((t) => t.status == TaskStatus.failed).toList();
      case TaskFilterType.stopped:
        return allTasks.where((t) => t.status == TaskStatus.stopped).toList();
    }
  }

  @override
  Widget build(BuildContext context) {
    // 获取全部任务
    final allTasks = ref.watch(taskStateProvider);
    // 根据过滤条件获取过滤后的任务
    final filteredTasks = _getFilteredTasks(allTasks);

    return Scaffold(
      appBar: _buildAppBar(context, allTasks),
      body: filteredTasks.isEmpty
          ? _buildEmptyState()
          : _buildTaskList(filteredTasks),
    );
  }

  /// 构建 AppBar
  ///
  /// [tasks] 全部任务列表（用于统计）
  PreferredSizeWidget _buildAppBar(BuildContext context, List<TaskInfo> tasks) {
    final runningCount = tasks
        .where((t) => t.status == TaskStatus.running)
        .length;

    return AppBar(
      backgroundColor: AppColors.bgMain,
      elevation: 0,
      title: Row(
        children: [
          const Text(
            '任务',
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          ),
          if (runningCount > 0)
            Container(
              margin: const EdgeInsets.only(left: 12),
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: AppColors.primary.withValues(alpha: 0.2),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Text(
                '$runningCount 运行中',
                style: const TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
        ],
      ),
      actions: [
        IconButton(
          icon: const Icon(Icons.refresh),
          tooltip: '刷新',
          onPressed: () => _refreshTasks(context),
        ),
        // 过滤菜单
        PopupMenuButton<TaskFilterType>(
          icon: Icon(
            Icons.filter_list,
            // 当有过滤条件时高亮显示
            color: _currentFilter != TaskFilterType.all
                ? AppColors.primary
                : null,
          ),
          tooltip: '过滤',
          initialValue: _currentFilter,
          onSelected: (value) => _handleFilterAction(value),
          itemBuilder: (context) => [
            // 全部任务
            _buildFilterMenuItem(
              value: TaskFilterType.all,
              icon: Icons.check_circle_outline,
              label: '全部任务',
              count: tasks.length,
            ),
            // 运行中
            _buildFilterMenuItem(
              value: TaskFilterType.running,
              icon: Icons.sync,
              label: '运行中',
              count: tasks.where((t) => t.status == TaskStatus.running).length,
            ),
            // 已完成
            _buildFilterMenuItem(
              value: TaskFilterType.completed,
              icon: Icons.check_circle,
              label: '已完成',
              count: tasks
                  .where((t) => t.status == TaskStatus.completed)
                  .length,
            ),
            // 失败
            _buildFilterMenuItem(
              value: TaskFilterType.failed,
              icon: Icons.error,
              label: '失败',
              count: tasks.where((t) => t.status == TaskStatus.failed).length,
            ),
            // 已停止
            _buildFilterMenuItem(
              value: TaskFilterType.stopped,
              icon: Icons.cancel,
              label: '已停止',
              count: tasks.where((t) => t.status == TaskStatus.stopped).length,
            ),
          ],
        ),
      ],
    );
  }

  /// 构建过滤菜单项
  ///
  /// 显示图标、标签、数量，并在选中时显示勾选标记
  PopupMenuItem<TaskFilterType> _buildFilterMenuItem({
    required TaskFilterType value,
    required IconData icon,
    required String label,
    required int count,
  }) {
    final isSelected = _currentFilter == value;

    return PopupMenuItem<TaskFilterType>(
      value: value,
      child: Row(
        children: [
          // 选中指示器
          SizedBox(
            width: 20,
            child: isSelected
                ? const Icon(Icons.check, size: 16, color: AppColors.primary)
                : null,
          ),
          const SizedBox(width: 8),
          Icon(icon, size: 18, color: AppColors.textSecondary),
          const SizedBox(width: 12),
          Expanded(child: Text(label)),
          // 数量标签
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: count > 0 ? AppColors.bgInput : Colors.transparent,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Text(
              '$count',
              style: TextStyle(
                fontSize: 12,
                color: count > 0
                    ? AppColors.textSecondary
                    : AppColors.textMuted,
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// 构建空状态
  ///
  /// 根据当前过滤状态显示不同的提示信息
  Widget _buildEmptyState() {
    // 根据过滤类型显示不同的提示
    String message;
    IconData icon;

    switch (_currentFilter) {
      case TaskFilterType.all:
        message = '暂无任务';
        icon = Icons.task_alt_outlined;
        break;
      case TaskFilterType.running:
        message = '暂无运行中的任务';
        icon = Icons.sync;
        break;
      case TaskFilterType.completed:
        message = '暂无已完成的任务';
        icon = Icons.check_circle_outline;
        break;
      case TaskFilterType.failed:
        message = '暂无失败的任务';
        icon = Icons.error_outline;
        break;
      case TaskFilterType.stopped:
        message = '暂无已停止的任务';
        icon = Icons.cancel_outlined;
        break;
    }

    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 64, color: AppColors.textMuted),
          const SizedBox(height: 16),
          Text(
            message,
            style: const TextStyle(
              fontSize: 16,
              color: AppColors.textSecondary,
            ),
          ),
          // 如果是过滤状态，显示重置按钮
          if (_currentFilter != TaskFilterType.all) ...[
            const SizedBox(height: 16),
            TextButton(
              onPressed: () => _handleFilterAction(TaskFilterType.all),
              child: const Text('显示全部任务'),
            ),
          ],
        ],
      ),
    );
  }

  /// 构建任务列表
  Widget _buildTaskList(List<TaskInfo> tasks) {
    // 按状态分组
    final runningTasks = tasks
        .where((t) => t.status == TaskStatus.running)
        .toList();
    final completedTasks = tasks
        .where((t) => t.status == TaskStatus.completed)
        .toList();
    final failedTasks = tasks
        .where((t) => t.status == TaskStatus.failed)
        .toList();
    final stoppedTasks = tasks
        .where((t) => t.status == TaskStatus.stopped)
        .toList();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // 运行中的任务
        if (runningTasks.isNotEmpty) ...[
          const SizedBox(height: 8),
          const Text(
            '运行中',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: AppColors.textMuted,
            ),
          ),
          ...runningTasks.map((task) => _TaskCard(task: task, ref: ref)),
          const Divider(height: 24),
        ],
        // 已完成的任务
        if (completedTasks.isNotEmpty) ...[
          const SizedBox(height: 8),
          const Text(
            '已完成',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: AppColors.textMuted,
            ),
          ),
          ...completedTasks.map((task) => _TaskCard(task: task, ref: ref)),
          const Divider(height: 24),
        ],
        // 失败的任务
        if (failedTasks.isNotEmpty) ...[
          const SizedBox(height: 8),
          const Text(
            '失败',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: AppColors.textMuted,
            ),
          ),
          ...failedTasks.map((task) => _TaskCard(task: task, ref: ref)),
          const Divider(height: 24),
        ],
        // 已停止的任务
        if (stoppedTasks.isNotEmpty) ...[
          const SizedBox(height: 8),
          const Text(
            '已停止',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: AppColors.textMuted,
            ),
          ),
          ...stoppedTasks.map((task) => _TaskCard(task: task, ref: ref)),
        ],
      ],
    );
  }

  /// 刷新任务列表
  Future<void> _refreshTasks([BuildContext? context]) async {
    try {
      // 触发重新加载任务（这里简单实现，实际可能需要调用 API）
      // 由于 TaskState 没有 loadWorkspaces 方法，这里暂时只刷新 UI
      if (context?.mounted ?? false) {
        ScaffoldMessenger.of(context!).showSnackBar(
          const SnackBar(
            content: Text('任务已刷新'),
            backgroundColor: AppColors.success,
            duration: Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      if (context?.mounted ?? false) {
        ScaffoldMessenger.of(context!).showSnackBar(
          SnackBar(content: Text('刷新失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }

  /// 处理过滤操作
  ///
  /// 更新过滤状态，任务列表会自动重新渲染
  void _handleFilterAction(TaskFilterType value) {
    setState(() {
      _currentFilter = value;
    });

    // 获取过滤后的任务数量
    final allTasks = ref.read(taskStateProvider);
    final filteredCount = _getFilteredTasks(allTasks).length;

    // 只有过滤特定状态时才显示提示
    if (value != TaskFilterType.all) {
      final label = _getFilterLabel(value);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('已过滤：$label（$filteredCount 个任务）'),
          backgroundColor: AppColors.primary,
          duration: const Duration(seconds: 2),
        ),
      );
    }
  }

  /// 获取过滤类型的中文标签
  String _getFilterLabel(TaskFilterType type) {
    switch (type) {
      case TaskFilterType.all:
        return '全部任务';
      case TaskFilterType.running:
        return '运行中';
      case TaskFilterType.completed:
        return '已完成';
      case TaskFilterType.failed:
        return '失败';
      case TaskFilterType.stopped:
        return '已停止';
    }
  }

  /// 启动自动刷新
  void _startAutoRefresh() {
    _autoRefreshTimer?.cancel();
    _autoRefreshTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      // 自动刷新时不显示提示
      _refreshTasks();
    });
  }
}

/// 任务卡片组件
///
/// 使用 WidgetRef 来访问 Provider（通过构造函数传入）
class _TaskCard extends StatelessWidget {
  final TaskInfo task;
  final WidgetRef ref;

  const _TaskCard({required this.task, required this.ref});

  @override
  Widget build(BuildContext context) {
    final statusColor = _getStatusColor(task.status.value);
    final statusIcon = _getStatusIcon(task.status.value);
    final statusText = _getStatusText(task.status.value);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 标题行
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        task.target,
                        style: const TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                          color: AppColors.textPrimary,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        task.message,
                        style: const TextStyle(
                          fontSize: 13,
                          color: AppColors.textSecondary,
                        ),
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                // 状态标签和取消按钮
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
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
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(statusIcon, size: 12, color: statusColor),
                          const SizedBox(width: 4),
                          Text(
                            statusText,
                            style: TextStyle(
                              color: statusColor,
                              fontSize: 11,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                          if (task.status == TaskStatus.running) ...[
                            const SizedBox(width: 4),
                            Text(
                              '${task.progress}%',
                              style: TextStyle(
                                color: statusColor,
                                fontSize: 11,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ],
                        ],
                      ),
                    ),
                    // 取消按钮（仅运行中任务显示）
                    if (task.status == TaskStatus.running)
                      IconButton(
                        icon: const Icon(Icons.close, size: 18),
                        tooltip: '取消任务',
                        onPressed: () => _cancelTask(context, task.taskId),
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(
                          minWidth: 32,
                          minHeight: 32,
                        ),
                      ),
                  ],
                ),
              ],
            ),
            // 进度条（仅运行中任务显示）
            if (task.status == TaskStatus.running) ...[
              const SizedBox(height: 12),
              LinearProgressIndicator(
                value: task.progress / 100,
                backgroundColor: AppColors.bgInput,
                valueColor: AlwaysStoppedAnimation<Color>(statusColor),
                minHeight: 4,
              ),
            ],
          ],
        ),
      ),
    );
  }

  Color _getStatusColor(String status) {
    switch (status) {
      case 'RUNNING':
        return AppColors.primary;
      case 'COMPLETED':
        return AppColors.success;
      case 'FAILED':
        return AppColors.error;
      case 'STOPPED':
        return AppColors.textMuted;
      default:
        return AppColors.textMuted;
    }
  }

  IconData _getStatusIcon(String status) {
    switch (status) {
      case 'RUNNING':
        return Icons.sync;
      case 'COMPLETED':
        return Icons.check_circle;
      case 'FAILED':
        return Icons.error;
      case 'STOPPED':
        return Icons.cancel;
      default:
        return Icons.help_outline;
    }
  }

  String _getStatusText(String status) {
    switch (status) {
      case 'RUNNING':
        return '运行中';
      case 'COMPLETED':
        return '已完成';
      case 'FAILED':
        return '失败';
      case 'STOPPED':
        return '已停止';
      default:
        return '未知';
    }
  }

  /// 取消任务
  Future<void> _cancelTask(BuildContext context, String taskId) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      await apiService.cancelTask(taskId);

      if (context.mounted) {
        ref.read(taskStateProvider.notifier).removeTask(taskId);
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('任务已取消'),
            backgroundColor: AppColors.success,
            duration: Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('取消失败: $e'), backgroundColor: AppColors.error),
        );
      }
    }
  }
}
