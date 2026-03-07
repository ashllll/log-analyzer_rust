// MockBridgeService - 模拟 FFI 桥接服务
//
// 用于测试环境，模拟与 Rust 后端的 FFI 通信
// 不依赖真实 FFI 初始化，使用纯 Dart 类型

/// Mock 搜索结果数据
class MockSearchResult {
  final String id;
  final String content;
  final String filePath;
  final int lineNumber;
  final int matchStart;
  final int matchEnd;

  const MockSearchResult({
    required this.id,
    required this.content,
    required this.filePath,
    required this.lineNumber,
    required this.matchStart,
    required this.matchEnd,
  });
}

/// Mock 虚拟文件树节点
class MockVirtualTreeNode {
  final String name;
  final String path;
  final String hash;
  final int size;
  final String? mimeType;
  final String? archiveType;
  final List<MockVirtualTreeNode> children;

  const MockVirtualTreeNode({
    required this.name,
    required this.path,
    required this.hash,
    required this.size,
    this.mimeType,
    this.archiveType,
    this.children = const [],
  });

  bool get isArchive => archiveType != null;
}

/// Mock 文件内容响应
class MockFileContentResponse {
  final String content;
  final String hash;
  final int size;

  const MockFileContentResponse({
    required this.content,
    required this.hash,
    required this.size,
  });
}

/// Mock 搜索历史条目
class MockSearchHistoryItem {
  final String query;
  final String workspaceId;
  final int resultCount;
  final String searchedAt;

  const MockSearchHistoryItem({
    required this.query,
    required this.workspaceId,
    required this.resultCount,
    required this.searchedAt,
  });
}

/// Mock 工作区数据
class MockWorkspaceData {
  final String id;
  final String name;
  final String path;
  final String status;
  final int files;
  final int size;
  final DateTime? lastOpenedAt;
  final DateTime? createdAt;

  const MockWorkspaceData({
    required this.id,
    required this.name,
    required this.path,
    required this.status,
    required this.files,
    required this.size,
    this.lastOpenedAt,
    this.createdAt,
  });
}

/// Mock FFI 桥接服务
///
/// 模拟 BridgeService 的行为，用于单元测试
/// 支持配置返回数据和模拟错误
class MockBridgeService {
  /// 是否 FFI 已启用（始终为 true 用于测试）
  bool isFfiEnabled = true;

  /// 是否已初始化
  bool isInitialized = true;

  /// Mock 搜索结果
  List<MockSearchResult> _searchResults = [];

  /// Mock 文件树节点
  List<MockVirtualTreeNode> _treeNodes = [];

  /// Mock 搜索历史
  final List<MockSearchHistoryItem> _searchHistory = [];

  /// Mock 工作区列表
  List<MockWorkspaceData> _workspaces = [];

  /// Mock 文件内容
  final Map<String, MockFileContentResponse> _fileContents = {};

  /// 错误模拟配置
  Exception? _searchError;
  Exception? _workspaceError;
  Exception? _historyError;
  Exception? _treeError;
  Exception? _fileReadError;

  // ==================== 配置方法 ====================

  /// 设置 Mock 搜索结果
  void setSearchResults(List<MockSearchResult> results) {
    _searchResults = results;
  }

  /// 设置 Mock 文件树节点
  void setTreeNodes(List<MockVirtualTreeNode> nodes) {
    _treeNodes = nodes;
  }

  /// 设置 Mock 搜索历史
  void setSearchHistory(List<MockSearchHistoryItem> history) {
    _searchHistory.clear();
    _searchHistory.addAll(history);
  }

  /// 设置 Mock 工作区列表
  void setWorkspaces(List<MockWorkspaceData> workspaces) {
    _workspaces = workspaces;
  }

  /// 添加 Mock 文件内容
  void addFileContent(String hash, MockFileContentResponse content) {
    _fileContents[hash] = content;
  }

  /// 配置搜索错误
  void setSearchError(Exception error) {
    _searchError = error;
  }

  /// 配置工作区错误
  void setWorkspaceError(Exception error) {
    _workspaceError = error;
  }

  /// 配置历史错误
  void setHistoryError(Exception error) {
    _historyError = error;
  }

  /// 配置文件树错误
  void setTreeError(Exception error) {
    _treeError = error;
  }

  /// 配置文件读取错误
  void setFileReadError(Exception error) {
    _fileReadError = error;
  }

  /// 重置所有 Mock 数据
  void reset() {
    _searchResults = [];
    _treeNodes = [];
    _searchHistory.clear();
    _workspaces = [];
    _fileContents.clear();
    _searchError = null;
    _workspaceError = null;
    _historyError = null;
    _treeError = null;
    _fileReadError = null;
  }

  // ==================== 模拟 FFI 方法 ====================

  /// 模拟搜索日志
  Future<String> searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    String? filters,
  }) async {
    if (_searchError != null) {
      throw _searchError!;
    }

    // 返回模拟的搜索 ID
    return 'mock-search-${DateTime.now().millisecondsSinceEpoch}';
  }

  /// 模拟获取搜索结果
  List<MockSearchResult> getSearchResults(String searchId) {
    if (_searchError != null) {
      throw _searchError!;
    }

    return _searchResults;
  }

  /// 模拟取消搜索
  Future<bool> cancelSearch(String searchId) async {
    return true;
  }

  /// 模拟获取活跃搜索数量
  Future<int> getActiveSearchesCount() async {
    return 0;
  }

  /// 模拟获取工作区列表
  Future<List<MockWorkspaceData>> getWorkspaces() async {
    if (_workspaceError != null) {
      throw _workspaceError!;
    }

    return _workspaces;
  }

  /// 模拟获取虚拟文件树
  Future<List<MockVirtualTreeNode>> getVirtualFileTree(
      String workspaceId) async {
    if (_treeError != null) {
      throw _treeError!;
    }

    return _treeNodes;
  }

  /// 模拟获取树子节点
  Future<List<MockVirtualTreeNode>> getTreeChildren({
    required String workspaceId,
    required String parentPath,
  }) async {
    if (_treeError != null) {
      throw _treeError!;
    }

    // 查找父节点并返回其子节点
    for (final node in _treeNodes) {
      if (node.path == parentPath && node.isArchive) {
        return node.children;
      }
    }

    return [];
  }

  /// 模拟通过哈希读取文件
  Future<MockFileContentResponse?> readFileByHash({
    required String workspaceId,
    required String hash,
  }) async {
    if (_fileReadError != null) {
      throw _fileReadError!;
    }

    return _fileContents[hash];
  }

  /// 模拟获取搜索历史
  List<MockSearchHistoryItem> getSearchHistory({
    required String workspaceId,
  }) {
    if (_historyError != null) {
      throw _historyError!;
    }

    return _searchHistory
        .where((h) => h.workspaceId == workspaceId)
        .toList();
  }

  /// 模拟添加搜索历史
  void addSearchHistory({
    required String query,
    required String workspaceId,
    required int resultCount,
  }) {
    if (_historyError != null) {
      throw _historyError!;
    }

    _searchHistory.add(MockSearchHistoryItem(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
      searchedAt: DateTime.now().toIso8601String(),
    ));
  }

  /// 模拟删除搜索历史
  void deleteSearchHistory({
    required String query,
    required String workspaceId,
  }) {
    if (_historyError != null) {
      throw _historyError!;
    }

    _searchHistory.removeWhere(
        (h) => h.query == query && h.workspaceId == workspaceId);
  }

  /// 模拟批量删除搜索历史
  void deleteSearchHistories({
    required List<String> queries,
    required String workspaceId,
  }) {
    if (_historyError != null) {
      throw _historyError!;
    }

    _searchHistory.removeWhere(
        (h) => queries.contains(h.query) && h.workspaceId == workspaceId);
  }

  /// 模拟清空搜索历史
  void clearSearchHistory({required String workspaceId}) {
    if (_historyError != null) {
      throw _historyError!;
    }

    _searchHistory.removeWhere((h) => h.workspaceId == workspaceId);
  }

  /// 模拟健康检查
  String checkHealth() {
    return 'MOCK_OK';
  }

  // ==================== 访问器（用于测试） ====================

  /// 获取内部搜索结果（测试用）
  List<MockSearchResult> get searchResults => _searchResults;

  /// 获取内部文件树节点（测试用）
  List<MockVirtualTreeNode> get treeNodes => _treeNodes;

  /// 获取内部搜索历史（测试用）
  List<MockSearchHistoryItem> get searchHistory => _searchHistory;

  /// 获取内部文件内容（测试用）
  Map<String, MockFileContentResponse> get fileContents => _fileContents;
}

/// 全局 MockBridgeService 实例
final mockBridgeService = MockBridgeService();
