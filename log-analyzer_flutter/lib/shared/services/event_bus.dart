import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// 事件处理器类型
typedef EventHandler<T> = Future<void> Function(T data);

/// 事件总线
///
/// 对应 React 版本的 EventBus.ts
/// 提供事件验证、幂等性保证、事件分发功能
class EventBus {

  /// 已处理的事件记录（幂等性检查）
  final Map<String, ProcessedEvent> _processedEvents = {};

  /// 事件处理器映射
  final Map<String, EventHandler<dynamic>> _handlers = {};

  /// 注册事件处理器
  void on<T>(String eventType, EventHandler<T> handler) {
    _handlers[eventType] = (dynamic data) => handler(data as T);
  }

  /// 取消事件处理器
  void off(String eventType) {
    _handlers.remove(eventType);
  }

  /// 处理事件（带幂等性检查）
  ///
  /// 对应 React 版本的 processEvent()
  Future<void> processEvent(String eventType, dynamic rawData) async {
    // 幂等性检查
    if (rawData is Map<String, dynamic>) {
      final data = rawData;

      // 检查是否包含版本号字段
      if (data.containsKey('version') && data.containsKey('task_id')) {
        final key = '${eventType}_${data['task_id']}';
        final version = data['version'] as int;

        final processed = _processedEvents[key];
        if (processed != null && processed.version >= version) {
          // 跳过重复或旧版本事件
          return;
        }

        // 记录已处理的事件
        _processedEvents[key] = ProcessedEvent(
          taskId: data['task_id'] as String,
          version: version,
          timestamp: DateTime.now().millisecondsSinceEpoch,
        );
      }
    }

    // 触发处理器
    final handler = _handlers[eventType];
    if (handler != null) {
      try {
        await handler(rawData);
      } catch (e) {
        // 记录错误但不中断事件处理
        debugPrint('EventBus: 处理事件 $eventType 时出错: $e');
      }
    }
  }

  /// 清理过期的事件记录
  ///
  /// 定期清理以防止内存泄漏
  void cleanupExpiredEvents({int maxAge = 3600000}) {
    // 默认清理 1 小时前的记录
    final now = DateTime.now().millisecondsSinceEpoch;
    final expiredKeys = <String>[];

    _processedEvents.forEach((key, event) {
      if (now - event.timestamp > maxAge) {
        expiredKeys.add(key);
      }
    });

    for (final key in expiredKeys) {
      _processedEvents.remove(key);
    }
  }

  /// 清除所有已处理事件记录
  void clear() {
    _processedEvents.clear();
  }

  /// 获取已处理事件数量
  int get processedEventCount => _processedEvents.length;
}

/// 已处理事件记录
class ProcessedEvent {
  final String taskId;
  final int version;
  final int timestamp;

  const ProcessedEvent({
    required this.taskId,
    required this.version,
    required this.timestamp,
  });
}

/// 事件总线 Provider
///
/// 使用 Riverpod 管理全局单例
final eventBusProvider = Provider<EventBus>((ref) {
  final bus = EventBus();

  // 设置定期清理任务
  final timer = Timer.periodic(
    const Duration(minutes: 10),
    (_) => bus.cleanupExpiredEvents(),
  );

  // 确保在 Provider 销毁时清理资源
  ref.onDispose(() {
    timer.cancel();
    bus.clear();
  });

  return bus;
});

/// 事件类型常量
class EventTypes {
  // 搜索事件
  static const String searchStart = 'search-start';
  static const String searchProgress = 'search-progress';
  static const String searchResults = 'search-results';
  static const String searchSummary = 'search-summary';
  static const String searchComplete = 'search-complete';
  static const String searchError = 'search-error';

  // 异步搜索事件
  static const String asyncSearchStart = 'async-search-start';
  static const String asyncSearchProgress = 'async-search-progress';
  static const String asyncSearchResults = 'async-search-results';
  static const String asyncSearchComplete = 'async-search-complete';
  static const String asyncSearchError = 'async-search-error';

  // 任务事件
  static const String taskUpdate = 'task-update';
  static const String taskRemoved = 'task-removed';
  static const String importComplete = 'import-complete';

  // 文件监听事件
  static const String fileChanged = 'file-changed';
  static const String newLogs = 'new-logs';

  // 工作区事件
  static const String workspaceStatusChanged = 'workspace-status-changed';
  static const String workspaceCreated = 'workspace-created';
  static const String workspaceDeleted = 'workspace-deleted';

  // 系统事件
  static const String systemError = 'system-error';
  static const String systemWarning = 'system-warning';
  static const String systemInfo = 'system-info';
}
