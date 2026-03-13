/// 任务页面
/// 
/// 展示事件驱动的任务状态更新
/// 使用 StreamProvider 实时显示任务进度

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../domain/entities/task.dart';
import '../../providers/task_provider.dart';
import '../../widgets/common/async_value_widget.dart';

/// 任务页面
class TasksPage extends ConsumerWidget {
  const TasksPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // 使用 AsyncValueWidget 统一处理状态
    return Scaffold(
      appBar: AppBar(
        title: const Text('任务管理'),
        actions: [
          // 清理已完成任务
          IconButton(
            icon: const Icon(Icons.cleaning_services),
            tooltip: '清理已完成',
            onPressed: () => _cleanupCompleted(context, ref),
          ),
        ],
      ),
      body: Column(
        children: [
          // 任务统计
          const _TaskStatsCard(),
          
          // 任务列表
          Expanded(
            child: Consumer(
              builder: (context, ref, child) {
                final tasksAsync = ref.watch(taskListProvider);
                
                return AsyncValueWidget<List<Task>>(
                  value: tasksAsync,
                  data: (tasks) => TaskListView(
                    tasks: tasks,
                    onCancel: (task) => _cancelTask(context, ref, task),
                  ),
                );
              },
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _cancelTask(
    BuildContext context,
    WidgetRef ref,
    Task task,
  ) async {
    try {
      await ref.read(taskListProvider.notifier).cancelTask(task.id);
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('任务已取消')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('取消失败: $e')),
        );
      }
    }
  }

  Future<void> _cleanupCompleted(BuildContext context, WidgetRef ref) async {
    final count = await ref.read(taskListProvider.notifier).cleanupCompleted();
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('已清理 $count 个任务')),
      );
    }
  }
}

/// 任务统计卡片
class _TaskStatsCard extends ConsumerWidget {
  const _TaskStatsCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final metricsAsync = ref.watch(taskMetricsNotifierProvider);
    final theme = Theme.of(context);

    return metricsAsync.when(
      data: (metrics) => Card(
        margin: const EdgeInsets.all(16),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            children: [
              _StatItem(
                label: '总计',
                value: metrics.total.toString(),
                color: theme.colorScheme.primary,
              ),
              _StatItem(
                label: '运行中',
                value: metrics.running.toString(),
                color: Colors.orange,
              ),
              _StatItem(
                label: '已完成',
                value: metrics.completed.toString(),
                color: Colors.green,
              ),
              _StatItem(
                label: '失败',
                value: metrics.failed.toString(),
                color: Colors.red,
              ),
            ],
          ),
        ),
      ),
      loading: () => const Card(
        margin: EdgeInsets.all(16),
        child: SizedBox(height: 80),
      ),
      error: (_, __) => const SizedBox.shrink(),
    );
  }
}

/// 统计项
class _StatItem extends StatelessWidget {
  final String label;
  final String value;
  final Color color;

  const _StatItem({
    required this.label,
    required this.value,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      children: [
        Text(
          value,
          style: theme.textTheme.headlineSmall?.copyWith(
            color: color,
            fontWeight: FontWeight.bold,
          ),
        ),
        Text(
          label,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}

/// 任务列表视图
class TaskListView extends StatelessWidget {
  final List<Task> tasks;
  final Function(Task) onCancel;

  const TaskListView({
    super.key,
    required this.tasks,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context) {
    if (tasks.isEmpty) {
      return const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.check_circle_outline, size: 64, color: Colors.grey),
            SizedBox(height: 16),
            Text('暂无任务'),
          ],
        ),
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      itemCount: tasks.length,
      itemBuilder: (context, index) => TaskCard(
        task: tasks[index],
        onCancel: () => onCancel(tasks[index]),
      ),
    );
  }
}

/// 任务卡片
class TaskCard extends StatelessWidget {
  final Task task;
  final VoidCallback onCancel;

  const TaskCard({
    super.key,
    required this.task,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 标题行
            Row(
              children: [
                _TaskTypeIcon(type: task.type),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        task.description,
                        style: theme.textTheme.titleSmall,
                      ),
                      Text(
                        task.type.toString().split('.').last,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                _StatusBadge(status: task.status),
              ],
            ),
            
            // 进度条
            if (task.isActive) ...[
              const SizedBox(height: 12),
              LinearProgressIndicator(
                value: task.progress / 100,
                backgroundColor: theme.colorScheme.surfaceContainerHighest,
              ),
              const SizedBox(height: 4),
              Text(
                task.formattedProgress,
                style: theme.textTheme.bodySmall,
              ),
            ],
            
            // 操作按钮
            if (task.isActive) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: TextButton(
                  onPressed: onCancel,
                  child: const Text('取消'),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// 任务类型图标
class _TaskTypeIcon extends StatelessWidget {
  final TaskType type;

  const _TaskTypeIcon({required this.type});

  @override
  Widget build(BuildContext context) {
    final icon = switch (type) {
      TaskType.importFolder => Icons.folder_copy,
      TaskType.refreshWorkspace => Icons.refresh,
      TaskType.search => Icons.search,
      TaskType.export => Icons.download,
      TaskType.indexing => Icons.storage,
    };

    return Icon(icon, color: Theme.of(context).colorScheme.primary);
  }
}

/// 状态徽章
class _StatusBadge extends StatelessWidget {
  final TaskStatus status;

  const _StatusBadge({required this.status});

  @override
  Widget build(BuildContext context) {
    final (color, text) = switch (status) {
      TaskStatus.pending => (Colors.grey, '等待'),
      TaskStatus.running => (Colors.orange, '运行'),
      TaskStatus.completed => (Colors.green, '完成'),
      TaskStatus.failed => (Colors.red, '失败'),
      TaskStatus.cancelled => (Colors.grey, '取消'),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: TextStyle(
          color: color,
          fontSize: 12,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}
