import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated_io.dart';

import '../models/saved_filter.dart';
import 'error_handler.dart';
import 'generated/ffi/bridge.dart' as ffi;
import 'generated/ffi/types.dart' as ffi_types;
import 'generated/frb_generated.dart';

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
        _initErrorMessage ?? 'FFI initialization previously failed',
      );
    }

    try {
      // Calculate the absolute path to the Rust library
      final externalLibrary = await _resolveExternalLibrary();

      await LogAnalyzerBridge.init(externalLibrary: externalLibrary);
      _isInitialized = true;
      debugPrint('FFI Bridge initialized successfully');
    } catch (e) {
      _initFailed = true;
      _initErrorMessage = e.toString();
      debugPrint('FFI Bridge initialization failed: $e');
      rethrow;
    }
  }

  /// Resolve the external library path
  ///
  /// On macOS, load the dylib directly from the Rust target directory
  Future<ExternalLibrary> _resolveExternalLibrary() async {
    if (!Platform.isMacOS && !Platform.isLinux && !Platform.isWindows) {
      // For other platforms (mobile, web), use default loader
      throw UnsupportedError(
        'Platform not supported for custom library loading',
      );
    }

    // Try common locations for the Rust library
    final possiblePaths = _getLibrarySearchPaths();

    for (final path in possiblePaths) {
      final file = File(path);
      if (await file.exists()) {
        debugPrint('Loading Rust library from: $path');
        return ExternalLibrary.open(path);
      }
    }

    // If not found in any location, throw an error with helpful message
    throw FileSystemException(
      'Could not find Rust library. Searched paths:\n${possiblePaths.join('\n')}',
    );
  }

  /// Get possible library paths based on platform
  List<String> _getLibrarySearchPaths() {
    // Get the project root directory (assuming we're running from the Flutter project)
    final scriptDir = Platform.script.toFilePath();
    final projectRoot = _findProjectRoot(scriptDir);

    if (projectRoot == null) {
      debugPrint('Could not determine project root from: $scriptDir');
    }

    final paths = <String>[];

    if (Platform.isMacOS) {
      const libraryName = 'liblog_analyzer.dylib';
      paths.addAll([
        // Relative from Flutter build directory
        '../log-analyzer/src-tauri/target/release/$libraryName',
        '../log-analyzer/src-tauri/target/debug/$libraryName',
        // From project root
        if (projectRoot != null) ...[
          '$projectRoot/../log-analyzer/src-tauri/target/release/$libraryName',
          '$projectRoot/../log-analyzer/src-tauri/target/debug/$libraryName',
        ],
        // Absolute path (fallback for development)
        '/Users/joeash/code/github/log-analyzer_rust/log-analyzer/src-tauri/target/release/$libraryName',
      ]);
    } else if (Platform.isLinux) {
      const libraryName = 'liblog_analyzer.so';
      paths.addAll([
        '../log-analyzer/src-tauri/target/release/$libraryName',
        '../log-analyzer/src-tauri/target/debug/$libraryName',
        if (projectRoot != null) ...[
          '$projectRoot/../log-analyzer/src-tauri/target/release/$libraryName',
          '$projectRoot/../log-analyzer/src-tauri/target/debug/$libraryName',
        ],
      ]);
    } else if (Platform.isWindows) {
      const libraryName = 'log_analyzer.dll';
      paths.addAll([
        '../log-analyzer/src-tauri/target/release/$libraryName',
        '../log-analyzer/src-tauri/target/debug/$libraryName',
        if (projectRoot != null) ...[
          '$projectRoot/../log-analyzer/src-tauri/target/release/$libraryName',
          '$projectRoot/../log-analyzer/src-tauri/target/debug/$libraryName',
        ],
      ]);
    }

    return paths;
  }

  /// Find project root by looking for pubspec.yaml
  String? _findProjectRoot(String startPath) {
    var dir = File(startPath).parent;
    for (var i = 0; i < 10; i++) {
      final pubspec = File('${dir.path}/pubspec.yaml');
      if (pubspec.existsSync()) {
        return dir.path;
      }
      final parent = dir.parent;
      if (parent.path == dir.path) break; // Reached root
      dir = parent;
    }
    return null;
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
  String searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    String? filters,
  }) {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      return ffi.searchLogs(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
        filters: filters,
      );
    } catch (e) {
      debugPrint('searchLogs error: $e');
      rethrow;
    }
  }

  /// 取消搜索
  bool cancelSearch(String searchId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.cancelSearch(searchId: searchId);
    } catch (e) {
      debugPrint('cancelSearch error: $e');
      return false;
    }
  }

  /// 获取活跃搜索数量
  int getActiveSearchesCount() {
    if (!isFfiEnabled) {
      return 0;
    }

    try {
      return ffi.getActiveSearchesCount();
    } catch (e) {
      debugPrint('getActiveSearchesCount error: $e');
      return 0;
    }
  }

  // ==================== 工作区操作 ====================

  /// 获取工作区列表
  List<ffi_types.WorkspaceData> getWorkspaces() {
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
  String createWorkspace({required String name, required String path}) {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      return ffi.createWorkspace(name: name, path: path);
    } catch (e) {
      debugPrint('createWorkspace error: $e');
      rethrow;
    }
  }

  /// 删除工作区
  bool deleteWorkspace(String workspaceId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.deleteWorkspace(workspaceId: workspaceId);
    } catch (e) {
      debugPrint('deleteWorkspace error: $e');
      return false;
    }
  }

  /// 刷新工作区
  String refreshWorkspace(String workspaceId, String path) {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      return ffi.refreshWorkspace(workspaceId: workspaceId, path: path);
    } catch (e) {
      debugPrint('refreshWorkspace error: $e');
      rethrow;
    }
  }

  /// 获取工作区状态
  ffi_types.WorkspaceStatusData? getWorkspaceStatus(String workspaceId) {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      return ffi.getWorkspaceStatus(workspaceId: workspaceId);
    } catch (e) {
      debugPrint('getWorkspaceStatus error: $e');
      return null;
    }
  }

  // ==================== 关键词操作 ====================

  /// 获取关键词列表
  List<ffi_types.FfiKeywordGroupData> getKeywords() {
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
  bool addKeywordGroup(ffi_types.KeywordGroupInput group) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.addKeywordGroup(group: group);
    } catch (e) {
      debugPrint('addKeywordGroup error: $e');
      return false;
    }
  }

  /// 更新关键词组
  bool updateKeywordGroup(String groupId, ffi_types.KeywordGroupInput group) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.updateKeywordGroup(groupId: groupId, group: group);
    } catch (e) {
      debugPrint('updateKeywordGroup error: $e');
      return false;
    }
  }

  /// 删除关键词组
  bool deleteKeywordGroup(String groupId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.deleteKeywordGroup(groupId: groupId);
    } catch (e) {
      debugPrint('deleteKeywordGroup error: $e');
      return false;
    }
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  ffi_types.TaskMetricsData? getTaskMetrics() {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      return ffi.getTaskMetrics();
    } catch (e) {
      debugPrint('getTaskMetrics error: $e');
      return null;
    }
  }

  /// 取消任务
  bool cancelTask(String taskId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.cancelTask(taskId: taskId);
    } catch (e) {
      debugPrint('cancelTask error: $e');
      return false;
    }
  }

  // ==================== 配置操作 ====================

  /// 加载配置
  ffi_types.ConfigData? loadConfig() {
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
  bool saveConfig(ffi_types.ConfigData config) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.saveConfig(config: config);
    } catch (e) {
      debugPrint('saveConfig error: $e');
      return false;
    }
  }

  // ==================== 性能监控 ====================

  /// 获取性能指标
  ffi_types.PerformanceMetricsData? getPerformanceMetrics(String timeRange) {
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
  bool startWatch({
    required String workspaceId,
    required List<String> paths,
    required bool recursive,
  }) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.startWatch(
        workspaceId: workspaceId,
        paths: paths,
        recursive: recursive,
      );
    } catch (e) {
      debugPrint('startWatch error: $e');
      return false;
    }
  }

  /// 停止文件监听
  bool stopWatch(String workspaceId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.stopWatch(workspaceId: workspaceId);
    } catch (e) {
      debugPrint('stopWatch error: $e');
      return false;
    }
  }

  /// 检查是否正在监听
  bool isWatching(String workspaceId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.isWatching(workspaceId: workspaceId);
    } catch (e) {
      debugPrint('isWatching error: $e');
      return false;
    }
  }

  // ==================== 导入操作 ====================

  /// 导入文件夹
  String importFolder(String path, String workspaceId) {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      return ffi.importFolder(path: path, workspaceId: workspaceId);
    } catch (e) {
      debugPrint('importFolder error: $e');
      rethrow;
    }
  }

  /// 检查 RAR 支持
  bool checkRarSupport() {
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
  String exportResults({
    required String searchId,
    required String format,
    required String outputPath,
  }) {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      return ffi.exportResults(
        searchId: searchId,
        format: format,
        outputPath: outputPath,
      );
    } catch (e) {
      debugPrint('exportResults error: $e');
      rethrow;
    }
  }

  // ==================== 搜索历史操作 ====================

  /// 添加搜索历史记录
  ///
  /// 将搜索查询添加到历史记录
  bool addSearchHistory({
    required String query,
    required String workspaceId,
    required int resultCount,
  }) {
    if (!isFfiEnabled) {
      throw FfiInitializationException('FFI not initialized');
    }

    try {
      return ffi.addSearchHistory(
        query: query,
        workspaceId: workspaceId,
        resultCount: resultCount,
      );
    } catch (e) {
      debugPrint('addSearchHistory error: $e');
      rethrow;
    }
  }

  /// 获取搜索历史记录
  ///
  /// 获取指定工作区或所有工作区的搜索历史
  /// 返回 Map 列表以便转换为本地模型
  List<Map<String, dynamic>> getSearchHistory({
    String? workspaceId,
    int? limit,
  }) {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      // FFI 返回 SearchHistoryData 列表，转换为 Map
      final result = ffi.getSearchHistory(
        workspaceId: workspaceId,
        limit: limit,
      );
      // 将 FFI 类型转换为 Map
      return result
          .map(
            (item) => {
              'query': item.query,
              'workspace_id': item.workspaceId,
              'result_count': item.resultCount,
              'searched_at': item.searchedAt,
            },
          )
          .toList();
    } catch (e) {
      debugPrint('getSearchHistory error: $e');
      return [];
    }
  }

  /// 删除搜索历史记录
  ///
  /// 删除指定工作区中特定查询的历史记录
  bool deleteSearchHistory({
    required String query,
    required String workspaceId,
  }) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.deleteSearchHistory(query: query, workspaceId: workspaceId);
    } catch (e) {
      debugPrint('deleteSearchHistory error: $e');
      return false;
    }
  }

  /// 批量删除搜索历史记录
  ///
  /// 批量删除指定工作区中多个查询的历史记录
  int deleteSearchHistories({
    required List<String> queries,
    required String workspaceId,
  }) {
    if (!isFfiEnabled) {
      return 0;
    }

    try {
      return ffi.deleteSearchHistories(
        queries: queries,
        workspaceId: workspaceId,
      );
    } catch (e) {
      debugPrint('deleteSearchHistories error: $e');
      return 0;
    }
  }

  /// 清空搜索历史
  ///
  /// 清空指定工作区或所有工作区的搜索历史
  int clearSearchHistory({String? workspaceId}) {
    if (!isFfiEnabled) {
      return 0;
    }

    try {
      return ffi.clearSearchHistory(workspaceId: workspaceId);
    } catch (e) {
      debugPrint('clearSearchHistory error: $e');
      return 0;
    }
  }

  // ==================== 虚拟文件树操作 ====================

  /// 获取虚拟文件树（根节点）
  ///
  /// 获取指定工作区的虚拟文件树结构
  ///
  /// # 参数
  ///
  /// * `workspaceId` - 工作区 ID
  ///
  /// # 返回
  ///
  /// 返回根节点列表
  List<ffi_types.VirtualTreeNodeData> getVirtualFileTree(String workspaceId) {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.getVirtualFileTree(workspaceId: workspaceId);
    } catch (e) {
      debugPrint('getVirtualFileTree error: $e');
      return [];
    }
  }

  /// 获取树子节点（懒加载）
  ///
  /// 获取指定父节点下的子节点
  ///
  /// # 参数
  ///
  /// * `workspaceId` - 工作区 ID
  /// * `parentPath` - 父节点路径
  ///
  /// # 返回
  ///
  /// 返回子节点列表
  List<ffi_types.VirtualTreeNodeData> getTreeChildren({
    required String workspaceId,
    required String parentPath,
  }) {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.getTreeChildren(
        workspaceId: workspaceId,
        parentPath: parentPath,
      );
    } catch (e) {
      debugPrint('getTreeChildren error: $e');
      return [];
    }
  }

  /// 通过哈希读取文件内容
  ///
  /// 从 CAS 存储读取指定哈希的文件内容
  ///
  /// # 参数
  ///
  /// * `workspaceId` - 工作区 ID
  /// * `hash` - 文件 SHA-256 哈希
  ///
  /// # 返回
  ///
  /// 返回文件内容响应
  ffi_types.FileContentResponseData? readFileByHash({
    required String workspaceId,
    required String hash,
  }) {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      return ffi.readFileByHash(workspaceId: workspaceId, hash: hash);
    } catch (e) {
      debugPrint('readFileByHash error: $e');
      return null;
    }
  }

  // ==================== 多关键词组合搜索操作 ====================

  /// 执行结构化搜索（多关键词组合搜索）
  ///
  /// 支持多个关键词的 AND/OR/NOT 组合搜索
  ///
  /// # 参数
  ///
  /// * `query` - 结构化搜索查询对象
  /// * `workspaceId` - 工作区 ID（可选，默认使用第一个可用工作区）
  /// * `maxResults` - 最大返回结果数量
  ///
  /// # 返回
  ///
  /// 返回匹配的搜索结果列表
  List<ffi_types.FfiSearchResultEntry> searchStructured({
    required ffi_types.StructuredSearchQueryData query,
    String? workspaceId,
    int maxResults = 10000,
  }) {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.searchStructured(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
      );
    } catch (e) {
      debugPrint('searchStructured error: $e');
      return [];
    }
  }

  /// 构建搜索查询对象
  ///
  /// 从关键词列表构建结构化搜索查询，便于使用
  ///
  /// # 参数
  ///
  /// * `keywords` - 关键词列表
  /// * `globalOperator` - 全局操作符 ("AND", "OR", "NOT")
  /// * `isRegex` - 是否使用正则表达式
  /// * `caseSensitive` - 是否大小写敏感
  ///
  /// # 返回
  ///
  /// 返回构建的结构化搜索查询对象
  ffi_types.StructuredSearchQueryData buildSearchQuery({
    required List<String> keywords,
    String globalOperator = 'AND',
    bool isRegex = false,
    bool caseSensitive = false,
  }) {
    if (!isFfiEnabled) {
      return const ffi_types.StructuredSearchQueryData(
        terms: [],
        globalOperator: ffi_types.QueryOperatorData.and,
      );
    }

    try {
      return ffi.buildSearchQuery(
        keywords: keywords,
        globalOperator: globalOperator,
        isRegex: isRegex,
        caseSensitive: caseSensitive,
      );
    } catch (e) {
      debugPrint('buildSearchQuery error: $e');
      return const ffi_types.StructuredSearchQueryData(
        terms: [],
        globalOperator: ffi_types.QueryOperatorData.and,
      );
    }
  }

  // ==================== 正则搜索操作 ====================

  /// 验证正则表达式语法
  ///
  /// 验证正则表达式是否有效，返回验证结果和错误信息
  ///
  /// # 参数
  ///
  /// * `pattern` - 正则表达式模式
  ///
  /// # 返回
  ///
  /// 返回验证结果，包含是否有效和可能的错误信息
  ffi_types.RegexValidationResult validateRegex(String pattern) {
    if (!isFfiEnabled) {
      return const ffi_types.RegexValidationResult(
        valid: false,
        errorMessage: 'FFI not initialized',
      );
    }

    try {
      return ffi.validateRegex(pattern: pattern);
    } catch (e) {
      debugPrint('validateRegex error: $e');
      return ffi_types.RegexValidationResult(
        valid: false,
        errorMessage: e.toString(),
      );
    }
  }

  /// 执行正则表达式搜索
  ///
  /// 在工作区中搜索匹配正则表达式的行
  ///
  /// # 参数
  ///
  /// * `pattern` - 正则表达式模式
  /// * `workspaceId` - 工作区 ID（可选，默认使用第一个可用工作区）
  /// * `maxResults` - 最大结果数量
  /// * `caseSensitive` - 是否大小写敏感
  ///
  /// # 返回
  ///
  /// 返回匹配的搜索结果列表
  List<ffi_types.FfiSearchResultEntry> searchRegex({
    required String pattern,
    String? workspaceId,
    int maxResults = 10000,
    bool caseSensitive = false,
  }) {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      return ffi.searchRegex(
        pattern: pattern,
        workspaceId: workspaceId,
        maxResults: maxResults,
        caseSensitive: caseSensitive,
      );
    } catch (e) {
      debugPrint('searchRegex error: $e');
      return [];
    }
  }

  // ==================== 过滤器操作 ====================

  /// 保存或更新过滤器
  ///
  /// 根据 workspace_id + name 唯一键保存或更新过滤器
  ///
  /// # 参数
  ///
  /// * `filter` - SavedFilter 对象
  ///
  /// # 返回
  ///
  /// 成功返回 true
  bool saveFilter(SavedFilter filter) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      // 将 SavedFilter 转换为 FFI SavedFilterInput
      final filterInput = ffi_types.SavedFilterInput(
        name: filter.name,
        description: filter.description,
        workspaceId: filter.workspaceId,
        termsJson: jsonEncode(filter.terms.map((t) => t.toJson()).toList()),
        globalOperator: filter.globalOperator,
        timeRangeStart: filter.timeRange?.start,
        timeRangeEnd: filter.timeRange?.end,
        levelsJson: filter.levels.isNotEmpty ? jsonEncode(filter.levels) : null,
        filePattern: filter.filePattern,
        isDefault: filter.isDefault,
        sortOrder: filter.sortOrder,
      );
      return ffi.saveFilter(filter: filterInput);
    } catch (e) {
      debugPrint('saveFilter error: $e');
      return false;
    }
  }

  /// 获取工作区的所有过滤器
  ///
  /// 获取指定工作区的所有已保存过滤器
  ///
  /// # 参数
  ///
  /// * `workspaceId` - 工作区 ID
  /// * `limit` - 最大返回数量（可选）
  ///
  /// # 返回
  ///
  /// 返回过滤器列表
  List<SavedFilter> getSavedFilters(String workspaceId, {int? limit}) {
    if (!isFfiEnabled) {
      return [];
    }

    try {
      final ffiFilters = ffi.getSavedFilters(
        workspaceId: workspaceId,
        limit: limit,
      );
      // 将 FFI SavedFilterData 转换为 SavedFilter
      return ffiFilters
          .map(
            (f) => SavedFilter.fromFfiMap({
              'id': f.id,
              'name': f.name,
              'description': f.description,
              'workspace_id': f.workspaceId,
              'terms_json': f.termsJson,
              'global_operator': f.globalOperator,
              'time_range_start': f.timeRangeStart,
              'time_range_end': f.timeRangeEnd,
              'levels_json': f.levelsJson,
              'file_pattern': f.filePattern,
              'is_default': f.isDefault,
              'sort_order': f.sortOrder,
              'usage_count': f.usageCount,
              'created_at': f.createdAt,
              'last_used_at': f.lastUsedAt,
            }),
          )
          .toList();
    } catch (e) {
      debugPrint('getSavedFilters error: $e');
      return [];
    }
  }

  /// 删除指定过滤器
  ///
  /// 删除指定工作区中的过滤器
  ///
  /// # 参数
  ///
  /// * `filterId` - 过滤器 ID
  /// * `workspaceId` - 工作区 ID
  ///
  /// # 返回
  ///
  /// 成功返回 true
  bool deleteFilter(String filterId, String workspaceId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.deleteFilter(filterId: filterId, workspaceId: workspaceId);
    } catch (e) {
      debugPrint('deleteFilter error: $e');
      return false;
    }
  }

  /// 更新过滤器使用统计
  ///
  /// 更新过滤器的使用次数和最后使用时间
  ///
  /// # 参数
  ///
  /// * `filterId` - 过滤器 ID
  /// * `workspaceId` - 工作区 ID
  ///
  /// # 返回
  ///
  /// 成功返回 true
  bool updateFilterUsage(String filterId, String workspaceId) {
    if (!isFfiEnabled) {
      return false;
    }

    try {
      return ffi.updateFilterUsage(
        filterId: filterId,
        workspaceId: workspaceId,
      );
    } catch (e) {
      debugPrint('updateFilterUsage error: $e');
      return false;
    }
  }

  // ==================== 日志级别统计操作 ====================

  /// 获取日志级别统计
  ///
  /// 返回工作区中每个日志级别的记录数量
  ///
  /// # 参数
  ///
  /// * `workspaceId` - 工作区 ID
  ///
  /// # 返回
  ///
  /// 返回日志级别统计结果，包含每个级别的数量
  Map<String, dynamic>? getLogLevelStats(String workspaceId) {
    if (!isFfiEnabled) {
      return null;
    }

    try {
      final stats = ffi.getLogLevelStats(workspaceId: workspaceId);
      return {
        'fatal_count': stats.fatalCount,
        'error_count': stats.errorCount,
        'warn_count': stats.warnCount,
        'info_count': stats.infoCount,
        'debug_count': stats.debugCount,
        'trace_count': stats.traceCount,
        'unknown_count': stats.unknownCount,
        'total': stats.total,
      };
    } catch (e) {
      debugPrint('getLogLevelStats error: $e');
      return null;
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
enum ChartTimeRange { minutes1, minutes5, minutes15, hour1 }

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
