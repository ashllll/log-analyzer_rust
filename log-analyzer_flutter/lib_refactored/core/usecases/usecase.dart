/// UseCase 抽象类
/// 
/// Clean Architecture 中的用例层
/// 每个用例都是独立的业务逻辑单元
/// 
/// 使用函数式错误处理返回 Either<Error, Result>

import 'package:fpdart/fpdart.dart';
import '../errors/app_error.dart';

/// 无参数的 UseCase
/// 
/// 使用方法：
/// ```dart
/// class GetWorkspacesUseCase extends NoParamsUseCase<List<Workspace>> {
///   @override
///   AppTask<List<Workspace>> call() {
///     return repository.getWorkspaces();
///   }
/// }
/// ```
abstract class NoParamsUseCase<Type> {
  /// 执行用例
  /// 
  /// 返回 TaskEither，不会立即执行
  AppTask<Type> call();
  
  /// 直接执行并返回结果
  Future<AppEither<Type>> execute() => call().runSafe();
}

/// 带参数的 UseCase
/// 
/// 使用方法：
/// ```dart
/// class SearchLogsUseCase extends UseCase<List<LogEntry>, SearchParams> {
///   @override
///   AppTask<List<LogEntry>> call(SearchParams params) {
///     return repository.search(params);
///   }
/// }
/// ```
abstract class UseCase<Type, Params> {
  /// 执行用例
  /// 
  /// 返回 TaskEither，不会立即执行
  AppTask<Type> call(Params params);
  
  /// 直接执行并返回结果
  Future<AppEither<Type>> execute(Params params) => call(params).runSafe();
}

/// 流式 UseCase
/// 
/// 用于需要持续返回数据的场景（如实时更新）
abstract class StreamUseCase<Type, Params> {
  /// 执行用例
  /// 
  /// 返回 Stream，可以监听多个结果
  Stream<Type> call(Params params);
}

/// 分页参数基类
class PaginationParams {
  final int page;
  final int pageSize;
  final String? cursor;

  const PaginationParams({
    this.page = 1,
    this.pageSize = 20,
    this.cursor,
  });

  const PaginationParams.first({this.pageSize = 20})
      : page = 1,
        cursor = null;

  PaginationParams next() => PaginationParams(
    page: page + 1,
    pageSize: pageSize,
    cursor: cursor,
  );

  int get offset => (page - 1) * pageSize;
}

/// 分页结果
class PaginatedResult<T> {
  final List<T> items;
  final int total;
  final int page;
  final int pageSize;
  final String? nextCursor;
  final bool hasMore;

  const PaginatedResult({
    required this.items,
    required this.total,
    required this.page,
    required this.pageSize,
    this.nextCursor,
    required this.hasMore,
  });

  factory PaginatedResult.empty() => const PaginatedResult(
    items: [],
    total: 0,
    page: 1,
    pageSize: 20,
    hasMore: false,
  );

  /// 当前是否为空
  bool get isEmpty => items.isEmpty;

  /// 当前是否有数据
  bool get isNotEmpty => items.isNotEmpty;

  /// 总页数
  int get totalPages => (total / pageSize).ceil();

  /// 是否第一页
  bool get isFirstPage => page == 1;

  /// 复制并添加新项目
  PaginatedResult<T> addItems(List<T> newItems, {bool hasMore = false}) {
    return PaginatedResult(
      items: [...items, ...newItems],
      total: total + newItems.length,
      page: page,
      pageSize: pageSize,
      nextCursor: nextCursor,
      hasMore: hasMore,
    );
  }
}

/// 用例结果包装器
/// 
/// 用于需要在结果中包含额外元数据的场景
class UseCaseResult<T> {
  final T data;
  final Map<String, dynamic> metadata;
  final DateTime executedAt;

  const UseCaseResult({
    required this.data,
    this.metadata = const {},
    required this.executedAt,
  });

  factory UseCaseResult.now(T data, {Map<String, dynamic> metadata = const {}}) {
    return UseCaseResult(
      data: data,
      metadata: metadata,
      executedAt: DateTime.now(),
    );
  }
}

/// 用例组合扩展
extension UseCaseComposition<T> on AppTask<T> {
  /// 映射结果
  AppTask<R> mapResult<R>(R Function(T) mapper) {
    return map(mapper);
  }

  /// 链式调用另一个用例
  AppTask<R> flatMapUseCase<R>(AppTask<R> Function(T) next) {
    return flatMap(next);
  }

  /// 添加副作用（如日志记录）
  AppTask<T> tap(void Function(T) action) {
    return map((t) {
      action(t);
      return t;
    });
  }

  /// 错误时执行副作用
  AppTask<T> tapError(void Function(AppError) action) {
    return orElse((error, _) {
      action(error);
      return left(error);
    });
  }
}

/// 用例执行日志
class UseCaseLogger {
  static void logStart(String useCaseName, [dynamic params]) {
    // 可以使用 logger 包
    // logger.d('▶️ $useCaseName started', params);
  }

  static void logSuccess(String useCaseName, [dynamic result]) {
    // logger.d('✅ $useCaseName completed', result);
  }

  static void logError(String useCaseName, AppError error) {
    // logger.e('❌ $useCaseName failed', error);
  }
}
