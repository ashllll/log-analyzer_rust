import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/common.dart';
import '../services/api_service.dart';
import '../../core/constants/app_constants.dart';
import 'app_provider.dart';

part 'task_provider.g.dart';

/// 任务状态 Provider
///
/// 对应 React 版本的 taskStore.ts
/// 管理后台任务生命周期
@riverpod
class TaskState extends _$TaskState {
  /// 清理定时器
  Timer? _cleanupTimer;

  /// 任务完成时间戳缓存（用于 TTL 清理）
  final Map<String, int> _completedAtCache = {};

  @override
  List<TaskInfo> build() {
    // 启动任务清理定时器
    _startCleanupTimer();

    // 加载初始任务指标
    Future.microtask(() => _loadTaskMetrics());

    // 注册 dispose 回调
    ref.onDispose(() {
      _cleanupTimer?.cancel();
      _completedAtCache.clear();
    });

    return [];
  }

  /// 加载任务指标
  ///
  /// 从后端获取当前任务状态
  Future<void> _loadTaskMetrics() async {
    try {
      final apiService = ref.read(apiServiceProvider);

      if (!apiService.isFfiAvailable) {
        debugPrint('TaskState: FFI 桥接不可用，跳过加载任务指标');
        return;
      }

      final metrics = await apiService.getTaskMetrics();
      debugPrint('TaskState: 任务指标 - 总计: ${metrics.total}, 运行中: ${metrics.running}');
    } catch (e) {
      debugPrint('TaskState: 加载任务指标失败: $e');
    }
  }

  /// 启动清理定时器
  ///
  /// 自动清理已完成的任务（TTL 机制）
  /// 对应 React 版本的 TTL 清理机制
  void _startCleanupTimer() {
    _cleanupTimer?.cancel();

    // 每 5 分钟清理一次过期任务
    _cleanupTimer = Timer.periodic(
      AppConstants.taskCleanupInterval,
      (_) => cleanupExpiredTasks(),
    );

    debugPrint('TaskState: 任务清理定时器已启动，间隔: ${AppConstants.taskCleanupInterval}');
  }

  /// 清理过期任务
  ///
  /// 清理已完成超过 TTL 时间的任务
  void cleanupExpiredTasks() {
    if (state.isEmpty) return;

    final now = DateTime.now().millisecondsSinceEpoch;
    final ttlMs = AppConstants.completedTaskTtl.inMilliseconds;
    int removedCount = 0;

    state = state.where((task) {
      // 保留运行中的任务
      if (task.status.value == 'RUNNING') {
        return true;
      }

      // 获取任务完成时间
      final completedAt = _completedAtCache[task.taskId];
      if (completedAt == null) {
        // 如果没有记录完成时间，记录当前时间
        _completedAtCache[task.taskId] = now;
        return true;
      }

      // 检查是否超过 TTL
      final elapsed = now - completedAt;
      if (elapsed > ttlMs) {
        removedCount++;
        _completedAtCache.remove(task.taskId);
        return false;
      }

      return true;
    }).toList();

    if (removedCount > 0) {
      debugPrint('TaskState: 已清理 $removedCount 个过期任务');
    }
  }

  /// 添加任务（如果不存在）
  ///
  /// 对应 React 版本的 addTaskIfNotExists()
  /// 实现任务去重
  void addTaskIfNotExists(TaskInfo task) {
    final exists = state.any((t) => t.taskId == task.taskId);
    if (!exists) {
      state = [...state, task];

      // 如果任务已完成，记录完成时间
      if (task.status.value != 'RUNNING') {
        _completedAtCache[task.taskId] = DateTime.now().millisecondsSinceEpoch;
      }

      debugPrint('TaskState: 添加任务 ${task.taskId}, 类型: ${task.taskType}');
    }
  }

  /// 更新任务
  ///
  /// 使用版本号进行幂等性检查
  void updateTask(TaskInfo updated) {
    final index = state.indexWhere((t) => t.taskId == updated.taskId);

    if (index >= 0) {
      final existing = state[index];

      // 版本号检查：跳过旧版本事件
      if (existing.version >= updated.version) {
        debugPrint('TaskState: 跳过旧版本任务更新 ${updated.taskId}, 现有版本: ${existing.version}, 新版本: ${updated.version}');
        return;
      }

      // 更新任务
      final newList = [...state];
      newList[index] = updated;
      state = newList;

      // 如果任务变为完成状态，记录完成时间
      if (existing.status.value == 'RUNNING' && updated.status.value != 'RUNNING') {
        _completedAtCache[updated.taskId] = DateTime.now().millisecondsSinceEpoch;
      }

      debugPrint('TaskState: 更新任务 ${updated.taskId}, 状态: ${updated.status.value}, 进度: ${updated.progress}%');
    } else {
      // 任务不存在，添加新任务
      state = [...state, updated];

      // 如果任务已完成，记录完成时间
      if (updated.status.value != 'RUNNING') {
        _completedAtCache[updated.taskId] = DateTime.now().millisecondsSinceEpoch;
      }

      debugPrint('TaskState: 添加新任务 ${updated.taskId}');
    }
  }

  /// 移除任务
  void removeTask(String taskId) {
    state = state.where((t) => t.taskId != taskId).toList();
    _completedAtCache.remove(taskId);
    debugPrint('TaskState: 移除任务 $taskId');
  }

  /// 取消任务
  ///
  /// 调用后端 API 取消正在运行的任务
  Future<bool> cancelTask(String taskId) async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 调用后端 API 取消任务
      await apiService.cancelTask(taskId);

      // 更新本地状态
      state = state.map((t) {
        if (t.taskId == taskId) {
          return t.copyWith(
            status: const TaskStatusData(value: 'STOPPED'),
          );
        }
        return t;
      }).toList();

      ref.read(appStateProvider.notifier).addToast(
            ToastType.success,
            '任务已取消',
          );

      return true;
    } catch (e) {
      debugPrint('TaskState: 取消任务失败: $e');
      ref.read(appStateProvider.notifier).addToast(
            ToastType.error,
            '取消任务失败: $e',
          );
      return false;
    }
  }

  /// 清除所有已完成的任务
  void clearCompletedTasks() {
    final completedIds = state
        .where((t) => t.status.value != 'RUNNING')
        .map((t) => t.taskId)
        .toList();

    state = state.where((t) => t.status.value == 'RUNNING').toList();

    // 清理缓存
    for (final id in completedIds) {
      _completedAtCache.remove(id);
    }

    debugPrint('TaskState: 清除 ${completedIds.length} 个已完成任务');
  }

  /// 获取运行中的任务数量
  int get runningCount => state.where((t) => t.status.value == 'RUNNING').length;

  /// 获取已完成任务数量
  int get completedCount => state.where((t) => t.status.value == 'COMPLETED').length;

  /// 获取失败任务数量
  int get failedCount => state.where((t) => t.status.value == 'FAILED').length;

  /// 获取指定工作区的任务
  List<TaskInfo> getTasksByWorkspace(String workspaceId) {
    return state.where((t) => t.workspaceId == workspaceId).toList();
  }

  /// 获取指定类型的任务
  List<TaskInfo> getTasksByType(String taskType) {
    return state.where((t) => t.taskType == taskType).toList();
  }

  /// 根据 ID 获取任务
  TaskInfo? getTaskById(String taskId) {
    try {
      return state.firstWhere((t) => t.taskId == taskId);
    } catch (e) {
      return null;
    }
  }

  /// 是否有运行中的任务
  bool get hasRunningTasks => runningCount > 0;
}

/// 任务过滤类型
enum TaskFilterType {
  all,
  running,
  completed,
  failed,
}
