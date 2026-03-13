/// 工作区实体
/// 
/// 这是领域层的核心实体，与数据层解耦
/// 使用 dart_mappable 或 freezed 实现不可变性

import 'package:dart_mappable/dart_mappable.dart';
import 'package:equatable/equatable.dart';

part 'workspace.mapper.dart';

/// 工作区状态
enum WorkspaceStatus {
  /// 就绪
  ready,
  
  /// 扫描中
  scanning,
  
  /// 索引中
  indexing,
  
  /// 错误
  error,
  
  /// 未初始化
  uninitialized,
}

/// 工作区实体
/// 
/// 使用 dart_mappable 生成映射代码
@MappableClass()
class Workspace with WorkspaceMappable, EquatableMixin {
  /// 工作区 ID
  final String id;
  
  /// 工作区名称
  final String name;
  
  /// 工作区路径
  final String path;
  
  /// 状态
  final WorkspaceStatus status;
  
  /// 文件数量
  final int fileCount;
  
  /// 日志行数
  final int totalLogLines;
  
  /// 最后更新时间
  final DateTime? lastUpdatedAt;
  
  /// 创建时间
  final DateTime? createdAt;
  
  /// 是否正在监听文件变化
  final bool isWatching;
  
  /// 存储大小（字节）
  final int? storageSize;

  const Workspace({
    required this.id,
    required this.name,
    required this.path,
    this.status = WorkspaceStatus.uninitialized,
    this.fileCount = 0,
    this.totalLogLines = 0,
    this.lastUpdatedAt,
    this.createdAt,
    this.isWatching = false,
    this.storageSize,
  });

  /// 空工作区（用于初始化）
  static const empty = Workspace(
    id: '',
    name: '',
    path: '',
    status: WorkspaceStatus.uninitialized,
  );

  /// 是否有效
  bool get isValid => id.isNotEmpty && name.isNotEmpty;

  /// 是否就绪
  bool get isReady => status == WorkspaceStatus.ready;

  /// 是否忙
  bool get isBusy => status == WorkspaceStatus.scanning || 
                     status == WorkspaceStatus.indexing;

  /// 格式化文件数量
  String get formattedFileCount {
    if (fileCount >= 1000000) {
      return '${(fileCount / 1000000).toStringAsFixed(1)}M';
    } else if (fileCount >= 1000) {
      return '${(fileCount / 1000).toStringAsFixed(1)}K';
    }
    return fileCount.toString();
  }

  /// 格式化日志行数
  String get formattedLogLines {
    if (totalLogLines >= 1000000) {
      return '${(totalLogLines / 1000000).toStringAsFixed(1)}M';
    } else if (totalLogLines >= 1000) {
      return '${(totalLogLines / 1000).toStringAsFixed(1)}K';
    }
    return totalLogLines.toString();
  }

  /// 格式化存储大小
  String get formattedStorageSize {
    if (storageSize == null) return '-';
    
    final size = storageSize!;
    if (size >= 1024 * 1024 * 1024) {
      return '${(size / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
    } else if (size >= 1024 * 1024) {
      return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    } else if (size >= 1024) {
      return '${(size / 1024).toStringAsFixed(1)} KB';
    }
    return '$size B';
  }

  @override
  List<Object?> get props => [
    id,
    name,
    path,
    status,
    fileCount,
    totalLogLines,
    lastUpdatedAt,
    isWatching,
    storageSize,
  ];

  // 代码生成器会生成 copyWith、toString、toJson、fromJson 等方法
  static WorkspaceMapper ensureInitialized() => WorkspaceMapper.ensureInitialized();
}

/// 工作区统计
@MappableClass()
class WorkspaceStats with WorkspaceStatsMappable {
  /// 总文件数
  final int totalFiles;
  
  /// 总日志行数
  final int totalLogLines;
  
  /// 索引大小
  final int indexSize;
  
  /// 最后扫描时间
  final DateTime? lastScanAt;
  
  /// 错误数量
  final int errorCount;

  const WorkspaceStats({
    this.totalFiles = 0,
    this.totalLogLines = 0,
    this.indexSize = 0,
    this.lastScanAt,
    this.errorCount = 0,
  });

  static const empty = WorkspaceStats();
}

/// 创建/更新工作区的参数
@MappableClass()
class CreateWorkspaceParams with CreateWorkspaceParamsMappable {
  final String name;
  final String path;

  const CreateWorkspaceParams({
    required this.name,
    required this.path,
  });

  /// 验证参数
  String? validate() {
    if (name.trim().isEmpty) {
      return '工作区名称不能为空';
    }
    if (path.trim().isEmpty) {
      return '工作区路径不能为空';
    }
    return null;
  }

  bool get isValid => validate() == null;
}

/// 刷新工作区的参数
@MappableClass()
class RefreshWorkspaceParams with RefreshWorkspaceParamsMappable {
  final String workspaceId;
  final String path;

  const RefreshWorkspaceParams({
    required this.workspaceId,
    required this.path,
  });
}
