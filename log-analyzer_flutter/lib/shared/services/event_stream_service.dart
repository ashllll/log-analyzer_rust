import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/common.dart';
import '../models/search.dart';
import 'generated/frb_generated.dart';

/// 事件流服务
///
/// 对应 React 版本的 websocketClient.ts
/// 处理来自 Rust 后端的实时事件流
///
/// ## 实现说明
///
/// 由于 flutter_rust_bridge 2.x 不支持 `#[frb(stream)]`，
/// 本服务采用以下策略实现事件流：
///
/// 1. **轮询模式**: 通过定时器定期获取任务状态更新
/// 2. **事件注入**: 提供外部注入事件的接口，支持 FFI 或其他通信方式
/// 3. **StreamChannel 预留**: 预留 Stream 接口，便于未来迁移
///
/// ## 事件类型映射
///
/// | Rust 事件类型 | Dart 事件 | 流控制器 |
/// |--------------|----------|---------|
/// | SearchResults | List<LogEntry> | searchResults |
/// | SearchSummary | SearchResultSummary | searchSummary |
/// | TaskUpdate | TaskProgress | taskUpdate |
/// | FileChanged | FileChangeEvent | fileChanged |
/// | WorkspaceStatus | WorkspaceStatusEvent | workspaceStatus |
class EventStreamService {
  /// flutter_rust_bridge API 实例（可选）
  ///
  /// 用于未来支持直接的流式事件传输
  LogAnalyzerBridgeApi? _bridgeApi;

  /// 轮询定时器
  Timer? _pollingTimer;

  /// 轮询间隔（毫秒）
  final int pollingIntervalMs;

  /// 是否已连接
  bool _isConnected = false;

  /// 内部流控制器 - 搜索结果
  final _searchResultsController = StreamController<List<LogEntry>>.broadcast();

  /// 内部流控制器 - 搜索摘要
  final _searchSummaryController =
      StreamController<SearchResultSummary>.broadcast();

  /// 内部流控制器 - 任务更新
  final _taskUpdateController = StreamController<TaskProgress>.broadcast();

  /// 内部流控制器 - 文件变化
  final _fileChangedController = StreamController<FileChangeEvent>.broadcast();

  /// 内部流控制器 - 工作区状态
  final _workspaceStatusController =
      StreamController<WorkspaceStatusEvent>.broadcast();

  /// 内部流控制器 - 连接状态
  final _connectionStatusController =
      StreamController<ConnectionStatus>.broadcast();

  /// 内部流控制器 - 错误
  final _errorController = StreamController<EventError>.broadcast();

  /// 内部流控制器 - 系统事件
  final _systemEventController = StreamController<SystemEvent>.broadcast();

  // ==================== 公开流 ====================

  /// 搜索结果流
  ///
  /// 对应 Rust 事件: SearchResults
  /// 当搜索完成并返回结果时触发
  Stream<List<LogEntry>> get searchResults => _searchResultsController.stream;

  /// 搜索摘要流
  ///
  /// 对应 Rust 事件: SearchSummary
  /// 当搜索进度更新时触发
  Stream<SearchResultSummary> get searchSummary =>
      _searchSummaryController.stream;

  /// 任务更新流
  ///
  /// 对应 Rust 事件: TaskUpdate
  /// 当任务状态变化时触发
  Stream<TaskProgress> get taskUpdate => _taskUpdateController.stream;

  /// 文件变化流
  ///
  /// 对应 Rust 事件: FileChanged
  /// 当监听的文件发生变化时触发
  Stream<FileChangeEvent> get fileChanged => _fileChangedController.stream;

  /// 工作区状态变化流
  ///
  /// 对应 Rust 事件: WorkspaceStatus
  /// 当工作区状态变化时触发
  Stream<WorkspaceStatusEvent> get workspaceStatus =>
      _workspaceStatusController.stream;

  /// 连接状态流
  ///
  /// 当与后端的连接状态变化时触发
  Stream<ConnectionStatus> get connectionStatus =>
      _connectionStatusController.stream;

  /// 错误流
  ///
  /// 当发生错误时触发
  Stream<EventError> get errors => _errorController.stream;

  /// 系统事件流
  ///
  /// 对应 Rust 事件: SystemInfo, SystemWarning, SystemError
  Stream<SystemEvent> get systemEvents => _systemEventController.stream;

  // ==================== 状态访问器 ====================

  /// 当前连接状态
  ConnectionStatus _connectionStatus = ConnectionStatus.disconnected;
  ConnectionStatus get currentConnectionStatus => _connectionStatus;

  /// 是否已连接
  bool get isConnected => _isConnected;

  // ==================== 构造函数 ====================

  /// 创建事件流服务实例
  ///
  /// [bridgeApi] flutter_rust_bridge API 实例（可选）
  /// [pollingIntervalMs] 轮询间隔，默认 500 毫秒
  EventStreamService({
    LogAnalyzerBridgeApi? bridgeApi,
    this.pollingIntervalMs = 500,
  }) : _bridgeApi = bridgeApi {
    _initialize();
  }

  /// 初始化事件流服务
  void _initialize() {
    _updateConnectionStatus(ConnectionStatus.connecting);

    // 设置 flutter_rust_bridge 流监听（如果提供了 API）
    if (_bridgeApi != null) {
      _setupBridgeListeners();
    } else {
      // 尝试使用全局实例
      try {
        // LogAnalyzerBridge.instance.api 可能在初始化后可用
        // 这里先标记为已连接，实际连接在 startPolling 时建立
        _updateConnectionStatus(ConnectionStatus.connected);
        _isConnected = true;
      } catch (e) {
        debugPrint('EventStreamService: Bridge API 不可用: $e');
        _updateConnectionStatus(ConnectionStatus.disconnected);
      }
    }

    // 在调试模式下启用模拟事件
    if (kDebugMode) {
      _simulateEvents();
    }
  }

  /// 设置 flutter_rust_bridge 流监听
  ///
  /// 注意：flutter_rust_bridge 2.x 不直接支持流，
  /// 这里预留接口以便未来扩展
  void _setupBridgeListeners() {
    _updateConnectionStatus(ConnectionStatus.connected);
    _isConnected = true;

    // 未来可以通过以下方式实现事件流：
    // 1. HTTP 长轮询
    // 2. WebSocket 连接
    // 3. 自定义 FFI 回调
  }

  // ==================== 事件处理 ====================

  /// 处理来自后端的原始事件
  ///
  /// 事件格式遵循 Rust AppEvent 的 JSON 序列化格式：
  /// ```json
  /// {
  ///   "type": "SearchResults",
  ///   "data": { ... }
  /// }
  /// ```
  ///
  /// 或带 tag 的格式：
  /// ```json
  /// {
  ///   "type": "event_type",
  ///   "data": { ... }
  /// }
  /// ```
  void handleRawEvent(String rawEvent) {
    try {
      final json = jsonDecode(rawEvent) as Map<String, dynamic>;
      _handleEvent(json);
    } catch (e) {
      debugPrint('EventStreamService: 解析事件失败: $e');
      _emitError(
        EventError(
          code: 'PARSE_ERROR',
          message: '解析事件失败: $e',
          rawEvent: rawEvent,
        ),
      );
    }
  }

  /// 处理解析后的事件
  void _handleEvent(Map<String, dynamic> event) {
    final eventType = event['type'] as String?;
    final data = event['data'];

    if (eventType == null) {
      debugPrint('EventStreamService: 事件缺少 type 字段');
      return;
    }

    // 根据 Rust AppEvent 枚举类型分发
    switch (eventType) {
      // ==================== 搜索事件 ====================
      case 'SearchStart':
        _handleSearchStart(data);
        break;
      case 'SearchProgress':
        _handleSearchProgress(data);
        break;
      case 'SearchResults':
        _handleSearchResults(data);
        break;
      case 'SearchSummary':
        _handleSearchSummary(data);
        break;
      case 'SearchComplete':
        _handleSearchComplete(data);
        break;
      case 'SearchError':
        _handleSearchError(data);
        break;

      // ==================== 异步搜索事件 ====================
      case 'AsyncSearchStart':
        _handleAsyncSearchStart(data);
        break;
      case 'AsyncSearchProgress':
        _handleAsyncSearchProgress(data);
        break;
      case 'AsyncSearchResults':
        _handleAsyncSearchResults(data);
        break;
      case 'AsyncSearchComplete':
        _handleAsyncSearchComplete(data);
        break;
      case 'AsyncSearchError':
        _handleAsyncSearchError(data);
        break;

      // ==================== 任务事件 ====================
      case 'TaskUpdate':
        _handleTaskUpdate(data);
        break;
      case 'ImportComplete':
        _handleImportComplete(data);
        break;

      // ==================== 文件监听事件 ====================
      case 'FileChanged':
        _handleFileChanged(data);
        break;
      case 'NewLogs':
        _handleNewLogs(data);
        break;

      // ==================== 系统事件 ====================
      case 'SystemError':
        _handleSystemError(data);
        break;
      case 'SystemWarning':
        _handleSystemWarning(data);
        break;
      case 'SystemInfo':
        _handleSystemInfo(data);
        break;

      default:
        debugPrint('EventStreamService: 未知事件类型: $eventType');
    }
  }

  // ==================== 搜索事件处理器 ====================

  void _handleSearchStart(dynamic data) {
    final message = data is Map ? data['message'] as String? : data?.toString();
    _emitSystemEvent(
      SystemEvent(
        type: SystemEventType.info,
        message: message ?? '搜索已开始',
        context: 'SearchStart',
      ),
    );
  }

  void _handleSearchProgress(dynamic data) {
    final progress = data is Map
        ? data['progress'] as int?
        : int.tryParse(data.toString());
    if (progress != null) {
      debugPrint('EventStreamService: 搜索进度 $progress%');
    }
  }

  void _handleSearchResults(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final resultsList = data['results'] as List<dynamic>?;
    if (resultsList == null) return;

    try {
      final results = resultsList
          .map((json) => LogEntry.fromJson(json as Map<String, dynamic>))
          .toList();
      emitSearchResults(results);
    } catch (e) {
      debugPrint('EventStreamService: 解析搜索结果失败: $e');
    }
  }

  void _handleSearchSummary(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final summary = data['summary'] as Map<String, dynamic>?;
    if (summary == null) return;

    try {
      final result = SearchResultSummary.fromJson(summary);
      emitSearchSummary(result);
    } catch (e) {
      debugPrint('EventStreamService: 解析搜索摘要失败: $e');
    }
  }

  void _handleSearchComplete(dynamic data) {
    final count = data is Map
        ? data['count'] as int?
        : int.tryParse(data.toString());
    _emitSystemEvent(
      SystemEvent(
        type: SystemEventType.info,
        message: '搜索完成，共 $count 条结果',
        context: 'SearchComplete',
      ),
    );
  }

  void _handleSearchError(dynamic data) {
    final error = data is Map ? data['error'] as String? : data?.toString();
    _emitError(EventError(code: 'SEARCH_ERROR', message: error ?? '搜索出错'));
  }

  // ==================== 异步搜索事件处理器 ====================

  void _handleAsyncSearchStart(dynamic data) {
    if (data is! Map<String, dynamic>) return;
    final searchId = data['search_id'] as String?;
    debugPrint('EventStreamService: 异步搜索已开始, ID: $searchId');
  }

  void _handleAsyncSearchProgress(dynamic data) {
    if (data is! Map<String, dynamic>) return;
    final searchId = data['search_id'] as String?;
    final progress = data['progress'] as int?;
    debugPrint('EventStreamService: 异步搜索进度, ID: $searchId, 进度: $progress%');
  }

  void _handleAsyncSearchResults(dynamic data) {
    _handleSearchResults(data);
  }

  void _handleAsyncSearchComplete(dynamic data) {
    if (data is! Map<String, dynamic>) return;
    final searchId = data['search_id'] as String?;
    final count = data['count'] as int?;
    debugPrint('EventStreamService: 异步搜索完成, ID: $searchId, 共 $count 条结果');
  }

  void _handleAsyncSearchError(dynamic data) {
    if (data is! Map<String, dynamic>) return;
    final searchId = data['search_id'] as String?;
    final error = data['error'] as String?;
    _emitError(
      EventError(
        code: 'ASYNC_SEARCH_ERROR',
        message: error ?? '异步搜索出错',
        context: searchId,
      ),
    );
  }

  // ==================== 任务事件处理器 ====================

  void _handleTaskUpdate(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final progressData = data['progress'] as Map<String, dynamic>?;
    if (progressData == null) return;

    try {
      final progress = TaskProgress.fromJson(progressData);
      emitTaskUpdate(progress);
    } catch (e) {
      debugPrint('EventStreamService: 解析任务进度失败: $e');
    }
  }

  void _handleImportComplete(dynamic data) {
    final taskId = data is Map ? data['task_id'] as String? : data?.toString();
    _emitSystemEvent(
      SystemEvent(type: SystemEventType.info, message: '导入完成', context: taskId),
    );
  }

  // ==================== 文件监听事件处理器 ====================

  void _handleFileChanged(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final eventData = data['event'] as Map<String, dynamic>?;
    if (eventData == null) return;

    try {
      final event = FileChangeEvent.fromJson(eventData);
      emitFileChanged(event);
    } catch (e) {
      debugPrint('EventStreamService: 解析文件变化事件失败: $e');
    }
  }

  void _handleNewLogs(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final entries = data['entries'] as List<dynamic>?;
    if (entries == null) return;

    try {
      final logEntries = entries
          .map((json) => LogEntry.fromJson(json as Map<String, dynamic>))
          .toList();
      emitSearchResults(logEntries);
    } catch (e) {
      debugPrint('EventStreamService: 解析新日志失败: $e');
    }
  }

  // ==================== 系统事件处理器 ====================

  void _handleSystemError(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final error = data['error'] as String?;
    final context = data['context'] as String?;

    _emitError(
      EventError(
        code: 'SYSTEM_ERROR',
        message: error ?? '系统错误',
        context: context,
      ),
    );
  }

  void _handleSystemWarning(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final warning = data['warning'] as String?;
    final context = data['context'] as String?;

    _emitSystemEvent(
      SystemEvent(
        type: SystemEventType.warning,
        message: warning ?? '系统警告',
        context: context,
      ),
    );
  }

  void _handleSystemInfo(dynamic data) {
    if (data is! Map<String, dynamic>) return;

    final info = data['info'] as String?;
    final context = data['context'] as String?;

    _emitSystemEvent(
      SystemEvent(
        type: SystemEventType.info,
        message: info ?? '系统信息',
        context: context,
      ),
    );
  }

  // ==================== 公开事件发射方法 ====================

  /// 发送搜索结果事件
  void emitSearchResults(List<LogEntry> results) {
    if (!_searchResultsController.isClosed) {
      _searchResultsController.add(results);
    }
  }

  /// 发送搜索摘要事件
  void emitSearchSummary(SearchResultSummary summary) {
    if (!_searchSummaryController.isClosed) {
      _searchSummaryController.add(summary);
    }
  }

  /// 发送任务更新事件
  void emitTaskUpdate(TaskProgress progress) {
    if (!_taskUpdateController.isClosed) {
      _taskUpdateController.add(progress);
    }
  }

  /// 发送文件变化事件
  void emitFileChanged(FileChangeEvent event) {
    if (!_fileChangedController.isClosed) {
      _fileChangedController.add(event);
    }
  }

  /// 发送工作区状态变化事件
  void emitWorkspaceStatus(WorkspaceStatusEvent event) {
    if (!_workspaceStatusController.isClosed) {
      _workspaceStatusController.add(event);
    }
  }

  // ==================== 轮询机制 ====================

  /// 启动轮询
  ///
  /// 定期获取任务状态更新
  void startPolling() {
    _pollingTimer?.cancel();
    _pollingTimer = Timer.periodic(
      Duration(milliseconds: pollingIntervalMs),
      (_) => _pollTaskUpdates(),
    );
  }

  /// 停止轮询
  void stopPolling() {
    _pollingTimer?.cancel();
    _pollingTimer = null;
  }

  /// 轮询任务更新
  Future<void> _pollTaskUpdates() async {
    if (_bridgeApi == null) return;

    try {
      // 获取任务指标
      final metrics = await _bridgeApi!
          .crateFfiCommandsBridgeFfiGetTaskMetrics();
      // 将指标转换为任务更新事件
      // 这里可以根据需要进一步处理
      debugPrint(
        'EventStreamService: 任务指标 - 总数: ${metrics.totalTasks}, 运行中: ${metrics.runningTasks}',
      );
    } catch (e) {
      debugPrint('EventStreamService: 轮询任务更新失败: $e');
    }
  }

  // ==================== 辅助方法 ====================

  /// 更新连接状态
  void _updateConnectionStatus(ConnectionStatus status) {
    _connectionStatus = status;
    if (!_connectionStatusController.isClosed) {
      _connectionStatusController.add(status);
    }
  }

  /// 发送错误事件
  void _emitError(EventError error) {
    if (!_errorController.isClosed) {
      _errorController.add(error);
    }
  }

  /// 发送系统事件
  void _emitSystemEvent(SystemEvent event) {
    if (!_systemEventController.isClosed) {
      _systemEventController.add(event);
    }
  }

  /// 模拟事件流（调试用）
  void _simulateEvents() {
    // 仅在调试模式下使用
    // 模拟连接状态变化
    Future.delayed(const Duration(seconds: 1), () {
      _updateConnectionStatus(ConnectionStatus.connected);
    });
  }

  // ==================== 生命周期 ====================

  /// 释放资源
  void dispose() {
    stopPolling();

    _searchResultsController.close();
    _searchSummaryController.close();
    _taskUpdateController.close();
    _fileChangedController.close();
    _workspaceStatusController.close();
    _connectionStatusController.close();
    _errorController.close();
    _systemEventController.close();
  }
}

// ==================== 数据类型定义 ====================

/// 连接状态枚举
enum ConnectionStatus {
  /// 正在连接
  connecting,

  /// 已连接
  connected,

  /// 已断开
  disconnected,

  /// 正在重连
  reconnecting,

  /// 连接错误
  error,
}

/// 工作区状态事件
class WorkspaceStatusEvent {
  /// 工作区 ID
  final String workspaceId;

  /// 状态值
  final String status;

  /// 附加消息
  final String? message;

  const WorkspaceStatusEvent({
    required this.workspaceId,
    required this.status,
    this.message,
  });

  @override
  String toString() =>
      'WorkspaceStatusEvent(workspaceId: $workspaceId, status: $status)';
}

/// 事件错误
class EventError {
  /// 错误代码
  final String code;

  /// 错误消息
  final String message;

  /// 上下文信息
  final String? context;

  /// 原始事件数据（如果有）
  final String? rawEvent;

  const EventError({
    required this.code,
    required this.message,
    this.context,
    this.rawEvent,
  });

  @override
  String toString() => 'EventError(code: $code, message: $message)';
}

/// 系统事件
class SystemEvent {
  /// 事件类型
  final SystemEventType type;

  /// 事件消息
  final String message;

  /// 上下文信息
  final String? context;

  const SystemEvent({required this.type, required this.message, this.context});

  @override
  String toString() => 'SystemEvent(type: $type, message: $message)';
}

/// 系统事件类型
enum SystemEventType {
  /// 信息
  info,

  /// 警告
  warning,

  /// 错误
  error,
}

// ==================== Provider ====================

/// 事件流服务 Provider
///
/// 使用 Riverpod 管理单例
final eventStreamServiceProvider = Provider<EventStreamService>((ref) {
  final service = EventStreamService();

  // 确保在 Provider 销毁时释放资源
  ref.onDispose(() {
    service.dispose();
  });

  return service;
});

/// 连接状态 Provider
final connectionStatusProvider = StreamProvider<ConnectionStatus>((ref) {
  final service = ref.watch(eventStreamServiceProvider);
  return service.connectionStatus;
});

/// 任务更新 Provider
final taskUpdateProvider = StreamProvider<TaskProgress>((ref) {
  final service = ref.watch(eventStreamServiceProvider);
  return service.taskUpdate;
});

/// 搜索结果 Provider
final searchResultsProvider = StreamProvider<List<LogEntry>>((ref) {
  final service = ref.watch(eventStreamServiceProvider);
  return service.searchResults;
});

/// 系统事件 Provider
final systemEventProvider = StreamProvider<SystemEvent>((ref) {
  final service = ref.watch(eventStreamServiceProvider);
  return service.systemEvents;
});

/// 错误 Provider
final eventErrorProvider = StreamProvider<EventError>((ref) {
  final service = ref.watch(eventStreamServiceProvider);
  return service.errors;
});
