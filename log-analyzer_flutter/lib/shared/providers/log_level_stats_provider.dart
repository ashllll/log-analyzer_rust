import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/bridge_service.dart';

part 'log_level_stats_provider.g.dart';

/// 日志级别统计数据模型
///
/// 用于在 Flutter 端展示日志级别统计信息
class LogLevelStats {
  /// FATAL 级别日志数量
  final int fatalCount;

  /// ERROR 级别日志数量
  final int errorCount;

  /// WARN 级别日志数量
  final int warnCount;

  /// INFO 级别日志数量
  final int infoCount;

  /// DEBUG 级别日志数量
  final int debugCount;

  /// TRACE 级别日志数量
  final int traceCount;

  /// 未知级别日志数量
  final int unknownCount;

  /// 总日志数量
  final int total;

  const LogLevelStats({
    required this.fatalCount,
    required this.errorCount,
    required this.warnCount,
    required this.infoCount,
    required this.debugCount,
    required this.traceCount,
    required this.unknownCount,
    required this.total,
  });

  /// 从 Map 创建
  factory LogLevelStats.fromMap(Map<String, dynamic>? map) {
    if (map == null) {
      return LogLevelStats.empty;
    }
    return LogLevelStats(
      fatalCount: (map['fatalCount'] as num?)?.toInt() ?? 0,
      errorCount: (map['errorCount'] as num?)?.toInt() ?? 0,
      warnCount: (map['warnCount'] as num?)?.toInt() ?? 0,
      infoCount: (map['infoCount'] as num?)?.toInt() ?? 0,
      debugCount: (map['debugCount'] as num?)?.toInt() ?? 0,
      traceCount: (map['traceCount'] as num?)?.toInt() ?? 0,
      unknownCount: (map['unknownCount'] as num?)?.toInt() ?? 0,
      total: (map['total'] as num?)?.toInt() ?? 0,
    );
  }

  /// 空统计数据
  static const empty = LogLevelStats(
    fatalCount: 0,
    errorCount: 0,
    warnCount: 0,
    infoCount: 0,
    debugCount: 0,
    traceCount: 0,
    unknownCount: 0,
    total: 0,
  );

  @override
  String toString() {
    return 'LogLevelStats(fatal: $fatalCount, error: $errorCount, warn: $warnCount, '
        'info: $infoCount, debug: $debugCount, trace: $traceCount, '
        'unknown: $unknownCount, total: $total)';
  }
}

/// BridgeService Provider
///
/// 提供 BridgeService 单例访问
@riverpod
BridgeService bridgeService(Ref ref) {
  return BridgeService.instance;
}

/// 日志级别统计 Provider
///
/// 使用 Riverpod 3.0 AsyncNotifier 管理日志级别统计状态
/// 支持 workspaceId 参数化，切换工作区时自动刷新
/// 支持 5 秒自动刷新（STATS-03 实时更新）
@riverpod
class LogLevelStatsNotifier extends _$LogLevelStatsNotifier {
  Timer? _refreshTimer;

  @override
  AsyncValue<LogLevelStats> build(String workspaceId) {
    // 初始加载统计数据（延迟执行避免在 build 中直接调用异步方法）
    Future.microtask(() => _loadStats());

    // 设置 5 秒自动刷新（STATS-03 实时更新）
    _setupAutoRefresh();

    // 当 workspaceId 变化时，重新加载
    ref.onDispose(() {
      _refreshTimer?.cancel();
    });

    // 返回初始加载状态
    return const AsyncLoading();
  }

  /// 设置自动刷新
  void _setupAutoRefresh() {
    _refreshTimer?.cancel();
    _refreshTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => _loadStats(),
    );
  }

  /// 加载日志级别统计
  ///
  /// 从 FFI 获取指定工作区的日志级别统计
  Future<void> _loadStats() async {
    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回空统计
      if (!bridge.isFfiEnabled) {
        debugPrint('LogLevelStatsProvider: FFI 未初始化，返回空统计');
        state = const AsyncData(LogLevelStats.empty);
        return;
      }

      // 获取日志级别统计
      final statsMap = await bridge.getLogLevelStats(workspaceId);

      final stats = LogLevelStats.fromMap(statsMap);
      state = AsyncData(stats);
      debugPrint('LogLevelStatsProvider: 已加载统计 - $stats');
    } catch (e) {
      debugPrint('LogLevelStatsProvider: 加载统计失败: $e');
      // FFI 调用失败时返回空统计
      state = const AsyncData(LogLevelStats.empty);
    }
  }

  /// 刷新日志级别统计
  ///
  /// 重新从后端加载统计数据
  Future<void> refresh() async {
    await _loadStats();
  }

  /// 停止自动刷新
  void stopAutoRefresh() {
    _refreshTimer?.cancel();
    _refreshTimer = null;
  }

  /// 重新启动自动刷新
  void startAutoRefresh() {
    if (_refreshTimer == null) {
      _setupAutoRefresh();
    }
  }
}
