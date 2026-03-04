import 'package:freezed_annotation/freezed_annotation.dart';

part 'monitoring_state.freezed.dart';

/// 监控状态数据模型
///
/// 存储实时监控功能的所有状态信息
@freezed
abstract class MonitoringState with _$MonitoringState {
  const factory MonitoringState({
    /// 监控是否启用
    @Default(false) bool isActive,

    /// 已处理的事件数
    @Default(0) int eventsProcessed,

    /// 待处理队列数量
    @Default(0) int pendingCount,

    /// 监控的目录数
    @Default(0) int monitoredDirsCount,

    /// 监控的文件数
    @Default(0) int monitoredFilesCount,

    /// 最后更新时间
    DateTime? lastUpdate,

    /// 错误信息（如有）
    String? errorMessage,

    /// 当前工作区ID
    String? workspaceId,
  }) = _MonitoringState;
}
