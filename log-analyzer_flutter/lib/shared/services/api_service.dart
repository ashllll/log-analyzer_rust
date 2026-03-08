import 'dart:convert';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:flutter/foundation.dart';

import '../models/common.dart';
import '../models/search.dart';
import 'bridge_service.dart';
import 'generated/ffi/types.dart';

part 'api_service.g.dart';

/// API 服务 Provider
///
/// 对应 React 版本的 api.ts
/// 封装所有后端 API 调用
@riverpod
ApiService apiService(Ref ref) {
  return ApiService();
}

/// 应用配置
class AppConfig {
  final FileFilterConfig? fileFilter;
  final Map<String, dynamic>? otherSettings;

  const AppConfig({this.fileFilter, this.otherSettings});

  factory AppConfig.fromJson(Map<String, dynamic> json) => AppConfig(
    fileFilter: json['file_filter'] != null
        ? FileFilterConfig.fromJson(json['file_filter'] as Map<String, dynamic>)
        : null,
    otherSettings: json['other_settings'] as Map<String, dynamic>?,
  );

  Map<String, dynamic> toJson() => {
    'file_filter': fileFilter?.toJson(),
    'other_settings': otherSettings,
  };
}

/// API 服务类
///
/// 使用 BridgeService 与 Rust 后端通信
/// 注意：字段名必须与 Rust 后端保持一致（snake_case）
class ApiService {
  final BridgeService _bridge = BridgeService.instance;

  /// FFI 桥接是否已初始化
  static bool _isInitialized = false;

  /// 初始化 FFI 桥接
  static Future<void> initialize() async {
    if (_isInitialized) return;
    try {
      await BridgeService.instance.initialize();
      _isInitialized = true;
    } catch (e) {
      debugPrint('API Service 初始化失败: $e');
    }
  }

  /// 检查 FFI 是否可用
  bool get isFfiAvailable => _bridge.isFfiEnabled && _isInitialized;

  /// 获取 BridgeService 实例
  ///
  /// 用于直接访问底层桥接服务
  BridgeService get bridge => _bridge;

  // ==================== 搜索操作 ====================

  /// 执行日志搜索
  ///
  /// 对应 Tauri 命令: search_logs
  /// 返回: search_id (String)
  Future<String> searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    SearchFilters? filters,
    FilterOptions? filterOptions,
  }) async {
    try {
      String? filtersJson;
      if (filters != null) {
        filtersJson = jsonEncode(filters.toJson());
      } else if (filterOptions != null) {
        // 将 FilterOptions 转换为 JSON
        filtersJson = jsonEncode(_filterOptionsToJson(filterOptions));
      }

      return await _bridge.searchLogs(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
        filters: filtersJson,
      );
    } catch (e) {
      throw ApiServiceException('搜索失败: $e');
    }
  }

  /// 取消搜索
  ///
  /// 对应 Tauri 命令: cancel_search
  Future<void> cancelSearch(String searchId) async {
    try {
      await _bridge.cancelSearch(searchId);
    } catch (e) {
      throw ApiServiceException('取消搜索失败: $e');
    }
  }

  /// 获取活跃搜索数量
  Future<int> getActiveSearchesCount() async {
    return _bridge.getActiveSearchesCount();
  }

  // ==================== 工作区操作 ====================

  /// 获取工作区列表
  Future<List<Workspace>> getWorkspaces() async {
    try {
      final data = await _bridge.getWorkspaces();
      return data.map((w) => _parseWorkspace(w)).toList();
    } catch (e) {
      debugPrint('获取工作区列表失败: $e');
      return [];
    }
  }

  /// 加载工作区
  ///
  /// 对应 Tauri 命令: load_workspace
  /// 返回: WorkspaceLoadResponse
  Future<WorkspaceLoadResponse> loadWorkspace(String workspaceId) async {
    try {
      final rawStatus = await _bridge.getWorkspaceStatus(workspaceId);
      if (rawStatus != null) {
        final status = rawStatus as Map<String, dynamic>;
        return WorkspaceLoadResponse(
          workspaceId: workspaceId,
          status: _parseStatusString(status['status'] as String? ?? ''),
          fileCount: status['file_count'] as int? ?? 0,
          totalSize: _formatSize(status['total_size'] as int? ?? 0),
        );
      }

      // 回退默认响应
      return WorkspaceLoadResponse(
        workspaceId: workspaceId,
        status: 'UNKNOWN',
        fileCount: 0,
        totalSize: '0 MB',
      );
    } catch (e) {
      throw ApiServiceException('加载工作区失败: $e');
    }
  }

  /// 创建工作区
  ///
  /// 对应 Tauri 命令: create_workspace
  /// 返回: workspace_id (String)
  Future<String> createWorkspace({
    required String name,
    required String path,
  }) async {
    try {
      return await _bridge.createWorkspace(name: name, path: path);
    } catch (e) {
      throw ApiServiceException('创建工作区失败: $e');
    }
  }

  /// 删除工作区
  ///
  /// 对应 Tauri 命令: delete_workspace
  Future<void> deleteWorkspace(String workspaceId) async {
    try {
      await _bridge.deleteWorkspace(workspaceId);
    } catch (e) {
      throw ApiServiceException('删除工作区失败: $e');
    }
  }

  /// 刷新工作区
  ///
  /// 对应 Tauri 命令: refresh_workspace
  /// 返回: task_id (String)
  Future<String> refreshWorkspace(String workspaceId, String path) async {
    try {
      return await _bridge.refreshWorkspace(workspaceId, path);
    } catch (e) {
      throw ApiServiceException('刷新工作区失败: $e');
    }
  }

  /// 获取工作区状态
  ///
  /// 对应 Tauri 命令: get_workspace_status
  /// 返回: WorkspaceStatusResponse
  Future<WorkspaceStatusResponse> getWorkspaceStatus(String workspaceId) async {
    try {
      final rawStatus = await _bridge.getWorkspaceStatus(workspaceId);
      if (rawStatus != null) {
        final status = rawStatus as Map<String, dynamic>;
        return WorkspaceStatusResponse(
          workspaceId: workspaceId,
          status: _parseStatusString(status['status'] as String? ?? ''),
          message: status['message'] as String?,
        );
      }

      return WorkspaceStatusResponse(
        workspaceId: workspaceId,
        status: 'UNKNOWN',
        message: null,
      );
    } catch (e) {
      throw ApiServiceException('获取工作区状态失败: $e');
    }
  }

  // ==================== 导入操作 ====================

  /// 导入文件夹
  ///
  /// 对应 Tauri 命令: import_folder
  /// 返回: task_id (String)
  Future<String> importFolder({
    required String path,
    required String workspaceId,
  }) async {
    try {
      return await _bridge.importFolder(path, workspaceId);
    } catch (e) {
      throw ApiServiceException('导入文件夹失败: $e');
    }
  }

  /// 检查 RAR 支持
  ///
  /// 对应 Tauri 命令: check_rar_support
  /// 返回: bool
  Future<bool> checkRarSupport() async {
    return _bridge.checkRarSupport();
  }

  // ==================== 压缩包导入操作 ====================

  /// 压缩包类型枚举
  static const List<String> supportedArchiveExtensions = [
    '.zip',
    '.tar',
    '.tar.gz',
    '.tgz',
    '.gz',
    '.rar',
    '.7z',
  ];

  /// 检测文件是否为压缩包
  static bool isArchiveFile(String path) {
    final lowerPath = path.toLowerCase();
    return lowerPath.endsWith('.zip') ||
        lowerPath.endsWith('.tar') ||
        lowerPath.endsWith('.tar.gz') ||
        lowerPath.endsWith('.tgz') ||
        lowerPath.endsWith('.gz') ||
        lowerPath.endsWith('.rar') ||
        lowerPath.endsWith('.7z');
  }

  /// 获取压缩包类型
  static ArchiveType? detectArchiveType(String path) {
    final lowerPath = path.toLowerCase();
    if (lowerPath.endsWith('.zip')) return ArchiveType.zip;
    if (lowerPath.endsWith('.tar') ||
        lowerPath.endsWith('.tar.gz') ||
        lowerPath.endsWith('.tgz')) {
      return ArchiveType.tar;
    }
    if (lowerPath.endsWith('.gz') && !lowerPath.endsWith('.tar.gz'))
      return ArchiveType.gzip;
    if (lowerPath.endsWith('.rar')) return ArchiveType.rar;
    if (lowerPath.endsWith('.7z')) return ArchiveType.sevenZ;
    return null;
  }

  /// 导入压缩包
  ///
  /// 使用 import_folder 命令导入压缩包（后端会自动处理解压）
  /// 对应 Tauri 命令: import_folder
  /// 返回: task_id (String)
  Future<String> importArchive({
    required String archivePath,
    required String workspaceId,
  }) async {
    try {
      // 使用现有的 import_folder 命令，后端会自动识别并处理压缩包
      return await _bridge.importFolder(archivePath, workspaceId);
    } catch (e) {
      throw ApiServiceException('导入压缩包失败: $e');
    }
  }

  /// 列出压缩包内容
  ///
  /// 对应 Tauri 命令: list_archive_contents
  Future<ArchiveContents> listArchiveContents(String archivePath) async {
    try {
      final result = await _bridge.listArchiveContents(archivePath);
      return ArchiveContents.fromJson(result);
    } catch (e) {
      debugPrint('listArchiveContents error: $e');
      // 返回空内容
      final type = detectArchiveType(archivePath);
      return ArchiveContents(
        type: type ?? ArchiveType.unknown,
        entries: [],
        totalSize: 0,
        fileCount: 0,
      );
    }
  }

  /// 读取压缩包内文件
  ///
  /// 对应 Tauri 命令: read_archive_file
  Future<ArchiveFileResult> readArchiveFile(
    String archivePath,
    String fileName,
  ) async {
    try {
      final result = await _bridge.readArchiveFile(archivePath, fileName);
      return ArchiveFileResult.fromJson(result);
    } catch (e) {
      debugPrint('readArchiveFile error: $e');
      rethrow;
    }
  }

  /// 选择性导入压缩包文件
  ///
  /// 注意：当前后端不支持选择性解压
  /// 导入全部内容
  Future<String> importArchiveFiles({
    required String archivePath,
    required String workspaceId,
    List<String>? selectedFiles,
  }) async {
    // 当前实现：导入全部文件
    // TODO: 后端实现选择性解压后使用 selectedFiles 参数
    return importArchive(archivePath: archivePath, workspaceId: workspaceId);
  }

  // ==================== 文件监听 ====================

  /// 启动文件监听
  ///
  /// 对应 Tauri 命令: start_watch
  Future<void> startWatch({
    required String workspaceId,
    required List<String> paths,
    bool recursive = true,
  }) async {
    try {
      await _bridge.startWatch(
        workspaceId: workspaceId,
        paths: paths,
        recursive: recursive,
      );
    } catch (e) {
      throw ApiServiceException('启动文件监听失败: $e');
    }
  }

  /// 停止文件监听
  ///
  /// 对应 Tauri 命令: stop_watch
  Future<void> stopWatch(String workspaceId) async {
    try {
      await _bridge.stopWatch(workspaceId);
    } catch (e) {
      throw ApiServiceException('停止文件监听失败: $e');
    }
  }

  /// 检查是否正在监听
  Future<bool> isWatching(String workspaceId) async {
    return _bridge.isWatching(workspaceId);
  }

  // ==================== 配置管理 ====================

  /// 保存配置
  ///
  /// 对应 Tauri 命令: save_config
  Future<void> saveConfig(AppConfig config) async {
    try {
      // 使用默认配置保存（简化实现）
      final defaultConfig = await ConfigData.default_();
      await _bridge.saveConfig(defaultConfig);
    } catch (e) {
      throw ApiServiceException('保存配置失败: $e');
    }
  }

  /// 加载配置
  ///
  /// 对应 Tauri 命令: load_config
  Future<AppConfig> loadConfig() async {
    try {
      final data = await _bridge.loadConfig();
      if (data != null) {
        // 从 FFI ConfigData 转换
        return AppConfig(
          fileFilter: FileFilterConfig(
            enabled: data.fileFilter.enabled,
            binaryDetectionEnabled: data.fileFilter.binaryDetectionEnabled,
            mode: data.fileFilter.mode,
            filenamePatterns: data.fileFilter.filenamePatterns,
            allowedExtensions: data.fileFilter.allowedExtensions,
            forbiddenExtensions: data.fileFilter.forbiddenExtensions,
          ),
        );
      }
      return const AppConfig();
    } catch (e) {
      throw ApiServiceException('加载配置失败: $e');
    }
  }

  // ==================== 导出操作 ====================

  /// 导出结果
  ///
  /// 对应 Tauri 命令: export_results
  /// 返回: 输出文件路径 (String)
  Future<String> exportResults({
    required String searchId,
    required String format, // 'csv' or 'json'
    required String outputPath,
  }) async {
    try {
      return await _bridge.exportResults(
        searchId: searchId,
        format: format,
        outputPath: outputPath,
      );
    } catch (e) {
      throw ApiServiceException('导出结果失败: $e');
    }
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  Future<TaskMetrics> getTaskMetrics() async {
    try {
      final data = await _bridge.getTaskMetrics();
      if (data != null) {
        return TaskMetrics.fromJson(data as Map<String, dynamic>);
      }
      return const TaskMetrics(total: 0, running: 0, completed: 0, failed: 0);
    } catch (e) {
      debugPrint('获取任务指标失败: $e');
      return const TaskMetrics(total: 0, running: 0, completed: 0, failed: 0);
    }
  }

  /// 取消任务
  ///
  /// 对应 Tauri 命令: cancel_task
  Future<void> cancelTask(String taskId) async {
    try {
      await _bridge.cancelTask(taskId);
    } catch (e) {
      throw ApiServiceException('取消任务失败: $e');
    }
  }

  // ==================== 查询操作 ====================

  /// 执行结构化查询
  ///
  /// 对应 Tauri 命令: execute_structured_query
  /// 返回: search_id (String)
  Future<String> executeStructuredQuery(SearchQuery query) async {
    try {
      return await _bridge.searchLogs(
        query: jsonEncode(query.toJson()),
        workspaceId: null,
        maxResults: 10000,
        filters: null,
      );
    } catch (e) {
      throw ApiServiceException('执行查询失败: $e');
    }
  }

  /// 验证查询
  ///
  /// 对应 Tauri 命令: validate_query
  /// 返回: QueryValidation
  Future<QueryValidation> validateQuery(SearchQuery query) async {
    try {
      // 简单的客户端验证
      final errors = <String>[];
      final warnings = <String>[];

      if (query.terms.isEmpty) {
        errors.add('查询不能为空');
      }

      final enabledTerms = query.terms.where((t) => t.enabled).toList();
      if (enabledTerms.isEmpty) {
        warnings.add('没有启用的搜索术语');
      }

      // 检查正则表达式语法
      for (final term in enabledTerms) {
        if (term.isRegex) {
          try {
            RegExp(term.value);
          } catch (e) {
            errors.add('无效的正则表达式: ${term.value}');
          }
        }
      }

      return QueryValidation(
        valid: errors.isEmpty,
        errors: errors.isEmpty ? null : errors,
        warnings: warnings.isEmpty ? null : warnings,
      );
    } catch (e) {
      throw ApiServiceException('验证查询失败: $e');
    }
  }

  // ==================== 性能监控 ====================

  /// 获取性能指标
  Future<PerformanceMetrics> getPerformanceMetrics(String timeRange) async {
    try {
      final data = await _bridge.getPerformanceMetrics(timeRange);
      if (data != null) {
        return PerformanceMetrics.fromJson(data as Map<String, dynamic>);
      }
      // 返回默认空指标
      return _emptyPerformanceMetrics();
    } catch (e) {
      debugPrint('获取性能指标失败: $e');
      return _emptyPerformanceMetrics();
    }
  }

  // ==================== 辅助方法 ====================

  /// 创建空的性能指标
  PerformanceMetrics _emptyPerformanceMetrics() {
    const emptyMetric = MetricData(current: 0, average: 0);
    const emptyCache = CacheMetrics(
      hitRate: 0,
      missCount: 0,
      hitCount: 0,
      size: 0,
    );
    const emptyMemory = MemoryMetrics(used: 0, total: 0);
    const emptyTask = TaskMetrics(
      total: 0,
      running: 0,
      completed: 0,
      failed: 0,
    );
    const emptyIndex = IndexMetrics(totalFiles: 0, indexedFiles: 0);

    return const PerformanceMetrics(
      searchLatency: emptyMetric,
      searchThroughput: emptyMetric,
      cacheMetrics: emptyCache,
      memoryMetrics: emptyMemory,
      taskMetrics: emptyTask,
      indexMetrics: emptyIndex,
    );
  }

  /// 将 FilterOptions 转换为 JSON Map
  Map<String, dynamic> _filterOptionsToJson(FilterOptions options) {
    return {
      'levels': options.levels,
      'time_range':
          options.timeRange.start != null || options.timeRange.end != null
          ? {'start': options.timeRange.start, 'end': options.timeRange.end}
          : null,
      'file_pattern': options.filePattern,
    };
  }

  /// 解析工作区数据
  Workspace _parseWorkspace(dynamic data) {
    if (data is Map<String, dynamic>) {
      return Workspace(
        id: data['id'] as String? ?? '',
        name: data['name'] as String? ?? '',
        path: data['path'] as String? ?? '',
        status: WorkspaceStatusData(value: _parseStatusString(data['status'])),
        size: _formatSize(data['total_size'] as int? ?? 0),
        files: data['file_count'] as int? ?? 0,
        watching: data['is_watching'] as bool?,
      );
    }
    // 返回默认空工作区
    return const Workspace(
      id: '',
      name: '',
      path: '',
      status: WorkspaceStatusData(value: 'UNKNOWN'),
      size: '0 MB',
      files: 0,
      watching: false,
    );
  }

  /// 解析状态字符串
  String _parseStatusString(dynamic status) {
    if (status == null) return 'UNKNOWN';
    if (status is String) return status;
    return 'UNKNOWN';
  }

  /// 格式化文件大小
  String _formatSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}

// ==================== 辅助类型 ====================

/// 工作区加载响应
class WorkspaceLoadResponse {
  final String workspaceId;
  final String status;
  final int fileCount;
  final String totalSize;

  const WorkspaceLoadResponse({
    required this.workspaceId,
    required this.status,
    required this.fileCount,
    required this.totalSize,
  });
}

/// 工作区状态响应
class WorkspaceStatusResponse {
  final String workspaceId;
  final String status;
  final String? message;

  const WorkspaceStatusResponse({
    required this.workspaceId,
    required this.status,
    this.message,
  });
}

/// 图表数据点
class ChartDataPoint {
  final DateTime timestamp;
  final double value;

  const ChartDataPoint({required this.timestamp, required this.value});

  factory ChartDataPoint.fromJson(Map<String, dynamic> json) => ChartDataPoint(
    timestamp: DateTime.parse(json['timestamp'] as String),
    value: (json['value'] as num).toDouble(),
  );
}

/// API 服务异常
class ApiServiceException implements Exception {
  final String message;
  final int? code;
  final String? help;

  const ApiServiceException(this.message, {this.code, this.help});

  @override
  String toString() => message;
}

// ==================== 压缩包相关类型 ====================

/// 压缩包类型枚举
enum ArchiveType { zip, tar, gzip, rar, sevenZ, unknown }

/// 压缩包条目
class ArchiveEntry {
  final String name;
  final String path;
  final int size;
  final bool isDirectory;
  final DateTime? modifiedTime;

  const ArchiveEntry({
    required this.name,
    required this.path,
    required this.size,
    required this.isDirectory,
    this.modifiedTime,
  });

  factory ArchiveEntry.fromJson(Map<String, dynamic> json) => ArchiveEntry(
    name: json['name'] as String? ?? '',
    path: json['path'] as String? ?? '',
    size: json['size'] as int? ?? 0,
    isDirectory: json['is_directory'] as bool? ?? false,
    modifiedTime: json['modified_time'] != null
        ? DateTime.tryParse(json['modified_time'] as String)
        : null,
  );

  Map<String, dynamic> toJson() => {
    'name': name,
    'path': path,
    'size': size,
    'is_directory': isDirectory,
    'modified_time': modifiedTime?.toIso8601String(),
  };
}

/// 压缩包内容
class ArchiveContents {
  final ArchiveType type;
  final List<ArchiveEntry> entries;
  final int totalSize;
  final int fileCount;

  const ArchiveContents({
    required this.type,
    required this.entries,
    required this.totalSize,
    required this.fileCount,
  });

  factory ArchiveContents.fromJson(Map<String, dynamic> json) =>
      ArchiveContents(
        type: ArchiveType.values.firstWhere(
          (e) => e.name == json['type'],
          orElse: () => ArchiveType.unknown,
        ),
        entries:
            (json['entries'] as List<dynamic>?)
                ?.map((e) => ArchiveEntry.fromJson(e as Map<String, dynamic>))
                .toList() ??
            [],
        totalSize: json['total_size'] as int? ?? 0,
        fileCount: json['file_count'] as int? ?? 0,
      );

  Map<String, dynamic> toJson() => {
    'type': type.name,
    'entries': entries.map((e) => e.toJson()).toList(),
    'total_size': totalSize,
    'file_count': fileCount,
  };
}

/// 压缩包文件读取结果
class ArchiveFileResult {
  final String content;
  final int size;
  final bool truncated;

  const ArchiveFileResult({
    required this.content,
    required this.size,
    required this.truncated,
  });

  factory ArchiveFileResult.fromJson(Map<String, dynamic> json) =>
      ArchiveFileResult(
        content: json['content'] as String? ?? '',
        size: json['size'] as int? ?? 0,
        truncated: json['truncated'] as bool? ?? false,
      );

  Map<String, dynamic> toJson() => {
    'content': content,
    'size': size,
    'truncated': truncated,
  };
}
