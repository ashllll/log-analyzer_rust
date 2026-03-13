import 'package:fpdart/fpdart.dart';

import '../../core/utils/isolate_utils.dart';

/// FFI 数据源抽象
///
/// 所有 FFI 调用都通过此类进行，确保异步执行和统一的错误处理
/// 这是 Clean Architecture 中的数据源层
abstract class FfiDataSource {
  /// 执行 FFI 查询（返回单个值）
  ///
  /// [query] 查询函数
  /// 返回封装的结果
  Future<FfiResult<T>> query<T>(Future<T> Function() query);

  /// 执行 FFI 查询（返回列表）
  ///
  /// [query] 查询函数
  /// 返回封装的列表结果
  Future<FfiResult<List<T>>> queryList<T>(Future<List<T>> Function() query);

  /// 执行 FFI 命令（无返回值）
  ///
  /// [command] 命令函数
  /// 返回封装的空结果
  Future<FfiResult<void>> command(Future<void> Function() command);

  /// 带超时的查询
  ///
  /// [query] 查询函数
  /// [timeout] 自定义超时时间
  Future<FfiResult<T>> queryWithTimeout<T>(
    Future<T> Function() query,
    Duration timeout,
  );
}

/// FFI 数据源实现
///
/// 使用 AsyncFfiCall 在 Isolate 中执行 FFI 调用
/// 确保 UI 线程不会被阻塞
class FfiDataSourceImpl implements FfiDataSource {
  final Duration _defaultTimeout;

  const FfiDataSourceImpl({
    Duration defaultTimeout = const Duration(seconds: 30),
  }) : _defaultTimeout = defaultTimeout;

  @override
  Future<FfiResult<T>> query<T>(Future<T> Function() query) async {
    return AsyncFfiCall.query(query: query, timeout: _defaultTimeout);
  }

  @override
  Future<FfiResult<List<T>>> queryList<T>(Future<List<T>> Function() query) async {
    return AsyncFfiCall.queryList(query: query, timeout: _defaultTimeout);
  }

  @override
  Future<FfiResult<void>> command(Future<void> Function() command) async {
    return AsyncFfiCall.command(command: command, timeout: _defaultTimeout);
  }

  @override
  Future<FfiResult<T>> queryWithTimeout<T>(
    Future<T> Function() query,
    Duration timeout,
  ) async {
    return AsyncFfiCall.query(query: query, timeout: timeout);
  }
}

/// 使用函数式风格的 FFI 数据源
///
/// 返回 fpdart 的 TaskEither 类型，便于函数式组合
class FfiDataSourceFunctional implements FfiDataSource {
  final Duration _defaultTimeout;

  const FfiDataSourceFunctional({
    Duration defaultTimeout = const Duration(seconds: 30),
  }) : _defaultTimeout = defaultTimeout;

  @override
  Future<FfiResult<T>> query<T>(Future<T> Function() query) async {
    final task = AsyncFfiCallFunctional.query(query: query);
    final result = await task.run();
    return result.fold(
      (error) => FfiResult.error(error),
      (data) => FfiResult.success(data),
    );
  }

  @override
  Future<FfiResult<List<T>>> queryList<T>(Future<List<T>> Function() query) async {
    final task = AsyncFfiCallFunctional.queryList(query: query);
    final result = await task.run();
    return result.fold(
      (error) => FfiResult.error(error),
      (data) => FfiResult.success(data),
    );
  }

  @override
  Future<FfiResult<void>> command(Future<void> Function() command) async {
    final task = AsyncFfiCallFunctional.command(command: command);
    final result = await task.run();
    return result.fold(
      (error) => FfiResult.error(error),
      (_) => FfiResult.success(null as dynamic),
    );
  }

  @override
  Future<FfiResult<T>> queryWithTimeout<T>(
    Future<T> Function() query,
    Duration timeout,
  ) async {
    final task = AsyncFfiCallFunctional.query(query: query, timeout: timeout);
    final result = await task.run();
    return result.fold(
      (error) => FfiResult.error(error),
      (data) => FfiResult.success(data),
    );
  }

  /// 获取函数式查询任务（不立即执行）
  ///
  /// 返回 TaskEither，可以与其他函数式操作组合
  TaskEither<String, T> queryTask<T>(Future<T> Function() query) {
    return AsyncFfiCallFunctional.query(query: query, timeout: _defaultTimeout);
  }

  /// 获取函数式查询列表任务（不立即执行）
  TaskEither<String, List<T>> queryListTask<T>(Future<List<T>> Function() query) {
    return AsyncFfiCallFunctional.queryList(query: query, timeout: _defaultTimeout);
  }

  /// 获取函数式命令任务（不立即执行）
  TaskEither<String, void> commandTask(Future<void> Function() command) {
    return AsyncFfiCallFunctional.command(command: command, timeout: _defaultTimeout);
  }
}

/// FFI 数据源工厂
///
/// 提供创建不同实现的工厂方法
class FfiDataSourceFactory {
  /// 创建标准实现
  static FfiDataSource create({Duration? timeout}) {
    return FfiDataSourceImpl(defaultTimeout: timeout ?? const Duration(seconds: 30));
  }

  /// 创建函数式实现
  static FfiDataSourceFunctional createFunctional({Duration? timeout}) {
    return FfiDataSourceFunctional(
      defaultTimeout: timeout ?? const Duration(seconds: 30),
    );
  }
}
