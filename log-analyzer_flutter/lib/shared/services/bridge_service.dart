import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'generated/ffi/bridge.dart' as ffi;
import 'generated/frb_generated.dart';

/// FFI 初始化异常
class FfiInitializationException implements Exception {
  final String message;

  FfiInitializationException(this.message);

  @override
  String toString() => 'FFI InitializationException: $message';
}

/// 桥接服务
///
/// 使用 FFI 与 Rust 后端通信
/// 延迟加载 - 首次调用时初始化
class BridgeService {
  /// 单例模式
  static BridgeService? _instance;
  static bool _isInitialized = false;
  static bool _initFailed = false;
  static String? _initErrorMessage;

  BridgeService._();

  /// 获取单例实例
  static BridgeService get instance {
    _instance ??= BridgeService._();
    return _instance!;
  }

  /// 延迟初始化 FFI
  ///
  /// 在首次调用时初始化 FFI 桥接
  Future<void> initialize() async {
    if (_isInitialized) return;
    if (_initFailed) {
      throw FfiInitializationException(
          _initErrorMessage ?? 'FFI initialization previously failed');
    }

    try {
      await LogAnalyzerBridge.init();
      _isInitialized = true;
      debugPrint('FFI Bridge initialized successfully');
    } catch (e) {
      _initFailed = true;
      _initErrorMessage = e.toString();
      debugPrint('FFI Bridge initialization failed: $e');
      rethrow;
    }
  }

  /// 是否 FFI 已启用
  bool get isFfiEnabled => _isInitialized && !_initFailed;

  /// 是否已初始化
  bool get isInitialized => _isInitialized;

  /// 获取初始化错误消息
  String? get initErrorMessage => _initErrorMessage;

  /// 健康检查
  String checkHealth() {
    if (!isFfiEnabled) {
      return 'FFI_NOT_INITIALIZED';
    }
    try {
      // FFI 桥接已可用，返回 OK
      return ffi.healthCheck();
    } catch (e) {
      debugPrint('healthCheck error: $e');
      return 'ERROR';
    }
  }

  // ==================== 搜索操作 ====================

  /// 执行日志搜索
  ///
  /// 返回搜索 ID 用于获取结果
  Future<String> searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    String? filters,
  }) async {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      final result = ffi.searchLogs(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
        filters: filters,
      );

      if (result.ok) {
        return result.data;
      }
      throw Exception(result.error);
    } catch (e) {
      debugPrint('searchLogs error: $e');
      rethrow;
    }
  }

  /// 取消搜索
  Future<bool> cancelSearch(String searchId) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.cancelSearch(searchId: searchId);
      return result.ok;
    } catch (e) {
      debugPrint('cancelSearch error: $e');
      return false;
    }
  }

  /// 获取活跃搜索数量
  Future<int> getActiveSearchesCount() async {
    if (!isFfiEnabled) {
      return 0;
    }

    try {
      final result = ffi.getActiveSearchesCount();
      return result.data;
    } catch (e) {
      debugPrint('getActiveSearchesCount error: $e');
      return 0;
    }
  }

  // ==================== 工作区操作 ====================

  /// 获取工作区列表
  Future<List<ffi.WorkspaceData>> getWorkspaces() async {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.getWorkspaces();
    } catch (e) {
      debugPrint('getWorkspaces error: $e');
      return [];
    }
  }

  /// 创建工作区
  Future<String> createWorkspace({
    required String name,
    required String path,
  }) async {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      final result = ffi.createWorkspace(name: name, path: path);

      if (result.ok) {
        return result.data;
      }
      throw Exception(result.error);
    } catch (e) {
      debugPrint('createWorkspace error: $e');
      rethrow;
    }
  }

  /// 删除工作区
  Future<bool> deleteWorkspace(String workspaceId) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.deleteWorkspace(workspaceId: workspaceId);
      return result.ok;
    } catch (e) {
      debugPrint('deleteWorkspace error: $e');
      return false;
    }
  }

  /// 刷新工作区
  Future<String> refreshWorkspace(String workspaceId, String path) async {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      final result = ffi.refreshWorkspace(
        workspaceId: workspaceId,
        path: path,
      );

      if (result.ok) {
        return result.data;
      }
      throw Exception(result.error);
    } catch (e) {
      debugPrint('refreshWorkspace error: $e');
      rethrow;
    }
  }

  /// 获取工作区状态
  Future<ffi.WorkspaceStatusData?> getWorkspaceStatus(String workspaceId) async {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      final result = ffi.getWorkspaceStatus(workspaceId: workspaceId);
      if (result.ok) {
        return result.data;
      }
      return null;
    } catch (e) {
      debugPrint('getWorkspaceStatus error: $e');
      return null;
    }
  }

  // ==================== 关键词操作 ====================

  /// 获取关键词列表
  Future<List<ffi.KeywordGroupData>> getKeywords() async {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.getKeywords();
    } catch (e) {
      debugPrint('getKeywords error: $e');
      return [];
    }
  }

  /// 添加关键词组
  Future<bool> addKeywordGroup(ffi.KeywordGroupInput group) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.addKeywordGroup(group: group);
      return result.ok;
    } catch (e) {
      debugPrint('addKeywordGroup error: $e');
      return false;
    }
  }

  /// 更新关键词组
  Future<bool> updateKeywordGroup(
      String groupId, ffi.KeywordGroupInput group) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.updateKeywordGroup(groupId: groupId, group: group);
      return result.ok;
    } catch (e) {
      debugPrint('updateKeywordGroup error: $e');
      return false;
    }
  }

  /// 删除关键词组
  Future<bool> deleteKeywordGroup(String groupId) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.deleteKeywordGroup(groupId: groupId);
      return result.ok;
    } catch (e) {
      debugPrint('deleteKeywordGroup error: $e');
      return false;
    }
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  Future<ffi.TaskMetricsData?> getTaskMetrics() async {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      final result = ffi.getTaskMetrics();
      if (result.ok) {
        return result.data;
      }
      return null;
    } catch (e) {
      debugPrint('getTaskMetrics error: $e');
      return null;
    }
  }

  /// 取消任务
  Future<bool> cancelTask(String taskId) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.cancelTask(taskId: taskId);
      return result.ok;
    } catch (e) {
      debugPrint('cancelTask error: $e');
      return false;
    }
  }

  // ==================== 配置操作 ====================

  /// 加载配置
  Future<ConfigData?> loadConfig() async {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      return ffi.loadConfig();
    } catch (e) {
      debugPrint('loadConfig error: $e');
      return null;
    }
  }

  /// 保存配置
  Future<bool> saveConfig(ConfigData config) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.saveConfig(config: config);
      return result.ok;
    } catch (e) {
      debugPrint('saveConfig error: $e');
      return false;
    }
  }

  // ==================== 性能监控 ====================

  /// 获取性能指标
  Future<ffi.PerformanceMetricsData?> getPerformanceMetrics(String timeRange) async {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      return ffi.getPerformanceMetrics(timeRange: timeRange);
    } catch (e) {
      debugPrint('getPerformanceMetrics error: $e');
      return null;
    }
  }

  // ==================== 文件监听 ====================

  /// 启动文件监听
  Future<bool> startWatch({
    required String workspaceId,
    required List<String> paths,
    required bool recursive,
  }) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.startWatch(
        workspaceId: workspaceId,
        paths: paths,
        recursive: recursive,
      );
      return result.ok;
    } catch (e) {
      debugPrint('startWatch error: $e');
      return false;
    }
  }

  /// 停止文件监听
  Future<bool> stopWatch(String workspaceId) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.stopWatch(workspaceId: workspaceId);
      return result.ok;
    } catch (e) {
      debugPrint('stopWatch error: $e');
      return false;
    }
  }

  /// 检查是否正在监听
  Future<bool> isWatching(String workspaceId) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.isWatching(workspaceId: workspaceId);
      return result.ok;
    } catch (e) {
      debugPrint('isWatching error: $e');
      return false;
    }
  }

  // ==================== 导入操作 ====================

  /// 导入文件夹
  Future<String> importFolder(String path, String workspaceId) async {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      final result = ffi.importFolder(
        path: path,
        workspaceId: workspaceId,
      );

      if (result.ok) {
        return result.data;
      }
      throw Exception(result.error);
    } catch (e) {
      debugPrint('importFolder error: $e');
      rethrow;
    }
  }

  /// 检查 RAR 支持
  Future<bool> checkRarSupport() async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.checkRarSupport();
    } catch (e) {
      debugPrint('checkRarSupport error: $e');
      return false;
    }
  }

  // ==================== 压缩包浏览操作 ====================

  /// 列出压缩包内容
  ///
  /// 使用 Tauri invoke 调用后端命令
  Future<Map<String, dynamic>> listArchiveContents(String archivePath) async {
    try {
      const channel = MethodChannel('com.joeash.log-analyzer/commands');
      final result = await channel.invokeMethod('list_archive_contents', {
        'archivePath': archivePath,
      });
      return Map<String, dynamic>.from(result as Map);
    } catch (e) {
      debugPrint('listArchiveContents error: $e');
      rethrow;
    }
  }

  /// 读取压缩包内文件
  ///
  /// 使用 Tauri invoke 调用后端命令
  Future<Map<String, dynamic>> readArchiveFile(
    String archivePath,
    String fileName,
  ) async {
    try {
      const channel = MethodChannel('com.joeash.log-analyzer/commands');
      final result = await channel.invokeMethod('read_archive_file', {
        'archivePath': archivePath,
        'fileName': fileName,
      });
      return Map<String, dynamic>.from(result as Map);
    } catch (e) {
      debugPrint('readArchiveFile error: $e');
      rethrow;
    }
  }

  // ==================== 导出操作 ====================

  /// 导出搜索结果
  Future<String> exportResults({
    required String searchId,
    required String format,
    required String outputPath,
  }) async {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      final result = ffi.exportResults(
        searchId: searchId,
        format: format,
        outputPath: outputPath,
      );

      if (result.ok) {
        return result.data;
      }
      throw Exception(result.error);
    } catch (e) {
      debugPrint('exportResults error: $e');
      rethrow;
    }
  }

  // ==================== 搜索历史操作 ====================

  /// 添加搜索历史记录
  ///
  /// 将搜索查询添加到历史记录
  Future<bool> addSearchHistory({
    required String query,
    required String workspaceId,
    required int resultCount,
  }) async {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      final result = ffi.addSearchHistory(
        query: query,
        workspaceId: workspaceId,
        resultCount: resultCount,
      );
      return result;
    } catch (e) {
      debugPrint('addSearchHistory error: $e');
      rethrow;
    }
  }

  /// 获取搜索历史记录
  ///
  /// 获取指定工作区或所有工作区的搜索历史
  Future<List<ffi.SearchHistoryData>> getSearchHistory({
    String? workspaceId,
    int? limit,
  }) async {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.getSearchHistory(
        workspaceId: workspaceId,
        limit: limit,
      );
    } catch (e) {
      debugPrint('getSearchHistory error: $e');
      return [];
    }
  }

  /// 删除搜索历史记录
  ///
  /// 删除指定工作区中特定查询的历史记录
  Future<bool> deleteSearchHistory({
    required String query,
    required String workspaceId,
  }) async {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      final result = ffi.deleteSearchHistory(
        query: query,
        workspaceId: workspaceId,
      );
      return result;
    } catch (e) {
      debugPrint('deleteSearchHistory error: $e');
      return false;
    }
  }

  /// 批量删除搜索历史记录
  ///
  /// 批量删除指定工作区中多个查询的历史记录
  Future<int> deleteSearchHistories({
    required List<String> queries,
    required String workspaceId,
  }) async {
    if (!isFfiEnabled) {
      return 0;
    }

    try {
      final result = ffi.deleteSearchHistories(
        queries: queries,
        workspaceId: workspaceId,
      );
      return result;
    } catch (e) {
      debugPrint('deleteSearchHistories error: $e');
      return 0;
    }
  }

  /// 清空搜索历史
  ///
  /// 清空指定工作区或所有工作区的搜索历史
  Future<int> clearSearchHistory({String? workspaceId}) async {
    if (!isFfiEnabled) {
      return 0;
    }

    try {
      final result = ffi.clearSearchHistory(workspaceId: workspaceId);
      return result;
    } catch (e) {
      debugPrint('clearSearchHistory error: $e');
      return 0;
    }
  }

  /// 释放资源
  void dispose() {
    LogAnalyzerBridge.dispose();
    _isInitialized = false;
    _initFailed = false;
    _initErrorMessage = null;
    _instance = null;
  }
}

// ==================== 类型定义 ====================

/// 图表时间范围（用于性能指标查询）
enum ChartTimeRange {
  minutes1,
  minutes5,
  minutes15,
  hour1,
}

/// 桥接异常
class BridgeException implements Exception {
  final String message;

  const BridgeException(this.message);

  @override
  String toString() => 'BridgeException: $message';
}

/// HTTP 未实现异常
class BridgeNotImplementedException implements Exception {
  final String message;

  BridgeNotImplementedException(this.message);

  @override
  String toString() => 'Bridge not implemented: $message';
}
