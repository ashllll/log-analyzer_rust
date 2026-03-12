// lib/shared/services/ffi_service.dart
/// FFI 服务
///
/// 提供静态方法访问 Rust 后端功能
/// 这是对 BridgeService 的简化封装，便于使用
import 'bridge_service.dart';
import 'generated/ffi/types.dart' as ffi_types;

/// FFI 服务类
///
/// 提供简洁的静态方法访问所有 FFI 功能
class FfiService {
  static final BridgeService _bridge = BridgeService.instance;

  // ==================== 初始化 ====================

  /// 初始化 FFI 桥接
  static Future<void> initialize() async {
    await _bridge.initialize();
  }

  /// 检查 FFI 是否已启用
  static bool get isInitialized => _bridge.isFfiEnabled;

  /// 获取初始化错误消息
  static String? get initErrorMessage => _bridge.initErrorMessage;

  /// 健康检查
  static String checkHealth() => _bridge.checkHealth();

  // ==================== 工作区操作 ====================

  /// 获取工作区列表
  static List<ffi_types.WorkspaceData> getWorkspaces() {
    return _bridge.getWorkspaces();
  }

  /// 创建工作区
  static String createWorkspace({required String name, required String path}) {
    return _bridge.createWorkspace(name: name, path: path);
  }

  /// 删除工作区
  static bool deleteWorkspace(String workspaceId) {
    return _bridge.deleteWorkspace(workspaceId);
  }

  /// 刷新工作区
  static String refreshWorkspace(String workspaceId, String path) {
    return _bridge.refreshWorkspace(workspaceId, path);
  }

  /// 获取工作区状态
  static ffi_types.WorkspaceStatusData? getWorkspaceStatus(String workspaceId) {
    return _bridge.getWorkspaceStatus(workspaceId);
  }

  // ==================== 搜索操作 ====================

  /// 执行日志搜索
  static String searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    String? filters,
  }) {
    return _bridge.searchLogs(
      query: query,
      workspaceId: workspaceId,
      maxResults: maxResults,
      filters: filters,
    );
  }

  /// 取消搜索
  static bool cancelSearch(String searchId) {
    return _bridge.cancelSearch(searchId);
  }

  /// 获取活跃搜索数量
  static int getActiveSearchesCount() {
    return _bridge.getActiveSearchesCount();
  }

  /// 执行结构化搜索
  static List<ffi_types.FfiSearchResultEntry> searchStructured({
    required ffi_types.StructuredSearchQueryData query,
    String? workspaceId,
    int maxResults = 10000,
  }) {
    return _bridge.searchStructured(
      query: query,
      workspaceId: workspaceId,
      maxResults: maxResults,
    );
  }

  /// 执行正则搜索
  static List<ffi_types.FfiSearchResultEntry> searchRegex({
    required String pattern,
    String? workspaceId,
    int maxResults = 10000,
    bool caseSensitive = false,
  }) {
    return _bridge.searchRegex(
      pattern: pattern,
      workspaceId: workspaceId,
      maxResults: maxResults,
      caseSensitive: caseSensitive,
    );
  }

  /// 验证正则表达式
  static ffi_types.RegexValidationResult validateRegex(String pattern) {
    return _bridge.validateRegex(pattern);
  }

  /// 构建搜索查询
  static ffi_types.StructuredSearchQueryData buildSearchQuery({
    required List<String> keywords,
    String globalOperator = 'AND',
    bool isRegex = false,
    bool caseSensitive = false,
  }) {
    return _bridge.buildSearchQuery(
      keywords: keywords,
      globalOperator: globalOperator,
      isRegex: isRegex,
      caseSensitive: caseSensitive,
    );
  }

  // ==================== 关键词操作 ====================

  /// 获取关键词列表
  static List<ffi_types.FfiKeywordGroupData> getKeywords() {
    return _bridge.getKeywords();
  }

  /// 添加关键词组
  static bool addKeywordGroup(ffi_types.KeywordGroupInput group) {
    return _bridge.addKeywordGroup(group);
  }

  /// 更新关键词组
  static bool updateKeywordGroup(
    String groupId,
    ffi_types.KeywordGroupInput group,
  ) {
    return _bridge.updateKeywordGroup(groupId, group);
  }

  /// 删除关键词组
  static bool deleteKeywordGroup(String groupId) {
    return _bridge.deleteKeywordGroup(groupId);
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  static ffi_types.TaskMetricsData? getTaskMetrics() {
    return _bridge.getTaskMetrics();
  }

  /// 取消任务
  static bool cancelTask(String taskId) {
    return _bridge.cancelTask(taskId);
  }

  // ==================== 配置操作 ====================

  /// 加载配置
  static ffi_types.ConfigData? loadConfig() {
    return _bridge.loadConfig();
  }

  /// 保存配置
  static bool saveConfig(ffi_types.ConfigData config) {
    return _bridge.saveConfig(config);
  }

  // ==================== 性能监控 ====================

  /// 获取性能指标
  static ffi_types.PerformanceMetricsData? getPerformanceMetrics(
    String timeRange,
  ) {
    return _bridge.getPerformanceMetrics(timeRange);
  }

  // ==================== 文件监听 ====================

  /// 启动文件监听
  static bool startWatch({
    required String workspaceId,
    required List<String> paths,
    required bool recursive,
  }) {
    return _bridge.startWatch(
      workspaceId: workspaceId,
      paths: paths,
      recursive: recursive,
    );
  }

  /// 停止文件监听
  static bool stopWatch(String workspaceId) {
    return _bridge.stopWatch(workspaceId);
  }

  /// 检查是否正在监听
  static bool isWatching(String workspaceId) {
    return _bridge.isWatching(workspaceId);
  }

  // ==================== 导入操作 ====================

  /// 导入文件夹
  static String importFolder(String path, String workspaceId) {
    return _bridge.importFolder(path, workspaceId);
  }

  /// 检查 RAR 支持
  static bool checkRarSupport() {
    return _bridge.checkRarSupport();
  }

  // ==================== 导出操作 ====================

  /// 导出搜索结果
  static String exportResults({
    required String searchId,
    required String format,
    required String outputPath,
  }) {
    return _bridge.exportResults(
      searchId: searchId,
      format: format,
      outputPath: outputPath,
    );
  }

  // ==================== 搜索历史操作 ====================

  /// 添加搜索历史记录
  static bool addSearchHistory({
    required String query,
    required String workspaceId,
    required int resultCount,
  }) {
    return _bridge.addSearchHistory(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
    );
  }

  /// 获取搜索历史记录
  static List<Map<String, dynamic>> getSearchHistory({
    String? workspaceId,
    int? limit,
  }) {
    return _bridge.getSearchHistory(workspaceId: workspaceId, limit: limit);
  }

  /// 删除搜索历史记录
  static bool deleteSearchHistory({
    required String query,
    required String workspaceId,
  }) {
    return _bridge.deleteSearchHistory(query: query, workspaceId: workspaceId);
  }

  /// 批量删除搜索历史记录
  static int deleteSearchHistories({
    required List<String> queries,
    required String workspaceId,
  }) {
    return _bridge.deleteSearchHistories(
      queries: queries,
      workspaceId: workspaceId,
    );
  }

  /// 清空搜索历史
  static int clearSearchHistory({String? workspaceId}) {
    return _bridge.clearSearchHistory(workspaceId: workspaceId);
  }

  // ==================== 虚拟文件树操作 ====================

  /// 获取虚拟文件树
  static List<ffi_types.VirtualTreeNodeData> getVirtualFileTree(
    String workspaceId,
  ) {
    return _bridge.getVirtualFileTree(workspaceId);
  }

  /// 获取树子节点
  static List<ffi_types.VirtualTreeNodeData> getTreeChildren({
    required String workspaceId,
    required String parentPath,
  }) {
    return _bridge.getTreeChildren(
      workspaceId: workspaceId,
      parentPath: parentPath,
    );
  }

  /// 通过哈希读取文件内容
  static ffi_types.FileContentResponseData? readFileByHash({
    required String workspaceId,
    required String hash,
  }) {
    return _bridge.readFileByHash(workspaceId: workspaceId, hash: hash);
  }

  // ==================== 日志级别统计 ====================

  /// 获取日志级别统计
  static Map<String, dynamic>? getLogLevelStats(String workspaceId) {
    return _bridge.getLogLevelStats(workspaceId);
  }
}

/// FFI 初始化异常
class FfiInitializationException implements Exception {
  final String message;
  FfiInitializationException(this.message);
  @override
  String toString() => 'FfiInitializationException: $message';
}
