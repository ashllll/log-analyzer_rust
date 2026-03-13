/// Isolate 工具类
/// 
/// 提供异步 FFI 调用所需的 Isolate 管理
/// 参考 flutter_rust_bridge 的 Isolate 模式

import 'dart:async';
import 'dart:isolate';
import 'package:flutter/foundation.dart';
import '../errors/app_error.dart';

/// Isolate 执行的消息
class _IsolateMessage<T, R> {
  final SendPort sendPort;
  final T argument;
  final FutureOr<R> Function(T) worker;

  _IsolateMessage({
    required this.sendPort,
    required this.argument,
    required this.worker,
  });
}

/// Isolate 工具类
/// 
/// 封装了 Isolate 的创建、通信和销毁
class IsolateUtils {
  /// 在 Isolate 中执行耗时操作
  /// 
  /// [worker] 在 Isolate 中执行的函数
  /// [argument] 传递给 worker 的参数
  /// [timeout] 可选的超时时间
  /// 
  /// 使用示例：
  /// ```dart
  /// final result = await IsolateUtils.run(
  ///   worker: heavyComputation,
  ///   argument: largeData,
  ///   timeout: Duration(seconds: 30),
  /// );
  /// ```
  static Future<R> run<T, R>({
    required FutureOr<R> Function(T) worker,
    required T argument,
    Duration? timeout,
  }) async {
    // 使用 Flutter 的 compute 函数（已经封装了 Isolate）
    if (timeout == null) {
      return await compute(_isolateEntryPoint<T, R>, _IsolateMessage(
        sendPort: DummySendPort(),
        argument: argument,
        worker: worker,
      ));
    }

    // 带超时的执行
    return await _runWithTimeout(worker, argument, timeout);
  }

  /// 带超时的 Isolate 执行
  static Future<R> _runWithTimeout<T, R>(
    FutureOr<R> Function(T) worker,
    T argument,
    Duration timeout,
  ) async {
    final completer = Completer<R>();
    Timer? timeoutTimer;

    try {
      final result = compute(_isolateEntryPoint<T, R>, _IsolateMessage(
        sendPort: DummySendPort(),
        argument: argument,
        worker: worker,
      ));

      // 设置超时
      timeoutTimer = Timer(timeout, () {
        if (!completer.isCompleted) {
          completer.completeError(
            NetworkError.timeout(duration: timeout),
          );
        }
      });

      result.then((value) {
        timeoutTimer?.cancel();
        if (!completer.isCompleted) {
          completer.complete(value);
        }
      }).catchError((error, stack) {
        timeoutTimer?.cancel();
        if (!completer.isCompleted) {
          completer.completeError(error, stack);
        }
      });

      return await completer.future;
    } catch (e, stack) {
      timeoutTimer?.cancel();
      throw _convertToAppError(e, stack);
    }
  }

  /// Isolate 入口点
  static FutureOr<R> _isolateEntryPoint<T, R>(_IsolateMessage<T, R> message) async {
    try {
      final result = await message.worker(message.argument);
      return result;
    } catch (e, stack) {
      // 在 Isolate 中捕获的错误需要重新抛出
      Error.throwWithStackTrace(e, stack);
    }
  }

  /// 批量处理数据（分片并行）
  /// 
  /// [items] 要处理的数据列表
  /// [processor] 处理函数
  /// [chunkSize] 每批处理的数量
  /// [maxConcurrent] 最大并发数
  static Future<List<R>> batchProcess<T, R>({
    required List<T> items,
    required FutureOr<R> Function(T) processor,
    int chunkSize = 100,
    int maxConcurrent = 4,
  }) async {
    if (items.isEmpty) return [];

    // 数据量小，直接顺序处理
    if (items.length <= chunkSize) {
      final results = <R>[];
      for (final item in items) {
        results.add(await processor(item));
      }
      return results;
    }

    // 分片并行处理
    final chunks = <List<T>>[];
    for (var i = 0; i < items.length; i += chunkSize) {
      chunks.add(items.sublist(
        i,
        i + chunkSize > items.length ? items.length : i + chunkSize,
      ));
    }

    final results = <R>[];
    final chunkFutures = chunks.map((chunk) => run(
      worker: (List<T> c) async {
        final chunkResults = <R>[];
        for (final item in c) {
          chunkResults.add(await processor(item));
        }
        return chunkResults;
      },
      argument: chunk,
    ));

    // 限制并发数
    for (var i = 0; i < chunkFutures.length; i += maxConcurrent) {
      final batch = chunkFutures.skip(i).take(maxConcurrent).toList();
      final batchResults = await Future.wait(batch);
      for (final batchResult in batchResults) {
        results.addAll(batchResult);
      }
    }

    return results;
  }

  /// 转换错误为 AppError
  static AppError _convertToAppError(Object error, StackTrace stack) {
    if (error is AppError) return error;

    // 检查是否是 Isolate 错误
    final errorString = error.toString();
    if (errorString.contains('Isolate')) {
      return FfiError.call(
        method: 'Isolate',
        details: errorString,
        cause: error,
      );
    }

    return UnknownError(
      message: 'Isolate 执行失败',
      technicalDetails: errorString,
      cause: error,
    );
  }
}

/// 虚拟 SendPort（用于 compute 函数）
class DummySendPort implements SendPort {
  @override
  void send(Object? message) {}

  @override
  SendPort get sendPort => this;

  @override
  Future<Capability> pause([Future<void>? resumeSignal]) async => Capability();

  @override
  void resume(Capability resumeCapability) {}

  @override
  bool get isClosed => false;

  @override
  bool get isPaused => false;

  @override
  Future<void> get done async {}

  @override
  void close() {}
}

/// 异步 FFI 调用包装器
/// 
/// 用于将同步 FFI 调用转换为异步 Isolate 调用
class AsyncFfiCall {
  /// 执行 FFI 调用（在 Isolate 中）
  /// 
  /// [call] FFI 调用函数
  /// [timeout] 可选超时
  static Future<R> execute<T, R>({
    required FutureOr<R> Function() call,
    Duration? timeout,
    String operation = 'FFI Call',
  }) async {
    return IsolateUtils.run(
      worker: (_) async => await call(),
      argument: null,
      timeout: timeout,
    );
  }

  /// 执行列表查询（在 Isolate 中）
  static Future<List<T>> queryList<T>({
    required List<T> Function() query,
    Duration? timeout,
    String operation = 'FFI Query',
  }) async {
    return IsolateUtils.run(
      worker: (_) => query(),
      argument: null,
      timeout: timeout,
    );
  }

  /// 执行单个查询（在 Isolate 中）
  static Future<T?> querySingle<T>({
    required T? Function() query,
    Duration? timeout,
    String operation = 'FFI Query',
  }) async {
    return IsolateUtils.run(
      worker: (_) => query(),
      argument: null,
      timeout: timeout,
    );
  }
}

/// 工作器池（高级用法）
/// 
/// 用于需要频繁创建 Isolate 的场景
class IsolateWorkerPool {
  final int _maxWorkers;
  final List<_Worker> _workers = [];
  final List<_PendingTask> _pendingTasks = [];
  int _currentWorkerIndex = 0;
  bool _isDisposed = false;

  IsolateWorkerPool({int maxWorkers = 4}) : _maxWorkers = maxWorkers;

  /// 初始化工作器池
  Future<void> initialize() async {
    for (var i = 0; i < _maxWorkers; i++) {
      _workers.add(await _Worker.create());
    }
  }

  /// 执行任务
  Future<R> execute<T, R>({
    required FutureOr<R> Function(T) task,
    required T argument,
  }) async {
    if (_isDisposed) {
      throw StateError('Worker pool is disposed');
    }

    // 获取下一个可用工作器
    final worker = _getNextWorker();
    
    return worker.execute(task, argument);
  }

  /// 获取下一个工作器（轮询）
  _Worker _getNextWorker() {
    if (_workers.isEmpty) {
      throw StateError('Worker pool not initialized');
    }
    final worker = _workers[_currentWorkerIndex];
    _currentWorkerIndex = (_currentWorkerIndex + 1) % _workers.length;
    return worker;
  }

  /// 释放资源
  void dispose() {
    _isDisposed = true;
    for (final worker in _workers) {
      worker.dispose();
    }
    _workers.clear();
    _pendingTasks.clear();
  }
}

/// 工作器
class _Worker {
  late final Isolate _isolate;
  late final SendPort _sendPort;
  final _responseController = StreamController<_WorkerResponse>.broadcast();

  static Future<_Worker> create() async {
    final worker = _Worker();
    await worker._initialize();
    return worker;
  }

  Future<void> _initialize() async {
    final receivePort = ReceivePort();
    
    _isolate = await Isolate.spawn(
      _workerEntry,
      receivePort.sendPort,
    );

    // 等待工作器准备好
    await for (final message in receivePort) {
      if (message is SendPort) {
        _sendPort = message;
        break;
      }
    }

    // 监听响应
    receivePort.listen((message) {
      if (message is _WorkerResponse) {
        _responseController.add(message);
      }
    });
  }

  Future<R> execute<T, R>(FutureOr<R> Function(T) task, T argument) async {
    final id = DateTime.now().millisecondsSinceEpoch.toString();
    final completer = Completer<R>();

    // 订阅响应
    late final StreamSubscription subscription;
    subscription = _responseController.stream.listen((response) {
      if (response.id == id) {
        subscription.cancel();
        if (response.error != null) {
          completer.completeError(response.error!, response.stackTrace);
        } else {
          completer.complete(response.data as R);
        }
      }
    });

    // 发送任务
    _sendPort.send(_WorkerTask(
      id: id,
      function: task,
      argument: argument,
    ));

    return completer.future;
  }

  void dispose() {
    _isolate.kill();
    _responseController.close();
  }

  static void _workerEntry(SendPort mainSendPort) {
    final receivePort = ReceivePort();
    mainSendPort.send(receivePort.sendPort);

    receivePort.listen((message) async {
      if (message is _WorkerTask) {
        try {
          final result = await message.function(message.argument);
          mainSendPort.send(_WorkerResponse(
            id: message.id,
            data: result,
          ));
        } catch (e, stack) {
          mainSendPort.send(_WorkerResponse(
            id: message.id,
            error: e,
            stackTrace: stack,
          ));
        }
      }
    });
  }
}

/// 工作任务
class _WorkerTask {
  final String id;
  final Function function;
  final dynamic argument;

  _WorkerTask({
    required this.id,
    required this.function,
    required this.argument,
  });
}

/// 工作响应
class _WorkerResponse {
  final String id;
  final dynamic data;
  final Object? error;
  final StackTrace? stackTrace;

  _WorkerResponse({
    required this.id,
    this.data,
    this.error,
    this.stackTrace,
  });
}

/// 待处理任务
class _PendingTask {
  final dynamic argument;
  final Function function;
  final Completer completer;

  _PendingTask({
    required this.argument,
    required this.function,
    required this.completer,
  });
}
