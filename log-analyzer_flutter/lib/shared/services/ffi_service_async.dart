/// FFI 异步服务封装
///
/// 提供安全、高效的异步 FFI 调用封装，遵循 Flutter 最佳实践。
///
/// ## 特性
///
/// - **异步优先**: 所有操作使用 async/await，避免阻塞 UI
/// - **错误处理**: 统一异常处理，将 Rust 错误转换为 Dart 异常
/// - **超时控制**: 内置超时机制，防止无限等待
/// - **取消支持**: 支持取消长时间运行的任务
/// - **Isolate 支持**: 在 Isolate 中执行密集计算，避免阻塞主线程
///
/// ## 使用方式
///
/// ```dart
/// final ffiService = FfiServiceAsync();
///
/// // 创建工作区
/// final workspaceId = await ffiService.createWorkspace('my_workspace', '/path/to/logs');
///
/// // 搜索日志
/// final searchId = await ffiService.searchLogs('ERROR', workspaceId: workspaceId);
/// ```
library;

import 'dart:async';
import 'dart:isolate';

import 'package:flutter/foundation.dart';
import 'package:log_analyzer_flutter/shared/services/generated/bridge_generated.dart';

/// FFI 错误类型
class FfiException implements Exception {
  /// 错误代码
  final String code;

  /// 错误消息
  final String message;

  /// 详细错误信息
  final String? details;

  /// 错误发生位置
  final String? location;

  FfiException({
    required this.code,
    required this.message,
    this.details,
    this.location,
  });

  @override
  String toString() => 'FfiException[$code]: $message';
}

/// FFI 服务配置
class FfiServiceConfig {
  /// 默认操作超时
  final Duration defaultTimeout;

  /// 是否启用 Isolate
  final bool useIsolate;

  /// 是否启用详细日志
  final bool verboseLogging;

  const FfiServiceConfig({
    this.defaultTimeout = const Duration(seconds: 30),
    this.useIsolate = true,
    this.verboseLogging = false,
  });

  static const FfiServiceConfig defaultConfig = FfiServiceConfig();
}

/// FFI 异步服务
///
/// 封装 flutter_rust_bridge 生成的 API，提供高级错误处理和异步支持
class FfiServiceAsync {
  static FfiServiceAsync? _instance;
  final LogAnalyzerApi _api;
  final FfiServiceConfig _config;

  FfiServiceAsync._(this._api, this._config);

  /// 获取单例实例
  ///
  /// 在应用启动时初始化：
  /// ```dart
  /// final api = await NativeCodeLoader.load();
  /// FfiServiceAsync.initialize(api);
  /// ```
  static FfiServiceAsync get instance {
    if (_instance == null) {
      throw StateError(
        'FfiServiceAsync 未初始化。请先调用 FfiServiceAsync.initialize()',
      );
    }
    return _instance!;
  }

  /// 初始化服务
  static void initialize(
    LogAnalyzerApi api, {
    FfiServiceConfig config = FfiServiceConfig.defaultConfig,
  }) {
    _instance = FfiServiceAsync._(api, config);
  }

  // ==================== 私有工具方法 ====================

  /// 包装异步调用，添加超时和错误处理
  Future<T> _wrapCall<T>(
    Future<T> Function() call, {
    Duration? timeout,
    String? operation,
  }) async {
    final effectiveTimeout = timeout ?? _config.defaultTimeout;

    if (_config.verboseLogging) {
      debugPrint('FFI Call: $operation (timeout: ${effectiveTimeout.inSeconds}s)');
    }

    try {
      final result = await call().timeout(
        effectiveTimeout,
        onTimeout: () => throw FfiException(
          code: 'TIMEOUT',
          message: '操作超时: $operation',
        ),
      );

      if (_config.verboseLogging) {
        debugPrint('FFI Success: $operation');
      }

      return result;
    } on FfiException {
      rethrow;
    } catch (e, stackTrace) {
      // 解析 Rust 错误消息
      final errorStr = e.toString();
      final match = RegExp(r'\[(\w+)\]\s*(.+)').firstMatch(errorStr);

      if (match != null) {
        throw FfiException(
          code: match.group(1) ?? 'UNKNOWN',
          message: match.group(2) ?? errorStr,
        );
      }

      if (_config.verboseLogging) {
        debugPrint('FFI Error: $operation - $e');
        debugPrint(stackTrace.toString());
      }

      throw FfiException(
        code: 'UNKNOWN',
        message: errorStr,
      );
    }
  }

  /// 在 Isolate 中执行密集型操作
  Future<T> _runInIsolate<T, P>(
    P parameter,
    Future<T> Function(P param) worker, {
    Duration? timeout,
  }) async {
    if (!_config.useIsolate) {
      return worker(parameter);
    }

    return compute(
      (msg) async {
        // 注意：在 Isolate 中无法直接访问 _api，需要重新初始化或使用消息传递
        // 这里简化处理，实际使用时需要根据具体情况调整
        return worker(msg as P);
      },
      parameter,
    );
  }

  // ==================== 系统操作 ====================

  /// 健康检查
  Future<String> healthCheck() async {
    return _wrapCall(
      () => Future.value(_api.healthCheck()),
      operation: 'healthCheck',
    );
  }

  /// 运行时健康检查（异步）
  Future<String> runtimeHealthCheck() async {
    return _wrapCall(
      () => _api.runtimeHealthCheckAsync(),
      operation: 'runtimeHealthCheck',
    );
  }

  /// 获取系统信息
  SystemInfoData getSystemInfo() {
    return _api.getSystemInfo();
  }

  // ==================== 工作区操作 ====================

  /// 获取工作区列表
  Future<List<WorkspaceData>> getWorkspaces() async {
    return _wrapCall(
      () => _api.getWorkspaces(),
      operation: 'getWorkspaces',
    );
  }

  /// 创建工作区
  Future<String> createWorkspace(String name, String path) async {
    return _wrapCall(
      () => _api.createWorkspace(name: name, path: path),
      operation: 'createWorkspace',
    );
  }

  /// 删除工作区
  Future<bool> deleteWorkspace(String workspaceId) async {
    return _wrapCall(
      () => _api.deleteWorkspace(workspaceId: workspaceId),
      operation: 'deleteWorkspace',
    );
  }

  /// 刷新工作区
  Future<String> refreshWorkspace(String workspaceId, String path) async {
    return _wrapCall(
      () => _api.refreshWorkspace(workspaceId: workspaceId, path: path),
      operation: 'refreshWorkspace',
    );
  }

  /// 获取工作区状态
  Future<WorkspaceStatusData> getWorkspaceStatus(String workspaceId) async {
    return _wrapCall(
      () => _api.getWorkspaceStatus(workspaceId: workspaceId),
      operation: 'getWorkspaceStatus',
    );
  }

  // ==================== 搜索操作 ====================

  /// 搜索日志
  ///
  /// [query] 搜索关键词
  /// [workspaceId] 工作区 ID（可选）
  /// [maxResults] 最大结果数量
  /// [filters] 过滤器 JSON 字符串（可选）
  Future<String> searchLogs(
    String query, {
    String? workspaceId,
    int maxResults = 1000,
    String? filters,
  }) async {
    return _wrapCall(
      () => _api.searchLogs(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
        filters: filters,
      ),
      operation: 'searchLogs',
    );
  }

  /// 取消搜索
  Future<bool> cancelSearch(String searchId) async {
    return _wrapCall(
      () => _api.cancelSearch(searchId: searchId),
      operation: 'cancelSearch',
    );
  }

  /// 获取活跃搜索数量
  int getActiveSearchesCount() {
    return _api.getActiveSearchesCount();
  }

  /// 正则搜索
  Future<List<FfiSearchResultEntry>> searchRegex(
    String pattern, {
    String? workspaceId,
    int maxResults = 1000,
    bool caseSensitive = false,
  }) async {
    return _wrapCall(
      () => _api.searchRegex(
        pattern: pattern,
        workspaceId: workspaceId,
        maxResults: maxResults,
        caseSensitive: caseSensitive,
      ),
      operation: 'searchRegex',
    );
  }

  /// 结构化搜索
  Future<List<FfiSearchResultEntry>> searchStructured(
    StructuredSearchQueryData query, {
    String? workspaceId,
    int maxResults = 1000,
  }) async {
    return _wrapCall(
      () => _api.searchStructured(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
      ),
      operation: 'searchStructured',
    );
  }

  /// 构建搜索查询
  StructuredSearchQueryData buildSearchQuery({
    required List<String> keywords,
    String globalOperator = 'AND',
    bool isRegex = false,
    bool caseSensitive = false,
  }) {
    return _api.buildSearchQuery(
      keywords: keywords,
      globalOperator: globalOperator,
      isRegex: isRegex,
      caseSensitive: caseSensitive,
    );
  }

  /// 验证正则表达式
  RegexValidationResult validateRegex(String pattern) {
    return _api.validateRegex(pattern: pattern);
  }

  // ==================== 关键词操作 ====================

  /// 获取关键词列表
  Future<List<FfiKeywordGroupData>> getKeywords() async {
    return _wrapCall(
      () => _api.getKeywords(),
      operation: 'getKeywords',
    );
  }

  /// 添加关键词组
  Future<bool> addKeywordGroup(KeywordGroupInput group) async {
    return _wrapCall(
      () => _api.addKeywordGroup(group: group),
      operation: 'addKeywordGroup',
    );
  }

  /// 更新关键词组
  Future<bool> updateKeywordGroup(
    String groupId,
    KeywordGroupInput group,
  ) async {
    return _wrapCall(
      () => _api.updateKeywordGroup(groupId: groupId, group: group),
      operation: 'updateKeywordGroup',
    );
  }

  /// 删除关键词组
  Future<bool> deleteKeywordGroup(String groupId) async {
    return _wrapCall(
      () => _api.deleteKeywordGroup(groupId: groupId),
      operation: 'deleteKeywordGroup',
    );
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  Future<TaskMetricsData> getTaskMetrics() async {
    return _wrapCall(
      () => _api.getTaskMetrics(),
      operation: 'getTaskMetrics',
    );
  }

  /// 取消任务
  Future<bool> cancelTask(String taskId) async {
    return _wrapCall(
      () => _api.cancelTask(taskId: taskId),
      operation: 'cancelTask',
    );
  }

  // ==================== 配置操作 ====================

  /// 加载配置
  Future<ConfigData> loadConfig() async {
    return _wrapCall(
      () => _api.loadConfig(),
      operation: 'loadConfig',
    );
  }

  /// 保存配置
  Future<bool> saveConfig(ConfigData config) async {
    return _wrapCall(
      () => _api.saveConfig(config: config),
      operation: 'saveConfig',
    );
  }

  // ==================== 性能监控 ====================

  /// 获取性能指标
  Future<PerformanceMetricsData> getPerformanceMetrics(String timeRange) async {
    return _wrapCall(
      () => _api.getPerformanceMetrics(timeRange: timeRange),
      operation: 'getPerformanceMetrics',
    );
  }

  // ==================== 文件监听 ====================

  /// 启动文件监听
  Future<bool> startWatch(
    String workspaceId,
    List<String> paths, {
    bool recursive = true,
  }) async {
    return _wrapCall(
      () => _api.startWatch(
        workspaceId: workspaceId,
        paths: paths,
        recursive: recursive,
      ),
      operation: 'startWatch',
    );
  }

  /// 停止文件监听
  Future<bool> stopWatch(String workspaceId) async {
    return _wrapCall(
      () => _api.stopWatch(workspaceId: workspaceId),
      operation: 'stopWatch',
    );
  }

  /// 检查是否正在监听
  bool isWatching(String workspaceId) {
    return _api.isWatching(workspaceId: workspaceId);
  }

  // ==================== 导入/导出操作 ====================

  /// 导入文件夹
  Future<String> importFolder(String path, String workspaceId) async {
    return _wrapCall(
      () => _api.importFolder(path: path, workspaceId: workspaceId),
      operation: 'importFolder',
    );
  }

  /// 检查 RAR 支持
  bool checkRarSupport() {
    return _api.checkRarSupport();
  }

  /// 导出搜索结果
  Future<String> exportResults(
    String searchId,
    String format,
    String outputPath,
  ) async {
    return _wrapCall(
      () => _api.exportResults(
        searchId: searchId,
        format: format,
        outputPath: outputPath,
      ),
      operation: 'exportResults',
    );
  }

  // ==================== 搜索历史操作 ====================

  /// 添加搜索历史
  Future<bool> addSearchHistory(
    String query,
    String workspaceId,
    int resultCount,
  ) async {
    return _wrapCall(
      () => _api.addSearchHistory(
        query: query,
        workspaceId: workspaceId,
        resultCount: resultCount,
      ),
      operation: 'addSearchHistory',
    );
  }

  /// 获取搜索历史
  Future<List<SearchHistoryData>> getSearchHistory({
    String? workspaceId,
    int? limit,
  }) async {
    return _wrapCall(
      () => _api.getSearchHistory(
        workspaceId: workspaceId,
        limit: limit,
      ),
      operation: 'getSearchHistory',
    );
  }

  /// 删除搜索历史
  Future<bool> deleteSearchHistory(String query, String workspaceId) async {
    return _wrapCall(
      () => _api.deleteSearchHistory(
        query: query,
        workspaceId: workspaceId,
      ),
      operation: 'deleteSearchHistory',
    );
  }

  /// 批量删除搜索历史
  Future<int> deleteSearchHistories(
    List<String> queries,
    String workspaceId,
  ) async {
    return _wrapCall(
      () => _api.deleteSearchHistories(
        queries: queries,
        workspaceId: workspaceId,
      ),
      operation: 'deleteSearchHistories',
    );
  }

  /// 清空搜索历史
  Future<int> clearSearchHistory({String? workspaceId}) async {
    return _wrapCall(
      () => _api.clearSearchHistory(workspaceId: workspaceId),
      operation: 'clearSearchHistory',
    );
  }

  // ==================== 虚拟文件树操作 ====================

  /// 获取虚拟文件树
  Future<List<VirtualTreeNodeData>> getVirtualFileTree(String workspaceId) async {
    return _wrapCall(
      () => _api.getVirtualFileTree(workspaceId: workspaceId),
      operation: 'getVirtualFileTree',
    );
  }

  /// 获取树子节点
  Future<List<VirtualTreeNodeData>> getTreeChildren(
    String workspaceId,
    String parentPath,
  ) async {
    return _wrapCall(
      () => _api.getTreeChildren(
        workspaceId: workspaceId,
        parentPath: parentPath,
      ),
      operation: 'getTreeChildren',
    );
  }

  /// 通过哈希读取文件
  Future<FileContentResponseData> readFileByHash(
    String workspaceId,
    String hash,
  ) async {
    return _wrapCall(
      () => _api.readFileByHash(
        workspaceId: workspaceId,
        hash: hash,
      ),
      operation: 'readFileByHash',
    );
  }

  // ==================== 过滤器操作 ====================

  /// 保存过滤器
  Future<bool> saveFilter(SavedFilterInput filter) async {
    return _wrapCall(
      () => _api.saveFilter(filter: filter),
      operation: 'saveFilter',
    );
  }

  /// 获取过滤器列表
  Future<List<SavedFilterData>> getSavedFilters(
    String workspaceId, {
    int? limit,
  }) async {
    return _wrapCall(
      () => _api.getSavedFilters(
        workspaceId: workspaceId,
        limit: limit,
      ),
      operation: 'getSavedFilters',
    );
  }

  /// 删除过滤器
  Future<bool> deleteFilter(String filterId, String workspaceId) async {
    return _wrapCall(
      () => _api.deleteFilter(
        filterId: filterId,
        workspaceId: workspaceId,
      ),
      operation: 'deleteFilter',
    );
  }

  /// 更新过滤器使用统计
  Future<bool> updateFilterUsage(String filterId, String workspaceId) async {
    return _wrapCall(
      () => _api.updateFilterUsage(
        filterId: filterId,
        workspaceId: workspaceId,
      ),
      operation: 'updateFilterUsage',
    );
  }

  // ==================== 日志级别统计 ====================

  /// 获取日志级别统计
  Future<LogLevelStatsOutput> getLogLevelStats(String workspaceId) async {
    return _wrapCall(
      () => _api.getLogLevelStats(workspaceId: workspaceId),
      operation: 'getLogLevelStats',
    );
  }

  // ==================== Session 操作 ====================

  /// 打开 Session
  Future<SessionInfo> openSession(String path) async {
    return _wrapCall(
      () => _api.openSession(path: path),
      operation: 'openSession',
    );
  }

  /// 获取 Session 信息
  Future<SessionInfo> getSessionInfo(String sessionId) async {
    return _wrapCall(
      () => _api.getSessionInfo(sessionId: sessionId),
      operation: 'getSessionInfo',
    );
  }

  /// 关闭 Session
  Future<bool> closeSession(String sessionId) async {
    return _wrapCall(
      () => _api.closeSession(sessionId: sessionId),
      operation: 'closeSession',
    );
  }

  /// 获取所有 Session
  Future<List<String>> getAllSessions() async {
    return _wrapCall(
      () => _api.getAllSessions(),
      operation: 'getAllSessions',
    );
  }
}

// ==================== Isolate 工作函数 ====================

/// Isolate 搜索参数
class _SearchIsolateParams {
  final String query;
  final String? workspaceId;
  final int maxResults;

  _SearchIsolateParams({
    required this.query,
    this.workspaceId,
    required this.maxResults,
  });
}

/// Isolate 搜索结果
class _SearchIsolateResult {
  final List<FfiSearchResultEntry> results;
  final String? error;

  _SearchIsolateResult({required this.results, this.error});
}

/// 在 Isolate 中执行搜索
///
/// 注意：由于 flutter_rust_bridge 不支持跨 Isolate 共享 API 实例，
/// 此函数需要特殊处理。实际使用时可能需要在 Isolate 中重新初始化 API。
Future<List<FfiSearchResultEntry>> _searchInIsolate(
  _SearchIsolateParams params,
) async {
  // 这里需要在 Isolate 中重新初始化 API
  // 简化示例，实际实现需要处理复杂情况
  throw UnimplementedError('Isolate 搜索需要特殊处理');
}
