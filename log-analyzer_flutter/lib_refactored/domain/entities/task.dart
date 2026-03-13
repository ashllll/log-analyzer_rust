/// 任务实体
/// 
/// 表示后台运行的异步任务

import 'package:dart_mappable/dart_mappable.dart';
import 'package:equatable/equatable.dart';

part 'task.mapper.dart';

/// 任务状态
enum TaskStatus {
  /// 待执行
  pending,
  /// 运行中
  running,
  /// 已完成
  completed,
  /// 失败
  failed,
  /// 已取消
  cancelled,
}

/// 任务类型
enum TaskType {
  /// 导入文件夹
  importFolder,
  /// 刷新工作区
  refreshWorkspace,
  /// 搜索
  search,
  /// 导出
  export,
  /// 索引
  indexing,
}

/// 任务实体
@MappableClass()
class Task with TaskMappable, EquatableMixin {
  /// 任务 ID
  final String id;
  
  /// 任务类型
  final TaskType type;
  
  /// 任务状态
  final TaskStatus status;
  
  /// 进度（0-100）
  final double progress;
  
  /// 任务描述
  final String description;
  
  /// 关联的工作区 ID
  final String? workspaceId;
  
  /// 创建时间
  final DateTime createdAt;
  
  /// 更新时间
  final DateTime? updatedAt;
  
  /// 完成时间
  final DateTime? completedAt;
  
  /// 错误信息
  final String? errorMessage;
  
  /// 版本号（用于幂等性检查）
  final int version;
  
  /// 额外数据
  final Map<String, dynamic>? metadata;

  const Task({
    required this.id,
    required this.type,
    this.status = TaskStatus.pending,
    this.progress = 0.0,
    required this.description,
    this.workspaceId,
    required this.createdAt,
    this.updatedAt,
    this.completedAt,
    this.errorMessage,
    this.version = 1,
    this.metadata,
  });

  /// 空任务
  static const empty = Task(
    id: '',
    type: TaskType.importFolder,
    description: '',
    createdAt: DateTime.fromMillisecondsSinceEpoch(0),
  );

  /// 是否活跃
  bool get isActive => status == TaskStatus.pending || status == TaskStatus.running;

  /// 是否已完成
  bool get isDone => 
      status == TaskStatus.completed || 
      status == TaskStatus.failed || 
      status == TaskStatus.cancelled;

  /// 是否成功完成
  bool get isSuccess => status == TaskStatus.completed;

  /// 是否失败
  bool get isFailed => status == TaskStatus.failed || status == TaskStatus.cancelled;

  /// 格式化进度
  String get formattedProgress => '${progress.toStringAsFixed(1)}%';

  /// 运行时长（毫秒）
  int? get durationMs {
    if (completedAt == null) return null;
    return completedAt!.difference(createdAt).inMilliseconds;
  }

  /// 格式化运行时长
  String? get formattedDuration {
    final duration = durationMs;
    if (duration == null) return null;
    
    if (duration < 1000) {
      return '${duration}ms';
    } else if (duration < 60000) {
      return '${(duration / 1000).toStringAsFixed(1)}s';
    } else {
      final minutes = duration ~/ 60000;
      final seconds = (duration % 60000) ~/ 1000;
      return '${minutes}m ${seconds}s';
    }
  }

  /// 状态显示文本
  String get statusDisplayName {
    switch (status) {
      case TaskStatus.pending:
        return '等待中';
      case TaskStatus.running:
        return '运行中';
      case TaskStatus.completed:
        return '已完成';
      case TaskStatus.failed:
        return '失败';
      case TaskStatus.cancelled:
        return '已取消';
    }
  }

  /// 更新进度
  Task withProgress(double newProgress) {
    return copyWith(
      progress: newProgress.clamp(0.0, 100.0),
      updatedAt: DateTime.now(),
      version: version + 1,
    );
  }

  /// 标记为完成
  Task completed() {
    return copyWith(
      status: TaskStatus.completed,
      progress: 100.0,
      completedAt: DateTime.now(),
      updatedAt: DateTime.now(),
      version: version + 1,
    );
  }

  /// 标记为失败
  Task failed(String error) {
    return copyWith(
      status: TaskStatus.failed,
      errorMessage: error,
      completedAt: DateTime.now(),
      updatedAt: DateTime.now(),
      version: version + 1,
    );
  }

  /// 标记为取消
  Task cancelled() {
    return copyWith(
      status: TaskStatus.cancelled,
      completedAt: DateTime.now(),
      updatedAt: DateTime.now(),
      version: version + 1,
    );
  }

  @override
  List<Object?> get props => [
    id,
    type,
    status,
    progress,
    description,
    workspaceId,
    createdAt,
    updatedAt,
    completedAt,
    errorMessage,
    version,
  ];
}

/// 任务指标
@MappableClass()
class TaskMetrics with TaskMetricsMappable {
  /// 总任务数
  final int total;
  
  /// 运行中任务数
  final int running;
  
  /// 待执行任务数
  final int pending;
  
  /// 已完成任务数
  final int completed;
  
  /// 失败任务数
  final int failed;

  const TaskMetrics({
    this.total = 0,
    this.running = 0,
    this.pending = 0,
    this.completed = 0,
    this.failed = 0,
  });

  static const empty = TaskMetrics();

  /// 是否所有任务已完成
  bool get allCompleted => total > 0 && running == 0 && pending == 0;
}

/// 任务事件
/// 
/// 用于事件驱动的任务状态更新
sealed class TaskEvent {
  final String taskId;
  final DateTime timestamp;

  const TaskEvent({
    required this.taskId,
    required this.timestamp,
  });
}

/// 任务创建事件
class TaskCreatedEvent extends TaskEvent {
  final Task task;

  const TaskCreatedEvent({
    required super.taskId,
    required this.task,
    required super.timestamp,
  });
}

/// 任务进度更新事件
class TaskProgressEvent extends TaskEvent {
  final double progress;
  final String? message;

  const TaskProgressEvent({
    required super.taskId,
    required this.progress,
    this.message,
    required super.timestamp,
  });
}

/// 任务完成事件
class TaskCompletedEvent extends TaskEvent {
  final bool success;
  final String? error;
  final dynamic result;

  const TaskCompletedEvent({
    required super.taskId,
    required this.success,
    this.error,
    this.result,
    required super.timestamp,
  });
}

/// 任务取消事件
class TaskCancelledEvent extends TaskEvent {
  const TaskCancelledEvent({
    required super.taskId,
    required super.timestamp,
  });
}
