/// 任务状态管理
/// 
/// 使用事件驱动架构替代轮询
/// 通过 StreamProvider 实现实时更新

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../../core/errors/app_error.dart';
import '../../domain/entities/task.dart';
import '../../domain/repositories/task_repository.dart';
import '../../data/repositories/task_repository_impl.dart';

part 'task_provider.g.dart';

// ==================== Repository Provider ====================

/// 任务仓库 Provider
@Riverpod(keepAlive: true)
TaskRepository taskRepository(Ref ref) {
  final repository = TaskRepositoryImpl();
  
  // 清理时释放资源
  ref.onDispose(() {
    repository.dispose();
  });
  
  return repository;
}

// ==================== Stream Provider（事件驱动）====================

/// 任务事件流
/// 
/// 直接暴露底层事件流，用于高级场景
@Riverpod(keepAlive: true)
Stream<TaskEvent> taskEvents(Ref ref) {
  final repository = ref.watch(taskRepositoryProvider);
  return repository.watchTaskEvents();
}

/// 实时任务列表流
/// 
/// 使用事件驱动自动更新，无需轮询
@riverpod
Stream<List<Task>> taskStream(Ref ref) {
  final repository = ref.watch(taskRepositoryProvider);
  
  // 初始加载
  repository.getTasks().run().then((result) {
    result.fold(
      (error) => [],
      (tasks) => tasks,
    );
  });
  
  // 监听事件流
  return repository.watchTasks();
}

// ==================== AsyncNotifier Providers ====================

/// 任务列表状态
/// 
/// 使用 AsyncNotifier 配合事件流实现自动更新
@riverpod
class TaskList extends _$TaskList {
  StreamSubscription<List<Task>>? _subscription;

  @override
  Future<List<Task>> build() async {
    // 订阅任务流
    _subscribeToTaskStream();
    
    // 初始加载
    final repository = ref.read(taskRepositoryProvider);
    final result = await repository.getTasks().run();
    
    return result.fold(
      (error) => throw error,
      (tasks) => tasks,
    );
  }

  void _subscribeToTaskStream() {
    // 取消之前的订阅
    _subscription?.cancel();
    
    // 订阅新的任务流
    final stream = ref.read(taskStreamProvider);
    _subscription = stream.listen((tasks) {
      state = AsyncValue.data(tasks);
    }, onError: (error) {
      state = AsyncValue.error(error, StackTrace.current);
    });
    
    // 清理时取消订阅
    ref.onDispose(() {
      _subscription?.cancel();
    });
  }

  /// 取消任务
  Future<void> cancelTask(String taskId) async {
    final repository = ref.read(taskRepositoryProvider);
    final result = await repository.cancelTask(taskId).run();
    
    result.fold(
      (error) => throw error,
      (_) {
        // 状态会通过事件流自动更新
      },
    );
  }

  /// 清理已完成的任务
  Future<int> cleanupCompleted() async {
    final repository = ref.read(taskRepositoryProvider);
    final result = await repository.cleanupCompletedTasks(
      olderThan: const Duration(minutes: 5),
    ).run();
    
    return result.getOrElse((_) => 0);
  }

  /// 手动刷新（通常不需要，因为事件流会自动更新）
  Future<void> refresh() async {
    final repository = ref.read(taskRepositoryProvider);
    final result = await repository.getTasks().run();
    
    result.fold(
      (error) => state = AsyncValue.error(error, StackTrace.current),
      (tasks) => state = AsyncValue.data(tasks),
    );
  }
}

/// 任务指标状态
@riverpod
class TaskMetricsNotifier extends _$TaskMetricsNotifier {
  @override
  Future<TaskMetrics> build() async {
    final repository = ref.read(taskRepositoryProvider);
    
    // 监听任务变化，自动更新指标
    ref.listen(taskStreamProvider, (previous, next) {
      if (next.hasValue) {
        _updateMetrics();
      }
    });
    
    final result = await repository.getTaskMetrics().run();
    return result.fold(
      (error) => throw error,
      (metrics) => metrics,
    );
  }

  Future<void> _updateMetrics() async {
    final repository = ref.read(taskRepositoryProvider);
    final result = await repository.getTaskMetrics().run();
    
    result.fold(
      (error) => null, // 保持现有状态
      (metrics) => state = AsyncValue.data(metrics),
    );
  }
}

// ==================== 派生状态 ====================

/// 所有任务
@riverpod
List<Task> allTasks(Ref ref) {
  final tasksAsync = ref.watch(taskListProvider);
  return tasksAsync.when(
    data: (tasks) => tasks,
    loading: () => [],
    error: (_, __) => [],
  );
}

/// 运行中的任务
@riverpod
List<Task> runningTasks(Ref ref) {
  final tasks = ref.watch(allTasksProvider);
  return tasks.where((t) => t.isActive).toList();
}

/// 已完成的任务
@riverpod
List<Task> completedTasks(Ref ref) {
  final tasks = ref.watch(allTasksProvider);
  return tasks.where((t) => t.isDone).toList();
}

/// 失败的任
@riverpod
List<Task> failedTasks(Ref ref) {
  final tasks = ref.watch(allTasksProvider);
  return tasks.where((t) => t.isFailed).toList();
}

/// 是否有运行中的任务
@riverpod
bool hasRunningTasks(Ref ref) {
  return ref.watch(runningTasksProvider).isNotEmpty;
}

/// 运行中任务数量
@riverpod
int runningTaskCount(Ref ref) {
  return ref.watch(runningTasksProvider).length;
}

/// 指定工作区的任务
@riverpod
List<Task> tasksByWorkspace(Ref ref, String workspaceId) {
  final tasks = ref.watch(allTasksProvider);
  return tasks.where((t) => t.workspaceId == workspaceId).toList();
}

/// 指定工作区的运行中任务
@riverpod
List<Task> runningTasksByWorkspace(Ref ref, String workspaceId) {
  final tasks = ref.watch(tasksByWorkspaceProvider(workspaceId));
  return tasks.where((t) => t.isActive).toList();
}

/// 指定工作区是否有运行中任务
@riverpod
bool workspaceHasRunningTasks(Ref ref, String workspaceId) {
  return ref.watch(runningTasksByWorkspaceProvider(workspaceId)).isNotEmpty;
}

/// 任务进度（0-100）
@riverpod
double? taskProgress(Ref ref, String taskId) {
  final tasks = ref.watch(allTasksProvider);
  final task = tasks.where((t) => t.id == taskId).firstOrNull;
  return task?.progress;
}

/// 任务状态
@riverpod
TaskStatus? taskStatus(Ref ref, String taskId) {
  final tasks = ref.watch(allTasksProvider);
  final task = tasks.where((t) => t.id == taskId).firstOrNull;
  return task?.status;
}
