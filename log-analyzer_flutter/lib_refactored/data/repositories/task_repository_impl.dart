/// 任务仓库实现
/// 
/// 实现 Domain 层定义的接口
/// 使用事件驱动架构替代轮询

import 'dart:async';

import 'package:fpdart/fpdart.dart';

import '../../core/errors/app_error.dart';
import '../../domain/entities/task.dart';
import '../../domain/repositories/task_repository.dart';
import '../datasources/ffi_datasource.dart';
import '../datasources/event_datasource.dart';
import '../mappers/task_mapper.dart';

/// 任务仓库实现
/// 
/// 使用事件驱动架构，通过事件流实现实时更新
class TaskRepositoryImpl implements TaskRepository {
  final FfiDataSource _ffiDataSource;
  final EventDataSource _eventDataSource;

  // 内存缓存
  final Map<String, Task> _taskCache = {};
  final _taskCacheController = StreamController<List<Task>>.broadcast();

  TaskRepositoryImpl({
    FfiDataSource? ffiDataSource,
    EventDataSource? eventDataSource,
  })  : _ffiDataSource = ffiDataSource ?? FfiDataSource.instance,
        _eventDataSource = eventDataSource ?? EventDataSource.instance {
    // 监听任务事件，更新缓存并广播
    _listenToTaskEvents();
  }

  /// 监听任务事件流
  void _listenToTaskEvents() {
    _eventDataSource.taskEvents.listen((event) {
      _handleTaskEvent(event);
    });
  }

  /// 处理任务事件
  void _handleTaskEvent(TaskEvent event) {
    final now = DateTime.now();

    switch (event) {
      case TaskCreatedEvent(:final task):
        _taskCache[task.id] = task;
        
      case TaskProgressEvent(:final taskId, :final progress):
        final existing = _taskCache[taskId];
        if (existing != null) {
          _taskCache[taskId] = existing.withProgress(progress);
        }
        
      case TaskCompletedEvent(:final taskId, :final success, :final error):
        final existing = _taskCache[taskId];
        if (existing != null) {
          if (success) {
            _taskCache[taskId] = existing.completed();
          } else {
            _taskCache[taskId] = existing.failed(error ?? '未知错误');
          }
        }
        
      case TaskCancelledEvent(:final taskId):
        final existing = _taskCache[taskId];
        if (existing != null) {
          _taskCache[taskId] = existing.cancelled();
        }
    }

    // 广播更新
    _taskCacheController.add(_taskCache.values.toList());
  }

  @override
  AppTask<List<Task>> getTasks() {
    return TaskEither(() async {
      try {
        // 从 FFI 获取任务指标（包含任务列表）
        final result = await _ffiDataSource.getTaskMetrics().run();

        return result.fold(
          (error) => left(error),
          (metrics) {
            // 合并缓存的任务
            final tasks = _taskCache.values.toList();
            return right(tasks);
          },
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '获取任务列表失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<Task?> getTaskById(String id) {
    return TaskEither(() async {
      // 先检查缓存
      final cached = _taskCache[id];
      if (cached != null) {
        return right(cached);
      }

      // 获取所有任务并查找
      final result = await getTasks().run();
      return result.map((tasks) {
        try {
          return tasks.firstWhere((t) => t.id == id);
        } catch (_) {
          return null;
        }
      });
    });
  }

  @override
  AppTask<List<Task>> getTasksByWorkspace(String workspaceId) {
    return getTasks().map((tasks) {
      return tasks.where((t) => t.workspaceId == workspaceId).toList();
    });
  }

  @override
  AppTask<TaskMetrics> getTaskMetrics() {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.getTaskMetrics().run();

        return result.fold(
          (error) => left(error),
          (metrics) => right(TaskMapper.metricsFromFfi(metrics)),
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '获取任务指标失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<void> cancelTask(String taskId) {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.cancelTask(taskId).run();

        return result.fold(
          (error) => left(error),
          (success) {
            if (success) {
              // 更新缓存
              final existing = _taskCache[taskId];
              if (existing != null) {
                _taskCache[taskId] = existing.cancelled();
                _taskCacheController.add(_taskCache.values.toList());
              }
              return right(null);
            } else {
              return left(FfiError.call(
                method: 'cancelTask',
                details: '取消任务失败',
              ));
            }
          },
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '取消任务失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  Stream<TaskEvent> watchTaskEvents() {
    return _eventDataSource.taskEvents;
  }

  @override
  Stream<List<Task>> watchTasks() {
    // 合并缓存流和定期刷新
    return _taskCacheController.stream;
  }

  @override
  AppTask<int> cleanupCompletedTasks({required Duration olderThan}) {
    return TaskEither(() async {
      final now = DateTime.now();
      final cutoff = now.subtract(olderThan);
      
      final toRemove = <String>[];
      
      _taskCache.forEach((id, task) {
        if (task.isDone && task.completedAt != null) {
          if (task.completedAt!.isBefore(cutoff)) {
            toRemove.add(id);
          }
        }
      });

      for (final id in toRemove) {
        _taskCache.remove(id);
      }

      // 广播更新
      _taskCacheController.add(_taskCache.values.toList());

      return right(toRemove.length);
    });
  }

  /// 添加任务（内部使用，用于手动创建任务时）
  void addTask(Task task) {
    _taskCache[task.id] = task;
    _taskCacheController.add(_taskCache.values.toList());
  }

  /// 释放资源
  void dispose() {
    _taskCacheController.close();
  }
}
