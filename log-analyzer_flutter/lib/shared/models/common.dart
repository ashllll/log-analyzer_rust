import 'package:freezed_annotation/freezed_annotation.dart';

part 'common.freezed.dart';
part 'common.g.dart';

/// 日志条目模型
///
/// 对应 Rust 后端的 LogEntry 结构体
/// 对应 React 版本的 src/types/common.ts
@freezed
abstract class LogEntry with _$LogEntry {
  const factory LogEntry({
    required int id,
    required String timestamp,
    required String level,
    required String file,
    @JsonKey(name: 'real_path') required String realPath,
    required int line,
    required String content,
    required List<String> tags,
    @JsonKey(name: 'matched_keywords') List<String>? matchedKeywords,
  }) = _LogEntry;

  factory LogEntry.fromJson(Map<String, dynamic> json) =>
      _$LogEntryFromJson(json);
}

/// 工作区模型
///
/// 对应 Rust 后端的 Workspace 结构体
/// 对应 React 版本的 src/types/common.ts
@freezed
abstract class Workspace with _$Workspace {
  const factory Workspace({
    required String id,
    required String name,
    required String path,
    required WorkspaceStatusData status,
    required String size,
    required int files,
    bool? watching,
    DateTime? lastOpenedAt,
    DateTime? createdAt,
  }) = _Workspace;

  factory Workspace.fromJson(Map<String, dynamic> json) =>
      _$WorkspaceFromJson(json);
}

/// 工作区状态数据
@freezed
abstract class WorkspaceStatusData with _$WorkspaceStatusData {
  const factory WorkspaceStatusData({required String value}) =
      _WorkspaceStatusData;

  factory WorkspaceStatusData.fromJson(Map<String, dynamic> json) =>
      _$WorkspaceStatusDataFromJson(json);
}

/// 任务进度模型
///
/// 对应 Rust 后端的 TaskProgress 结构体
/// 对应 React 版本的 src/types/common.ts
@freezed
abstract class TaskProgress with _$TaskProgress {
  const factory TaskProgress({
    @JsonKey(name: 'task_id') required String taskId,
    @JsonKey(name: 'task_type') required String taskType,
    required String target,
    required String status,
    required String message,
    required int progress,
    @JsonKey(name: 'workspace_id') String? workspaceId,
  }) = _TaskProgress;

  factory TaskProgress.fromJson(Map<String, dynamic> json) =>
      _$TaskProgressFromJson(json);
}

/// 任务信息模型
///
/// 对应 Rust 后端的 TaskInfo 结构体
/// 对应 React 版本的 src/types/common.ts
@freezed
abstract class TaskInfo with _$TaskInfo {
  const factory TaskInfo({
    @JsonKey(name: 'task_id') required String taskId,
    @JsonKey(name: 'task_type') required String taskType,
    required String target,
    required int progress,
    required String message,
    required TaskStatusData status,
    required int version,
    @JsonKey(name: 'workspace_id') String? workspaceId,
  }) = _TaskInfo;

  factory TaskInfo.fromJson(Map<String, dynamic> json) =>
      _$TaskInfoFromJson(json);
}

/// 任务状态数据
@freezed
abstract class TaskStatusData with _$TaskStatusData {
  const factory TaskStatusData({required String value}) = _TaskStatusData;

  factory TaskStatusData.fromJson(Map<String, dynamic> json) =>
      _$TaskStatusDataFromJson(json);
}

/// 文件变化事件模型
///
/// 对应 Rust 后端的 FileChangeEvent 结构体
@freezed
abstract class FileChangeEvent with _$FileChangeEvent {
  const factory FileChangeEvent({
    @JsonKey(name: 'event_type') required String eventType,
    @JsonKey(name: 'file_path') required String filePath,
    @JsonKey(name: 'workspace_id') required String workspaceId,
    required int timestamp,
  }) = _FileChangeEvent;

  factory FileChangeEvent.fromJson(Map<String, dynamic> json) =>
      _$FileChangeEventFromJson(json);
}

/// 高级过滤器选项
///
/// 对应 React 版本的 FilterOptions
@freezed
abstract class FilterOptions with _$FilterOptions {
  const factory FilterOptions({
    required TimeRange timeRange,
    required List<String> levels,
    @JsonKey(name: 'file_pattern') String? filePattern,
  }) = _FilterOptions;

  factory FilterOptions.fromJson(Map<String, dynamic> json) =>
      _$FilterOptionsFromJson(json);
}

/// 时间范围
@freezed
abstract class TimeRange with _$TimeRange {
  const factory TimeRange({String? start, String? end}) = _TimeRange;

  factory TimeRange.fromJson(Map<String, dynamic> json) =>
      _$TimeRangeFromJson(json);
}

/// 文件过滤配置
///
/// 对应 React 版本的 FileFilterConfig
@freezed
abstract class FileFilterConfig with _$FileFilterConfig {
  const factory FileFilterConfig({
    required bool enabled,
    @JsonKey(name: 'binary_detection_enabled')
    required bool binaryDetectionEnabled,
    required FilterModeData mode,
    @JsonKey(name: 'filename_patterns') required List<String> filenamePatterns,
    @JsonKey(name: 'allowed_extensions')
    required List<String> allowedExtensions,
    @JsonKey(name: 'forbidden_extensions')
    required List<String> forbiddenExtensions,
  }) = _FileFilterConfig;

  factory FileFilterConfig.fromJson(Map<String, dynamic> json) =>
      _$FileFilterConfigFromJson(json);
}

/// 过滤器模式数据
@freezed
abstract class FilterModeData with _$FilterModeData {
  const factory FilterModeData({required String value}) = _FilterModeData;

  factory FilterModeData.fromJson(Map<String, dynamic> json) =>
      _$FilterModeDataFromJson(json);
}

/// 性能指标模型
///
/// 对应 React 版本的 PerformanceMetrics
@freezed
abstract class PerformanceMetrics with _$PerformanceMetrics {
  const factory PerformanceMetrics({
    @JsonKey(name: 'search_latency') required MetricData searchLatency,
    @JsonKey(name: 'search_throughput') required MetricData searchThroughput,
    @JsonKey(name: 'cache_metrics') required CacheMetrics cacheMetrics,
    @JsonKey(name: 'memory_metrics') required MemoryMetrics memoryMetrics,
    @JsonKey(name: 'task_metrics') required TaskMetrics taskMetrics,
    @JsonKey(name: 'index_metrics') required IndexMetrics indexMetrics,
  }) = _PerformanceMetrics;

  factory PerformanceMetrics.fromJson(Map<String, dynamic> json) =>
      _$PerformanceMetricsFromJson(json);
}

/// 指标数据（包含当前值、平均值、P95、P99）
@freezed
abstract class MetricData with _$MetricData {
  const factory MetricData({
    required double current,
    required double average,
    double? p95,
    double? p99,
    double? peak,
  }) = _MetricData;

  factory MetricData.fromJson(Map<String, dynamic> json) =>
      _$MetricDataFromJson(json);
}

/// 缓存指标
@freezed
abstract class CacheMetrics with _$CacheMetrics {
  const factory CacheMetrics({
    @JsonKey(name: 'hit_rate') required double hitRate,
    @JsonKey(name: 'miss_count') required int missCount,
    @JsonKey(name: 'hit_count') required int hitCount,
    required int size,
    int? capacity,
  }) = _CacheMetrics;

  factory CacheMetrics.fromJson(Map<String, dynamic> json) =>
      _$CacheMetricsFromJson(json);
}

/// 内存指标
@freezed
abstract class MemoryMetrics with _$MemoryMetrics {
  const factory MemoryMetrics({
    required double used,
    required double total,
    @JsonKey(name: 'heap_used') double? heapUsed,
    double? external,
  }) = _MemoryMetrics;

  factory MemoryMetrics.fromJson(Map<String, dynamic> json) =>
      _$MemoryMetricsFromJson(json);
}

/// 任务指标
@freezed
abstract class TaskMetrics with _$TaskMetrics {
  const factory TaskMetrics({
    required int total,
    required int running,
    required int completed,
    required int failed,
  }) = _TaskMetrics;

  factory TaskMetrics.fromJson(Map<String, dynamic> json) =>
      _$TaskMetricsFromJson(json);
}

/// 索引指标
@freezed
abstract class IndexMetrics with _$IndexMetrics {
  const factory IndexMetrics({
    @JsonKey(name: 'total_files') required int totalFiles,
    @JsonKey(name: 'indexed_files') required int indexedFiles,
  }) = _IndexMetrics;

  factory IndexMetrics.fromJson(Map<String, dynamic> json) =>
      _$IndexMetricsFromJson(json);
}
