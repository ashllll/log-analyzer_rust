/// 任务仓库接口
/// 
/// 定义任务相关的数据操作

import 'package:fpdart/fpdart.dart';
import '../../core/errors/app_error.dart';
import '../entities/task.dart';

/// 任务仓库接口
abstract class TaskRepository {
  /// 获取所有任务
  AppTask<List<Task>> getTasks();

  /// 根据 ID 获取任务
  AppTask<Task?> getTaskById(String id);

  /// 获取指定工作区的任务
  AppTask<List<Task>> getTasksByWorkspace(String workspaceId);

  /// 获取任务指标
  AppTask<TaskMetrics> getTaskMetrics();

  /// 取消任务
  AppTask<void> cancelTask(String taskId);

  /// 监听任务事件
  /// 
  /// 返回任务事件的实时流，用于事件驱动的状态更新
  /// 替代传统的轮询机制
  Stream<TaskEvent> watchTaskEvents();

  /// 监听任务列表变化
  Stream<List<Task>> watchTasks();

  /// 清理已完成的任务
  /// 
  /// [olderThan] 清理超过指定时间的任务
  AppTask<int> cleanupCompletedTasks({required Duration olderThan});
}
