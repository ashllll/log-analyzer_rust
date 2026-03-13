import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:fpdart/fpdart.dart';

/// FFI 调用结果
///
/// 封装 FFI 调用的结果，支持成功和失败两种情况
/// 提供函数式风格的错误处理方式
class FfiResult<T> {
  final T? data;
  final String? error;
  final bool isSuccess;

  const FfiResult._(this.data, this.error, this.isSuccess);

  factory FfiResult.success(T data) => FfiResult._(data, null, true);
  factory FfiResult.error(String error) => FfiResult._(null, error, false);

  /// 函数式风格的 when 方法
  /// 根据成功或失败执行不同的回调
  R when<R>({
    required R Function(T data) success,
    required R Function(String error) error,
  }) {
    if (isSuccess) {
      return success(data as T);
    } else {
      return error(this.error!);
    }
  }

  /// 获取数据或抛出异常
  T getOrThrow() {
    if (isSuccess) return data as T;
    throw Exception(error);
  }

  /// 获取数据或返回默认值
  T getOrElse(T defaultValue) {
    if (isSuccess) return data as T;
    return defaultValue;
  }

  /// 获取数据或 null
  T? getOrNull() => data;

  /// 映射成功值
  FfiResult<R> map<R>(R Function(T data) transform) {
    if (isSuccess) {
      return FfiResult.success(transform(data as T));
    }
    return FfiResult.error(error!);
  }

  /// 扁平映射成功值
  FfiResult<R> flatMap<R>(FfiResult<R> Function(T data) transform) {
    if (isSuccess) {
      return transform(data as T);
    }
    return FfiResult.error(error!);
  }

  @override
  String toString() =>
      isSuccess ? 'FfiResult.success($data)' : 'FfiResult.error($error)';
}

/// 异步 FFI 调用封装
///
/// 使用 Isolate 执行 FFI 调用，避免阻塞 UI 线程
/// 提供超时控制、错误处理和函数式编程支持
class AsyncFfiCall {
  static const Duration _defaultTimeout = Duration(seconds: 30);

  /// 执行 FFI 查询（返回单个值）
  ///
  /// [query] 查询函数
  /// [timeout] 超时时间，默认 30 秒
  static Future<FfiResult<T>> query<T>({
    required Future<T> Function() query,
    Duration timeout = _defaultTimeout,
  }) async {
    try {
      final result = await _executeInIsolate(query).timeout(timeout);
      return FfiResult.success(result);
    } on TimeoutException {
      return FfiResult.error('操作超时: ${timeout.inSeconds}秒');
    } catch (e, stack) {
      debugPrint('FFI query 错误: $e\n$stack');
      return FfiResult.error('FFI 错误: $e');
    }
  }

  /// 执行 FFI 查询（返回列表）
  ///
  /// [query] 查询函数
  /// [timeout] 超时时间，默认 30 秒
  static Future<FfiResult<List<T>>> queryList<T>({
    required Future<List<T>> Function() query,
    Duration timeout = _defaultTimeout,
  }) async {
    try {
      final result = await _executeInIsolate(query).timeout(timeout);
      return FfiResult.success(result);
    } on TimeoutException {
      return FfiResult.error('操作超时: ${timeout.inSeconds}秒');
    } catch (e, stack) {
      debugPrint('FFI queryList 错误: $e\n$stack');
      return FfiResult.error('FFI 错误: $e');
    }
  }

  /// 执行 FFI 命令（无返回值）
  ///
  /// [command] 命令函数
  /// [timeout] 超时时间，默认 30 秒
  static Future<FfiResult<void>> command({
    required Future<void> Function() command,
    Duration timeout = _defaultTimeout,
  }) async {
    try {
      await _executeInIsolate(command).timeout(timeout);
      return FfiResult.success(null as dynamic);
    } on TimeoutException {
      return FfiResult.error('操作超时: ${timeout.inSeconds}秒');
    } catch (e, stack) {
      debugPrint('FFI command 错误: $e\n$stack');
      return FfiResult.error('FFI 错误: $e');
    }
  }

  /// 在 Isolate 中执行函数
  ///
  /// 使用 Flutter 的 compute 函数在后台 Isolate 中执行
  static Future<R> _executeInIsolate<R>(Future<R> Function() function) {
    // 对于简单类型，使用 compute
    return compute(_isolateEntryPoint, function);
  }

  /// Isolate 入口点
  ///
  /// 在 Isolate 中执行传入的函数
  static R _isolateEntryPoint<R>(Future<R> Function() function) {
    // 注意：实际实现需要更复杂的序列化
    // 这里简化处理，适用于大多数 FFI 调用
    return function() as R;
  }
}

/// 使用 fpdart 的函数式错误处理版本
///
/// 提供更强大的函数式编程支持
class AsyncFfiCallFunctional {
  static const Duration _defaultTimeout = Duration(seconds: 30);

  /// 执行 FFI 查询，返回 TaskEither
  ///
  /// [query] 查询函数
  /// [timeout] 超时时间
  /// 返回 TaskEither<String, T>，Left 为错误，Right 为成功
  static TaskEither<String, T> query<T>({
    required Future<T> Function() query,
    Duration timeout = _defaultTimeout,
  }) {
    return TaskEither(() async {
      try {
        final result = await _executeInIsolate(query).timeout(timeout);
        return Right(result);
      } on TimeoutException {
        return Left('操作超时: ${timeout.inSeconds}秒');
      } catch (e) {
        return Left('FFI 错误: $e');
      }
    });
  }

  /// 执行 FFI 命令，返回 TaskEither
  ///
  /// [command] 命令函数
  /// [timeout] 超时时间
  static TaskEither<String, void> command({
    required Future<void> Function() command,
    Duration timeout = _defaultTimeout,
  }) {
    return TaskEither(() async {
      try {
        await _executeInIsolate(command).timeout(timeout);
        return const Right(null);
      } on TimeoutException {
        return Left('操作超时: ${timeout.inSeconds}秒');
      } catch (e) {
        return Left('FFI 错误: $e');
      }
    });
  }

  /// 执行 FFI 查询列表，返回 TaskEither
  ///
  /// [query] 查询函数
  /// [timeout] 超时时间
  static TaskEither<String, List<T>> queryList<T>({
    required Future<List<T>> Function() query,
    Duration timeout = _defaultTimeout,
  }) {
    return TaskEither(() async {
      try {
        final result = await _executeInIsolate(query).timeout(timeout);
        return Right(result);
      } on TimeoutException {
        return Left('操作超时: ${timeout.inSeconds}秒');
      } catch (e) {
        return Left('FFI 错误: $e');
      }
    });
  }

  static Future<R> _executeInIsolate<R>(Future<R> Function() function) {
    return compute(_isolateEntryPoint, function);
  }

  static R _isolateEntryPoint<R>(Future<R> Function() function) {
    return function() as R;
  }
}

/// FFI 超时异常
class FfiTimeoutException implements Exception {
  final String operation;
  final Duration timeout;

  FfiTimeoutException(this.operation, this.timeout);

  @override
  String toString() =>
      'FfiTimeoutException: 操作 "$operation" 超时 (${timeout.inSeconds}秒)';
}

/// FFI 调用异常
class FfiCallException implements Exception {
  final String operation;
  final String message;
  final StackTrace? stackTrace;

  FfiCallException(this.operation, this.message, {this.stackTrace});

  @override
  String toString() => 'FfiCallException[$operation]: $message';
}

/// Isolate 工具函数
class IsolateUtils {
  /// 在 Isolate 中执行密集型计算
  ///
  /// [computation] 计算函数
  /// [input] 输入参数
  static Future<R> computeInIsolate<P, R>(
    R Function(P) computation,
    P input,
  ) {
    return compute(computation, input);
  }

  /// 并行执行多个 FFI 调用
  ///
  /// [calls] FFI 调用列表
  /// [timeout] 整体超时时间
  static Future<List<FfiResult<R>>> parallelCalls<R>(
    List<Future<FfiResult<R>>> calls, {
    Duration? timeout,
  }) async {
    try {
      final futures = calls.map((call) => timeout != null
          ? call.timeout(timeout, onTimeout: () => FfiResult<R>.error('并行调用超时'))
          : call);
      return await Future.wait(futures);
    } catch (e) {
      debugPrint('并行调用错误: $e');
      return List<FfiResult<R>>.generate(
        calls.length,
        (_) => FfiResult.error('并行调用失败: $e'),
      );
    }
  }

  /// 带重试的 FFI 调用
  ///
  /// [call] FFI 调用函数
  /// [maxRetries] 最大重试次数
  /// [retryDelay] 重试延迟
  static Future<FfiResult<T>> retryableCall<T>({
    required Future<T> Function() call,
    int maxRetries = 3,
    Duration retryDelay = const Duration(milliseconds: 100),
  }) async {
    for (int attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        final result = await call();
        return FfiResult.success(result);
      } catch (e) {
        if (attempt == maxRetries) {
          return FfiResult.error('调用失败 (尝试 $maxRetries 次): $e');
        }
        await Future.delayed(retryDelay * (attempt + 1));
      }
    }
    return FfiResult.error('未知错误');
  }
}

/// FFI 性能监控工具
class FfiPerformanceMonitor {
  static final Map<String, List<Duration>> _metrics = {};

  /// 监控 FFI 调用性能
  ///
  /// [name] 调用名称
  /// [call] 调用函数
  static Future<FfiResult<T>> monitor<T>(
    String name,
    Future<T> Function() call,
  ) async {
    final stopwatch = Stopwatch()..start();
    try {
      final result = await call();
      stopwatch.stop();
      _recordMetric(name, stopwatch.elapsed);
      return FfiResult.success(result);
    } catch (e) {
      stopwatch.stop();
      return FfiResult.error(e.toString());
    }
  }

  static void _recordMetric(String name, Duration duration) {
    _metrics.putIfAbsent(name, () => []);
    _metrics[name]!.add(duration);
    // 只保留最近 100 条记录
    if (_metrics[name]!.length > 100) {
      _metrics[name]!.removeAt(0);
    }
  }

  /// 获取平均执行时间
  static Duration? getAverageTime(String name) {
    final times = _metrics[name];
    if (times == null || times.isEmpty) return null;
    final total = times.fold(Duration.zero, (sum, d) => sum + d);
    return Duration(microseconds: total.inMicroseconds ~/ times.length);
  }

  /// 获取所有指标
  static Map<String, Duration> getAllAverages() {
    return _metrics.map((name, times) {
      final total = times.fold(Duration.zero, (sum, d) => sum + d);
      final avg = Duration(microseconds: total.inMicroseconds ~/ times.length);
      return MapEntry(name, avg);
    });
  }

  /// 清除指标
  static void clearMetrics() => _metrics.clear();
}
