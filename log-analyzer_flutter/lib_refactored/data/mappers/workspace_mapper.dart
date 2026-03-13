/// 工作区映射器
/// 
/// 负责 FFI 数据类型和 Domain 实体之间的转换

import '../../domain/entities/workspace.dart';
import '../../shared/services/generated/ffi/types.dart' as ffi_types;

/// 工作区映射器
class WorkspaceMapper {
  const WorkspaceMapper._();

  /// FFI 类型转 Domain 实体
  static Workspace fromFfi(ffi_types.WorkspaceData data) {
    return Workspace(
      id: data.id,
      name: data.name,
      path: data.path,
      status: _mapStatus(data.status),
      fileCount: data.fileCount,
      totalLogLines: data.totalLogLines,
      lastUpdatedAt: data.lastUpdatedAt != null
          ? DateTime.parse(data.lastUpdatedAt!)
          : null,
      createdAt: data.createdAt != null
          ? DateTime.parse(data.createdAt!)
          : null,
      isWatching: data.isWatching,
      storageSize: data.storageSize,
    );
  }

  /// FFI 状态列表转 Domain 实体列表
  static List<Workspace> fromFfiList(List<ffi_types.WorkspaceData> dataList) {
    return dataList.map(fromFfi).toList();
  }

  /// 映射工作区状态
  static WorkspaceStatus _mapStatus(ffi_types.WorkspaceStatusData? status) {
    final value = status?.value?.toLowerCase() ?? 'uninitialized';
    return switch (value) {
      'ready' => WorkspaceStatus.ready,
      'scanning' => WorkspaceStatus.scanning,
      'indexing' => WorkspaceStatus.indexing,
      'error' => WorkspaceStatus.error,
      _ => WorkspaceStatus.uninitialized,
    };
  }

  /// Domain 实体转 FFI 类型（创建参数）
  static Map<String, dynamic> toCreateParams(CreateWorkspaceParams params) {
    return {
      'name': params.name,
      'path': params.path,
    };
  }
}

/// 工作区统计映射器
class WorkspaceStatsMapper {
  const WorkspaceStatsMapper._();

  /// 从 FFI 数据解析
  static WorkspaceStats fromFfi(ffi_types.WorkspaceStatusData? status) {
    if (status == null) return WorkspaceStats.empty;

    return WorkspaceStats(
      totalFiles: status.fileCount ?? 0,
      totalLogLines: status.totalLogLines ?? 0,
      indexSize: 0, // FFI 可能没有提供
      lastScanAt: status.lastScanAt != null
          ? DateTime.parse(status.lastScanAt!)
          : null,
      errorCount: 0, // 需要额外获取
    );
  }

  /// 从 Map 解析（备用方案）
  static WorkspaceStats fromMap(Map<String, dynamic> map) {
    return WorkspaceStats(
      totalFiles: map['total_files'] as int? ?? 0,
      totalLogLines: map['total_log_lines'] as int? ?? 0,
      indexSize: map['index_size'] as int? ?? 0,
      lastScanAt: map['last_scan_at'] != null
          ? DateTime.parse(map['last_scan_at'] as String)
          : null,
      errorCount: map['error_count'] as int? ?? 0,
    );
  }
}
