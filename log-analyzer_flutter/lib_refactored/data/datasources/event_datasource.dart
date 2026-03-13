/// 事件数据源
/// 
/// 负责与后端的实时事件通信
/// 替代传统的轮询机制，使用事件驱动架构

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../../domain/entities/task.dart';
import '../../domain/entities/log_entry.dart';

/// 事件类型
enum EventType {
  /// 任务创建
  taskCreated,
  /// 任务进度
  taskProgress,
  /// 任务完成
  taskCompleted,
  /// 任务取消
  taskCancelled,
  /// 搜索结果
  searchResults,
  /// 搜索摘要
  searchSummary,
  /// 文件系统事件
  fileSystemEvent,
  /// 工作区更新
  workspaceUpdated,
}

/// 原始事件数据
class RawEvent {
  final EventType type;
  final Map<String, dynamic> data;
  final DateTime timestamp;

  const RawEvent({
    required this.type,
    required this.data,
    required this.timestamp,
  });
}

/// 事件数据源
/// 
/// 管理所有实时事件流
/// 使用单例模式确保全局唯一
class EventDataSource {
  static EventDataSource? _instance;
  
  // 事件流控制器
  final _taskEventController = StreamController<TaskEvent>.broadcast();
  final _searchResultController = StreamController<SearchResult>.broadcast();
  final _rawEventController = StreamController<RawEvent>.broadcast();
  
  // 重连控制
  Timer? _reconnectTimer;
  bool _isConnected = false;
  int _reconnectAttempts = 0;
  static const _maxReconnectAttempts = 5;
  static const _reconnectDelay = Duration(seconds: 5);

  EventDataSource._();

  static EventDataSource get instance {
    _instance ??= EventDataSource._();
    return _instance!;
  }

  // ==================== 公共流访问 ====================

  /// 任务事件流
  Stream<TaskEvent> get taskEvents => _taskEventController.stream;

  /// 搜索结果流
  Stream<SearchResult> get searchResults => _searchResultController.stream;

  /// 原始事件流（用于调试）
  Stream<RawEvent> get rawEvents => _rawEventController.stream;

  // ==================== 连接管理 ====================

  /// 初始化连接
  /// 
  /// 在应用启动时调用
  void initialize() {
    if (_isConnected) return;
    
    _connect();
    
    // 启动心跳检查
    _startHeartbeat();
  }

  /// 连接事件源
  void _connect() {
    _isConnected = true;
    _reconnectAttempts = 0;
    
    // TODO: 实现与后端的事件连接
    // 这里可以使用 WebSocket、Server-Sent Events 或 FFI 回调
    
    debugPrint('事件数据源已连接');
  }

  /// 断开连接
  void disconnect() {
    _isConnected = false;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    
    debugPrint('事件数据源已断开');
  }

  /// 重新连接
  void _reconnect() {
    if (_reconnectAttempts >= _maxReconnectAttempts) {
      debugPrint('重连次数超过限制，停止重连');
      return;
    }

    _reconnectAttempts++;
    debugPrint('尝试重连 ($_reconnectAttempts/$_maxReconnectAttempts)...');

    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(_reconnectDelay * _reconnectAttempts, () {
      _connect();
    });
  }

  /// 启动心跳检查
  void _startHeartbeat() {
    // 定期发送心跳以保持连接
    Timer.periodic(const Duration(seconds: 30), (timer) {
      if (!_isConnected) {
        timer.cancel();
        return;
      }
      // TODO: 发送心跳
    });
  }

  // ==================== 事件发布（供 FFI 回调使用） ====================

  /// 发布任务事件
  void publishTaskEvent(TaskEvent event) {
    if (!_taskEventController.isClosed) {
      _taskEventController.add(event);
    }
    
    // 同时发布到原始事件流
    _publishRawEvent(EventType.taskProgress, {
      'taskId': event.taskId,
      'type': event.runtimeType.toString(),
    });
  }

  /// 发布搜索结果
  void publishSearchResult(SearchResult result) {
    if (!_searchResultController.isClosed) {
      _searchResultController.add(result);
    }
  }

  /// 发布原始事件
  void _publishRawEvent(EventType type, Map<String, dynamic> data) {
    if (!_rawEventController.isClosed) {
      _rawEventController.add(RawEvent(
        type: type,
        data: data,
        timestamp: DateTime.now(),
      ));
    }
  }

  // ==================== 事件处理 ====================

  /// 处理来自 FFI 的任务事件
  void handleFfiTaskEvent(Map<String, dynamic> data) {
    try {
      final taskId = data['task_id'] as String? ?? '';
      final eventType = data['event_type'] as String? ?? '';
      final timestamp = DateTime.fromMillisecondsSinceEpoch(
        data['timestamp'] as int? ?? DateTime.now().millisecondsSinceEpoch,
      );

      final event = switch (eventType) {
        'created' => TaskCreatedEvent(
            taskId: taskId,
            task: _parseTask(data['task']),
            timestamp: timestamp,
          ),
        'progress' => TaskProgressEvent(
            taskId: taskId,
            progress: (data['progress'] as num?)?.toDouble() ?? 0.0,
            message: data['message'] as String?,
            timestamp: timestamp,
          ),
        'completed' => TaskCompletedEvent(
            taskId: taskId,
            success: data['success'] as bool? ?? false,
            error: data['error'] as String?,
            result: data['result'],
            timestamp: timestamp,
          ),
        'cancelled' => TaskCancelledEvent(
            taskId: taskId,
            timestamp: timestamp,
          ),
        _ => null,
      };

      if (event != null) {
        publishTaskEvent(event);
      }
    } catch (e, stack) {
      debugPrint('处理任务事件失败: $e\n$stack');
    }
  }

  /// 处理来自 FFI 的搜索结果
  void handleFfiSearchResult(Map<String, dynamic> data) {
    try {
      final result = SearchResult(
        searchId: data['search_id'] as String? ?? '',
        totalMatches: data['total_matches'] as int? ?? 0,
        scannedFiles: data['scanned_files'] as int? ?? 0,
        durationMs: data['duration_ms'] as int? ?? 0,
        isComplete: data['is_complete'] as bool? ?? false,
        entries: _parseLogEntries(data['entries']),
      );

      publishSearchResult(result);
    } catch (e, stack) {
      debugPrint('处理搜索结果失败: $e\n$stack');
    }
  }

  // ==================== 辅助方法 ====================

  Task _parseTask(dynamic data) {
    // 解析任务数据
    // TODO: 实现完整的解析逻辑
    return Task.empty;
  }

  List<LogEntry> _parseLogEntries(dynamic data) {
    if (data == null) return [];
    if (data is! List) return [];
    
    // TODO: 实现完整的解析逻辑
    return [];
  }

  /// 释放资源
  void dispose() {
    disconnect();
    _taskEventController.close();
    _searchResultController.close();
    _rawEventController.close();
  }
}

/// 事件订阅管理器
/// 
/// 管理多个事件订阅，支持批量取消
class EventSubscriptionManager {
  final List<StreamSubscription> _subscriptions = [];

  /// 添加订阅
  void add(StreamSubscription subscription) {
    _subscriptions.add(subscription);
  }

  /// 取消所有订阅
  void cancelAll() {
    for (final sub in _subscriptions) {
      sub.cancel();
    }
    _subscriptions.clear();
  }

  /// 获取订阅数量
  int get count => _subscriptions.length;
}
