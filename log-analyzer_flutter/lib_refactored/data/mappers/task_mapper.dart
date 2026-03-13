/// 任务映射器
/// 
/// 负责 FFI 任务数据和 Domain 实体之间的转换

import '../../domain/entities/task.dart';
import '../../shared/services/generated/ffi/types.dart' as ffi_types;

/// 任务映射器
class TaskMapper {
  const TaskMapper._();

  /// FFI 类型转 Domain 实体
  static Task? fromFfi(ffi_types.TaskInfoData? data) {
    if (data == null) return null;

    return Task(
      id: data.taskId,
      type: _mapTaskType(data.taskType),
      status: _mapStatus(data.status),
      progress: data.progress ?? 0.0,
      description: data.description ?? '',
      workspaceId: data.workspaceId,
      createdAt: data.createdAt != null
          ? DateTime.parse(data.createdAt!)
          : DateTime.now(),
      updatedAt: data.updatedAt != null
          ? DateTime.parse(data.updatedAt!)
          : null,
      completedAt: data.completedAt != null
          ? DateTime.parse(data.completedAt!)
          : null,
      errorMessage: data.errorMessage,
      version: data.version ?? 1,
      metadata: data.metadata != null
          ? Map<String, dynamic>.from(data.metadata!)
          : null,
    );
  }

  /// FFI 任务列表转 Domain 列表
  static List<Task> fromFfiList(List<ffi_types.TaskInfoData> dataList) {
    return dataList
        .map(fromFfi)
        .whereType<Task>()
        .toList();
  }

  /// FFI 指标转 Domain 指标
  static TaskMetrics metricsFromFfi(ffi_types.TaskMetricsData? data) {
    if (data == null) return TaskMetrics.empty;

    return TaskMetrics(
      total: data.total ?? 0,
      running: data.running ?? 0,
      pending: data.pending ?? 0,
      completed: data.completed ?? 0,
      failed: data.failed ?? 0,
    );
  }

  /// 映射任务类型
  static TaskType _mapTaskType(String? type) {
    final value = type?.toLowerCase() ?? '';
    return switch (value) {
      'import_folder' => TaskType.importFolder,
      'refresh_workspace' => TaskType.refreshWorkspace,
      'search' => TaskType.search,
      'export' => TaskType.export,
      'indexing' => TaskType.indexing,
      _ => TaskType.importFolder,
    };
  }

  /// 映射任务状态
  static TaskStatus _mapStatus(ffi_types.TaskStatusData? status) {
    final value = status?.value?.toLowerCase() ?? 'pending';
    return switch (value) {
      'pending' => TaskStatus.pending,
      'running' => TaskStatus.running,
      'completed' => TaskStatus.completed,
      'failed' => TaskStatus.failed,
      'cancelled' => TaskStatus.cancelled,
      'stopped' => TaskStatus.cancelled,
      _ => TaskStatus.pending,
    };
  }

  /// Domain 状态转 FFI 状态
  static String toFfiStatus(TaskStatus status) {
    return switch (status) {
      TaskStatus.pending => 'pending',
      TaskStatus.running => 'running',
      TaskStatus.completed => 'completed',
      TaskStatus.failed => 'failed',
      TaskStatus.cancelled => 'cancelled',
    };
  }
}
