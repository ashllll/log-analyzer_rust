import 'dart:async';
import 'dart:collection';

import 'package:flutter_riverpod/legacy.dart';

import '../../../shared/models/common.dart';
import '../../../shared/services/api_service.dart';
import '../../../shared/services/event_stream_service.dart';
import '../models/monitoring_state.dart';

/// 监控状态 Provider
final monitoringProvider =
    StateNotifierProvider<MonitoringNotifier, MonitoringState>((ref) {
      return MonitoringNotifier();
    });

/// 监控状态管理器
///
/// 负责启动/停止监控、处理文件变化事件、实现防抖和限流逻辑
class MonitoringNotifier extends StateNotifier<MonitoringState> {
  MonitoringNotifier() : super(const MonitoringState()) {
    _listenToFileChanges();
  }

  final ApiService _apiService = ApiService();
  StreamSubscription<FileChangeEvent>? _fileChangeSubscription;

  /// 待处理事件队列
  final Queue<FileChangeEvent> _pendingQueue = Queue();

  /// 队列最大长度
  static const int maxQueueLength = 1000;

  /// 防抖定时器
  Timer? _debounceTimer;

  /// 防抖延迟（毫秒）
  static const int debounceDelayMs = 500;

  /// 限流计数器
  int _rateLimitCounter = 0;

  /// 限流周期（秒）
  static const int rateLimitPeriodSec = 1;

  /// 每秒最大处理数
  static const int maxEventsPerSecond = 10;

  /// 限流定时器
  Timer? _rateLimitTimer;

  /// 重试配置
  static const List<int> retryDelays = [100, 200, 400];

  /// 监听文件变化事件
  void _listenToFileChanges() {
    final eventStream = EventStreamService();
    _fileChangeSubscription = eventStream.fileChanged.listen((event) {
      onFileChanged(event);
    });

    // 启动限流定时器
    _rateLimitTimer = Timer.periodic(
      const Duration(seconds: rateLimitPeriodSec),
      (_) {
        _rateLimitCounter = 0;
      },
    );
  }

  /// 启动监控
  ///
  /// [workspaceId] 工作区ID
  /// [paths] 要监控的路径列表
  Future<void> startMonitoring(String workspaceId, List<String> paths) async {
    try {
      // 调用后端启动监控
      await _retryCall(
        () => _apiService.startWatch(workspaceId: workspaceId, paths: paths),
      );

      // 更新状态
      state = state.copyWith(
        isActive: true,
        workspaceId: workspaceId,
        monitoredDirsCount: paths.length,
        monitoredFilesCount: 0, // TODO: 从后端获取实际文件数
        errorMessage: null,
        lastUpdate: DateTime.now(),
      );
    } catch (e) {
      state = state.copyWith(
        isActive: false,
        errorMessage: '启动监控失败: $e',
        lastUpdate: DateTime.now(),
      );
    }
  }

  /// 停止监控
  ///
  /// [workspaceId] 工作区ID
  Future<void> stopMonitoring(String workspaceId) async {
    try {
      // 调用后端停止监控
      await _retryCall(() => _apiService.stopWatch(workspaceId));

      // 清理队列
      _pendingQueue.clear();
      _debounceTimer?.cancel();

      // 更新状态
      state = state.copyWith(
        isActive: false,
        pendingCount: 0,
        errorMessage: null,
        lastUpdate: DateTime.now(),
      );
    } catch (e) {
      state = state.copyWith(
        errorMessage: '停止监控失败: $e',
        lastUpdate: DateTime.now(),
      );
    }
  }

  /// 处理文件变化事件
  void onFileChanged(FileChangeEvent event) {
    // 添加到队列
    addToQueue(event);

    // 更新统计
    state = state.copyWith(
      eventsProcessed: state.eventsProcessed + 1,
      lastUpdate: DateTime.now(),
    );
  }

  /// 将事件加入待处理队列
  ///
  /// 实现 500ms 防抖和限流逻辑
  void addToQueue(FileChangeEvent event) {
    // 如果队列已满，丢弃旧请求
    if (_pendingQueue.length >= maxQueueLength) {
      _pendingQueue.removeFirst();
    }

    _pendingQueue.add(event);
    state = state.copyWith(pendingCount: _pendingQueue.length);

    // 取消之前的防抖定时器
    _debounceTimer?.cancel();

    // 设置新的防抖定时器
    _debounceTimer = Timer(const Duration(milliseconds: debounceDelayMs), () {
      _processQueue();
    });
  }

  /// 处理队列中的事件
  void _processQueue() {
    // 检查限流
    while (_pendingQueue.isNotEmpty && _rateLimitCounter < maxEventsPerSecond) {
      final event = _pendingQueue.removeFirst();
      _handleEvent(event);
      _rateLimitCounter++;
    }

    // 更新队列长度
    state = state.copyWith(pendingCount: _pendingQueue.length);
  }

  /// 处理单个事件
  void _handleEvent(FileChangeEvent event) {
    switch (event.eventType) {
      case 'create':
        // TODO: 调用索引添加文件
        break;
      case 'modify':
        // TODO: 更新索引
        break;
      case 'remove':
        // TODO: 从索引移除
        break;
    }
  }

  /// 带重试的调用
  Future<void> _retryCall(Future<void> Function() call) async {
    for (int i = 0; i <= retryDelays.length; i++) {
      try {
        await call();
        return;
      } catch (e) {
        if (i < retryDelays.length) {
          await Future.delayed(Duration(milliseconds: retryDelays[i]));
        }
      }
    }
    // 最后一次尝试失败后抛出异常
    try {
      await call();
    } catch (e) {
      rethrow;
    }
  }

  @override
  void dispose() {
    _fileChangeSubscription?.cancel();
    _debounceTimer?.cancel();
    _rateLimitTimer?.cancel();
    super.dispose();
  }
}
