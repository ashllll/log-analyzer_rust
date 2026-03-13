/// 应用错误类型
///
/// 使用函数式错误处理模式（参考 fpdart）
/// 所有错误都是可序列化的，便于日志记录和错误追踪
import 'package:equatable/equatable.dart';
import 'package:fpdart/fpdart.dart';

/// 错误类型枚举
enum ErrorType {
  /// 网络/连接错误
  network,
  
  /// FFI 通信错误
  ffi,
  
  /// 验证错误
  validation,
  
  /// 未找到
  notFound,
  
  /// 权限错误
  unauthorized,
  
  /// 超时
  timeout,
  
  /// 取消
  cancelled,
  
  /// 未知错误
  unknown,
  
  /// 输入/输出错误
  io,
  
  /// 解析错误
  parse,
}

/// 应用错误基类
///
/// 实现了 Equatable 便于比较
/// 支持错误链（cause）用于调试
abstract class AppError extends Equatable {
  /// 错误类型
  final ErrorType type;
  
  /// 错误消息（用户友好）
  final String message;
  
  /// 技术详情（用于调试）
  final String? technicalDetails;
  
  /// 原始错误
  final Object? cause;
  
  /// 错误码（可选）
  final String? code;
  
  /// 额外上下文数据
  final Map<String, dynamic>? context;

  const AppError({
    required this.type,
    required this.message,
    this.technicalDetails,
    this.cause,
    this.code,
    this.context,
  });

  @override
  List<Object?> get props => [type, message, technicalDetails, code, context];

  @override
  String toString() {
    final buffer = StringBuffer()
      ..write('AppError[$type]: $message');
    if (technicalDetails != null) {
      buffer.write(' | Details: $technicalDetails');
    }
    if (cause != null) {
      buffer.write(' | Cause: $cause');
    }
    return buffer.toString();
  }
}

/// FFI 错误
class FfiError extends AppError {
  const FfiError({
    required super.message,
    super.technicalDetails,
    super.cause,
    super.code,
    super.context,
  }) : super(type: ErrorType.ffi);

  /// 工厂方法：初始化错误
  factory FfiError.initialization({
    String? details,
    Object? cause,
  }) => FfiError(
    message: 'FFI 初始化失败',
    technicalDetails: details,
    cause: cause,
    code: 'FFI_INIT_ERROR',
  );

  /// 工厂方法：调用错误
  factory FfiError.call({
    required String method,
    String? details,
    Object? cause,
  }) => FfiError(
    message: 'FFI 调用失败: $method',
    technicalDetails: details,
    cause: cause,
    code: 'FFI_CALL_ERROR',
    context: {'method': method},
  );
}

/// 网络错误
class NetworkError extends AppError {
  const NetworkError({
    required super.message,
    super.technicalDetails,
    super.cause,
    super.code,
    super.context,
  }) : super(type: ErrorType.network);

  /// 工厂方法：连接超时
  factory NetworkError.timeout({
    required Duration duration,
    Object? cause,
  }) => NetworkError(
    message: '连接超时，请检查网络',
    technicalDetails: 'Timeout after ${duration.inSeconds}s',
    cause: cause,
    code: 'NETWORK_TIMEOUT',
    context: {'timeout': duration.inMilliseconds},
  );
}

/// 验证错误
class ValidationError extends AppError {
  final Map<String, String> fieldErrors;

  const ValidationError({
    required super.message,
    this.fieldErrors = const {},
    super.technicalDetails,
    super.cause,
    super.code,
    super.context,
  }) : super(type: ErrorType.validation);

  @override
  List<Object?> get props => [...super.props, fieldErrors];

  /// 工厂方法：字段验证错误
  factory ValidationError.field({
    required String field,
    required String error,
  }) => ValidationError(
    message: '验证失败',
    fieldErrors: {field: error},
    code: 'VALIDATION_ERROR',
  );
}

/// 未找到错误
class NotFoundError extends AppError {
  const NotFoundError({
    required super.message,
    required String resource,
    String? id,
    super.technicalDetails,
    super.cause,
    super.code,
  }) : super(
    type: ErrorType.notFound,
    context: {'resource': resource, if (id != null) 'id': id},
  );
}

/// IO 错误
class IOError extends AppError {
  const IOError({
    required super.message,
    required String path,
    super.technicalDetails,
    super.cause,
    super.code,
  }) : super(
    type: ErrorType.io,
    context: {'path': path},
  );
}

/// 解析错误
class ParseError extends AppError {
  const ParseError({
    required super.message,
    required dynamic data,
    super.technicalDetails,
    super.cause,
    super.code,
  }) : super(
    type: ErrorType.parse,
    context: {'data': data.toString()},
  );
}

/// 未知错误
class UnknownError extends AppError {
  const UnknownError({
    required super.message,
    super.technicalDetails,
    super.cause,
    super.code,
    super.context,
  }) : super(type: ErrorType.unknown);
}

/// 取消错误
class CancelledError extends AppError {
  const CancelledError({
    super.message = '操作已取消',
    super.technicalDetails,
    super.cause,
  }) : super(type: ErrorType.cancelled, code: 'CANCELLED');
}

// ==================== 类型别名 ====================

/// 结果类型 - Either<Error, Success>
/// 
/// 使用方式：
/// ```dart
/// TaskEither<AppError, List<Workspace>> getWorkspaces();
/// ```
typedef AppEither<T> = Either<AppError, T>;

/// 异步结果类型
typedef AppTask<T> = TaskEither<AppError, T>;

/// IO 结果类型（同步）
typedef AppIO<T> = IOEither<AppError, T>;

// ==================== 扩展方法 ====================

extension AppErrorExtensions on AppError {
  /// 转换为用户友好消息
  String toUserMessage() {
    return message;
  }

  /// 是否应该上报到 Sentry
  bool get shouldReportToSentry {
    return type != ErrorType.validation && 
           type != ErrorType.cancelled &&
           type != ErrorType.notFound;
  }
}

extension EitherExtensions<L extends AppError, R> on Either<L, R> {
  /// 获取成功值，失败时返回 null
  R? getOrNull() => fold((_) => null, (r) => r);
  
  /// 获取错误，成功时返回 null
  L? getErrorOrNull() => fold((l) => l, (_) => null);
  
  /// 是否成功
  bool get isSuccess => isRight();
  
  /// 是否失败
  bool get isFailure => isLeft();
}

extension TaskEitherExtensions<L extends AppError, R> on TaskEither<L, R> {
  /// 执行并返回 Either
  Future<Either<L, R>> runSafe() async {
    try {
      return await run();
    } catch (e, stack) {
      return left(UnknownError(
        message: '未预期的错误',
        technicalDetails: e.toString(),
        cause: e,
      ) as L);
    }
  }
}
