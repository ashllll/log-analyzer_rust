import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:dio/dio.dart';

// 启用 HTTP 客户端模式
const bool _useHttpClient = true;

/// 桥接服务
///
/// 使用 HTTP API 与 Rust 后端通信
/// 通过 RESTful API 调用实现跨进程通信
class BridgeService {
  /// 单例模式
  static final BridgeService _instance = BridgeService._internal();
  factory BridgeService() => _instance;

  late final Dio _dio;

  BridgeService._internal() {
    _dio = Dio(BaseOptions(
      baseUrl: 'http://localhost:8080',
      connectTimeout: const Duration(seconds: 30),
      receiveTimeout: const Duration(seconds: 30),
      headers: {
        'Content-Type': 'application/json',
      },
    ));

    // 添加日志拦截器（调试模式）
    if (kDebugMode) {
      _dio.interceptors.add(LogInterceptor(
        requestBody: true,
        responseBody: true,
        error: true,
      ));
    }

    _initialize();
  }

  static bool _isInitialized = false;

  /// 初始化桥接
  void _initialize() {
    if (_isInitialized) return;
    _isInitialized = true;
    debugPrint('BridgeService: 使用 HTTP API (http://localhost:8080)');
  }

  /// 是否使用 HTTP 客户端
  bool get isHttpEnabled => _useHttpClient;

  /// 是否已初始化
  bool get isInitialized => _isInitialized;

  /// 健康检查
  Future<String> healthCheck() async {
    try {
      final response = await _dio.get('/health');
      if (response.statusCode == 200) {
        return 'OK';
      }
      return 'ERROR';
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
    String? filtersJson,
  }) async {
    try {
      final response = await _dio.post('/api/search', data: {
        'query': query,
        'workspace_id': workspaceId,
        'max_results': maxResults,
        if (filtersJson != null) 'filters': filtersJson,
      });

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return data['data']['search_id'];
        }
        throw Exception(data['error'] ?? 'Search failed');
      }
      throw Exception('HTTP ${response.statusCode}');
    } catch (e) {
      debugPrint('searchLogs error: $e');
      rethrow;
    }
  }

  /// 取消搜索
  Future<bool> cancelSearch(String searchId) async {
    try {
      final response = await _dio.post('/api/search/cancel', data: {
        'search_id': searchId,
      });
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('cancelSearch error: $e');
      return false;
    }
  }

  /// 获取活跃搜索数量
  Future<int> getActiveSearchesCount() async {
    try {
      final response = await _dio.get('/api/search/active/count');
      if (response.statusCode == 200) {
        return response.data['data'] ?? 0;
      }
      return 0;
    } catch (e) {
      debugPrint('getActiveSearchesCount error: $e');
      return 0;
    }
  }

  // ==================== 工作区操作 ====================

  /// 获取工作区列表
  Future<List<Map<String, dynamic>>> getWorkspaces() async {
    try {
      final response = await _dio.get('/api/workspaces');

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return List<Map<String, dynamic>>.from(data['data']);
        }
        throw Exception(data['error'] ?? 'Failed to get workspaces');
      }
      throw Exception('HTTP ${response.statusCode}');
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
    try {
      final response = await _dio.post('/api/workspace', data: {
        'name': name,
        'path': path,
      });

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return data['data'];
        }
        throw Exception(data['error'] ?? 'Failed to create workspace');
      }
      throw Exception('HTTP ${response.statusCode}');
    } catch (e) {
      debugPrint('createWorkspace error: $e');
      rethrow;
    }
  }

  /// 删除工作区
  Future<bool> deleteWorkspace(String workspaceId) async {
    try {
      final response = await _dio.delete('/api/workspace/$workspaceId');
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('deleteWorkspace error: $e');
      return false;
    }
  }

  /// 刷新工作区
  Future<String> refreshWorkspace(String workspaceId, String path) async {
    try {
      final response = await _dio.post('/api/workspace/$workspaceId/refresh', data: {
        'path': path,
      });

      if (response.statusCode == 200) {
        return workspaceId;
      }
      throw Exception('HTTP ${response.statusCode}');
    } catch (e) {
      debugPrint('refreshWorkspace error: $e');
      rethrow;
    }
  }

  /// 获取工作区状态
  Future<Map<String, dynamic>?> getWorkspaceStatus(String workspaceId) async {
    try {
      final response = await _dio.get('/api/workspace/$workspaceId/status');

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return data['data'];
        }
        return null;
      }
      return null;
    } catch (e) {
      debugPrint('getWorkspaceStatus error: $e');
      return null;
    }
  }

  // ==================== 关键词操作 ====================

  /// 获取关键词列表
  Future<List<Map<String, dynamic>>> getKeywords() async {
    try {
      final response = await _dio.get('/api/keywords');

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return List<Map<String, dynamic>>.from(data['data']);
        }
        return [];
      }
      return [];
    } catch (e) {
      debugPrint('getKeywords error: $e');
      return [];
    }
  }

  /// 添加关键词组
  Future<bool> addKeywordGroup(Map<String, dynamic> group) async {
    try {
      final response = await _dio.post('/api/keywords', data: group);
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('addKeywordGroup error: $e');
      return false;
    }
  }

  /// 更新关键词组
  Future<bool> updateKeywordGroup(String groupId, Map<String, dynamic> group) async {
    try {
      final response = await _dio.put('/api/keywords/$groupId', data: group);
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('updateKeywordGroup error: $e');
      return false;
    }
  }

  /// 删除关键词组
  Future<bool> deleteKeywordGroup(String groupId) async {
    try {
      final response = await _dio.delete('/api/keywords/$groupId');
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('deleteKeywordGroup error: $e');
      return false;
    }
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  Future<Map<String, dynamic>?> getTaskMetrics() async {
    try {
      final response = await _dio.get('/api/tasks/metrics');

      if (response.statusCode == 200) {
        return response.data['data'];
      }
      return null;
    } catch (e) {
      debugPrint('getTaskMetrics error: $e');
      return null;
    }
  }

  /// 取消任务
  Future<bool> cancelTask(String taskId) async {
    try {
      final response = await _dio.post('/api/task/cancel', data: {
        'task_id': taskId,
      });

      if (response.statusCode == 200) {
        final data = response.data;
        return data['success'] == true;
      }
      return false;
    } catch (e) {
      debugPrint('cancelTask error: $e');
      return false;
    }
  }

  // ==================== 配置操作 ====================

  /// 加载配置
  Future<Map<String, dynamic>?> loadConfig() async {
    try {
      final response = await _dio.get('/api/config');

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return data['data'];
        }
        return null;
      }
      return null;
    } catch (e) {
      debugPrint('loadConfig error: $e');
      return null;
    }
  }

  /// 保存配置
  Future<bool> saveConfig(Map<String, dynamic> config) async {
    try {
      final response = await _dio.post('/api/config', data: config);
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('saveConfig error: $e');
      return false;
    }
  }

  // ==================== 性能监控 ====================

  /// 获取性能指标
  Future<Map<String, dynamic>?> getPerformanceMetrics(String timeRange) async {
    try {
      final response = await _dio.get('/api/performance/metrics', queryParameters: {
        'time_range': timeRange,
      });

      if (response.statusCode == 200) {
        return response.data['data'];
      }
      return null;
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
    try {
      final response = await _dio.post('/api/watch/start', data: {
        'workspace_id': workspaceId,
        'paths': paths,
        'recursive': recursive,
      });
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('startWatch error: $e');
      return false;
    }
  }

  /// 停止文件监听
  Future<bool> stopWatch(String workspaceId) async {
    try {
      final response = await _dio.post('/api/watch/stop', data: {
        'workspace_id': workspaceId,
      });
      return response.statusCode == 200;
    } catch (e) {
      debugPrint('stopWatch error: $e');
      return false;
    }
  }

  /// 检查是否正在监听
  Future<bool> isWatching(String workspaceId) async {
    try {
      final response = await _dio.get('/api/watch/status/$workspaceId');
      if (response.statusCode == 200) {
        return response.data['data'] ?? false;
      }
      return false;
    } catch (e) {
      debugPrint('isWatching error: $e');
      return false;
    }
  }

  // ==================== 导入操作 ====================

  /// 导入文件夹
  Future<String> importFolder(String path, String workspaceId) async {
    try {
      final response = await _dio.post('/api/import/folder', data: {
        'path': path,
        'workspace_id': workspaceId,
      });

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return data['data']['task_id'];
        }
        throw Exception(data['error'] ?? 'Import failed');
      }
      throw Exception('HTTP ${response.statusCode}');
    } catch (e) {
      debugPrint('importFolder error: $e');
      rethrow;
    }
  }

  /// 检查 RAR 支持
  Future<bool> checkRarSupport() async {
    try {
      final response = await _dio.get('/api/features/rar');
      if (response.statusCode == 200) {
        return response.data['data'] ?? false;
      }
      return false;
    } catch (e) {
      debugPrint('checkRarSupport error: $e');
      return false;
    }
  }

  // ==================== 导出操作 ====================

  /// 导出搜索结果
  Future<String> exportResults({
    required String searchId,
    required String format,
    required String outputPath,
  }) async {
    try {
      final response = await _dio.post('/api/export', data: {
        'search_id': searchId,
        'format': format,
        'output_path': outputPath,
      });

      if (response.statusCode == 200) {
        final data = response.data;
        if (data['success'] == true && data['data'] != null) {
          return data['data']['path'];
        }
        throw Exception(data['error'] ?? 'Export failed');
      }
      throw Exception('HTTP ${response.statusCode}');
    } catch (e) {
      debugPrint('exportResults error: $e');
      rethrow;
    }
  }

  /// 释放资源
  void dispose() {
    _dio.close();
    _isInitialized = false;
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
