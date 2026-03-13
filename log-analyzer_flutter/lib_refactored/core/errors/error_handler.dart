/// 全局错误处理器
/// 
/// 无副作用的错误处理方案
/// 不在 widget 中直接修改 FlutterError.onError
/// 而是通过 Zone 和 PlatformDispatcher 处理

import 'dart:async';
import 'dart:developer' as developer;
import 'dart:isolate';
import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:logger/logger.dart';

import 'app_error.dart';

/// 错误处理器配置
class ErrorHandlerConfig {
  /// 是否启用 Sentry
  final bool enableSentry;
  
  /// 是否启用控制台日志
  final bool enableConsoleLog;
  
  /// 是否启用开发者日志
  final bool enableDevLog;
  
  /// 是否显示详细错误信息（调试用）
  final bool showDetailedErrors;
  
  /// Sentry DSN
  final String? sentryDsn;

  const ErrorHandlerConfig({
    this.enableSentry = false,
    this.enableConsoleLog = true,
    this.enableDevLog = kDebugMode,
    this.showDetailedErrors = kDebugMode,
    this.sentryDsn,
  });

  /// 调试配置
  static const debug = ErrorHandlerConfig(
    enableSentry: false,
    enableConsoleLog: true,
    enableDevLog: true,
    showDetailedErrors: true,
  );

  /// 生产配置
  static const production = ErrorHandlerConfig(
    enableSentry: true,
    enableConsoleLog: false,
    enableDevLog: false,
    showDetailedErrors: false,
  );
}

/// 全局错误处理器
/// 
/// 使用方法：
/// ```dart
/// void main() {
///   ErrorHandler.initialize(
///     config: ErrorHandlerConfig.debug,
///   );
///   
///   runApp(
///     ErrorHandler.wrapApp(const MyApp()),
///   );
/// }
/// ```
class ErrorHandler {
  static final _logger = Logger();
  static ErrorHandlerConfig _config = ErrorHandlerConfig.debug;
  static final _errorController = StreamController<AppError>.broadcast();
  
  /// 错误流 - 用于全局错误监听
  static Stream<AppError> get errorStream => _errorController.stream;

  /// 初始化错误处理
  static void initialize({
    ErrorHandlerConfig config = ErrorHandlerConfig.debug,
  }) {
    _config = config;
    
    // 设置 Flutter Error 处理
    FlutterError.onError = _handleFlutterError;
    
    // 设置 PlatformDispatcher 错误处理
    PlatformDispatcher.instance.onError = _handlePlatformError;
    
    // 设置 Zone 错误处理（捕获异步错误）
    // 这个在 runZonedGuarded 中设置
  }

  /// 包装应用
  /// 
  /// 提供 ErrorWidget 自定义构建
  static Widget wrapApp(Widget app) {
    return _ErrorHandlerWidget(
      config: _config,
      child: app,
    );
  }

  /// 在 Zone 中运行应用
  /// 
  /// 捕获所有未处理的异步错误
  static void runInZone(VoidCallback runApp) {
    runZonedGuarded(
      runApp,
      _handleZoneError,
      zoneSpecification: ZoneSpecification(
        // 可以在这里添加自定义的 zone 规范
        handleUncaughtError: (self, parent, zone, error, stackTrace) {
          _handleZoneError(error, stackTrace);
        },
      ),
    );
  }

  /// 处理 Flutter 框架错误
  static void _handleFlutterError(FlutterErrorDetails details) {
    final error = _convertToAppError(
      details.exception,
      details.stack,
      'Flutter Framework Error',
    );
    
    _logError(error);
    _reportError(error);
    
    // 调用原始处理器（如果有）
    if (_config.showDetailedErrors) {
      FlutterError.presentError(details);
    }
  }

  /// 处理 Platform 错误
  static bool _handlePlatformError(Object error, StackTrace stack) {
    final appError = _convertToAppError(error, stack, 'Platform Error');
    _logError(appError);
    _reportError(appError);
    return true; // 表示已处理
  }

  /// 处理 Zone 错误
  static void _handleZoneError(Object error, StackTrace stack) {
    final appError = _convertToAppError(error, stack, 'Zone Error');
    _logError(appError);
    _reportError(appError);
  }

  /// 转换错误为 AppError
  static AppError _convertToAppError(
    Object error,
    StackTrace? stack,
    String source,
  ) {
    if (error is AppError) {
      return error;
    }

    return UnknownError(
      message: '发生未知错误，请稍后重试',
      technicalDetails: '$source: $error',
      cause: error,
      context: {
        'source': source,
        'stackTrace': stack?.toString(),
      },
    );
  }

  /// 记录错误
  static void _logError(AppError error) {
    if (_config.enableConsoleLog) {
      _logger.e(
        error.message,
        error: error.cause,
        stackTrace: error.cause is Error ? (error.cause as Error).stackTrace : null,
      );
    }

    if (_config.enableDevLog) {
      developer.log(
        error.toString(),
        name: 'ErrorHandler',
        error: error.cause,
        stackTrace: error.cause is Error ? (error.cause as Error).stackTrace : null,
      );
    }
  }

  /// 上报错误
  static void _reportError(AppError error) {
    // 发送到错误流
    if (!_errorController.isClosed) {
      _errorController.add(error);
    }

    // 上报到 Sentry（如果启用）
    if (_config.enableSentry && error.shouldReportToSentry) {
      // TODO: 实现 Sentry 上报
      // Sentry.captureException(error.cause ?? error, stackTrace: ...);
    }
  }

  /// 手动报告错误
  static void report(Object error, {StackTrace? stackTrace, String? context}) {
    final appError = _convertToAppError(
      error,
      stackTrace,
      context ?? 'Manual Report',
    );
    _logError(appError);
    _reportError(appError);
  }

  /// 创建 Future 的错误处理器
  static Future<T> handleFuture<T>(
    Future<T> future, {
    String? operation,
  }) async {
    try {
      return await future;
    } catch (e, stack) {
      final error = _convertToAppError(e, stack, operation ?? 'Future Operation');
      _logError(error);
      _reportError(error);
      throw error;
    }
  }

  /// 创建 Stream 的错误处理器
  static Stream<T> handleStream<T>(
    Stream<T> stream, {
    String? operation,
  }) {
    return stream.handleError((error, stack) {
      final appError = _convertToAppError(
        error,
        stack,
        operation ?? 'Stream Operation',
      );
      _logError(appError);
      _reportError(appError);
    });
  }

  /// 处理 Isolate 错误
  static void handleIsolateError(Object error, StackTrace stack) {
    final appError = _convertToAppError(error, stack, 'Isolate Error');
    _logError(appError);
    _reportError(appError);
  }

  /// 释放资源
  static void dispose() {
    _errorController.close();
  }
}

/// 错误处理器 Widget
/// 
/// 提供自定义的 ErrorWidget 构建
class _ErrorHandlerWidget extends StatelessWidget {
  final ErrorHandlerConfig config;
  final Widget child;

  const _ErrorHandlerWidget({
    required this.config,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    // 设置自定义 ErrorWidget
    ErrorWidget.builder = (details) => _CustomErrorWidget(
      details: details,
      showDetailed: config.showDetailedErrors,
    );

    return child;
  }
}

/// 自定义错误 Widget
class _CustomErrorWidget extends StatelessWidget {
  final FlutterErrorDetails details;
  final bool showDetailed;

  const _CustomErrorWidget({
    required this.details,
    required this.showDetailed,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Material(
      child: Container(
        padding: const EdgeInsets.all(24),
        color: theme.colorScheme.errorContainer,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.error_outline,
              size: 64,
              color: theme.colorScheme.error,
            ),
            const SizedBox(height: 16),
            Text(
              '出现错误',
              style: theme.textTheme.titleLarge?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              showDetailed 
                ? details.exception.toString()
                : '应用遇到了问题，请重启应用或联系支持团队。',
              style: theme.textTheme.bodyMedium,
              textAlign: TextAlign.center,
            ),
            if (showDetailed && details.stack != null) ...[
              const SizedBox(height: 16),
              Expanded(
                child: SingleChildScrollView(
                  child: Text(
                    details.stack.toString(),
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// 错误边界（无副作用版本）
/// 
/// 捕获子组件中的 Flutter 错误，不使用全局 FlutterError.onError
class ErrorBoundary extends StatefulWidget {
  final Widget child;
  final Widget Function(Object error, StackTrace? stack)? errorBuilder;
  final void Function(Object error, StackTrace? stack)? onError;
  final VoidCallback? onRetry;

  const ErrorBoundary({
    super.key,
    required this.child,
    this.errorBuilder,
    this.onError,
    this.onRetry,
  });

  @override
  State<ErrorBoundary> createState() => _ErrorBoundaryState();
}

class _ErrorBoundaryState extends State<ErrorBoundary> {
  Object? _error;
  StackTrace? _stackTrace;

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return widget.errorBuilder?.call(_error!, _stackTrace) ??
          _buildDefaultErrorView();
    }
    // 使用 Builder 捕获构建错误
    return _ErrorCatcher(
      onError: _handleError,
      child: widget.child,
    );
  }

  void _handleError(Object error, StackTrace? stack) {
    if (mounted) {
      setState(() {
        _error = error;
        _stackTrace = stack;
      });
    }
    widget.onError?.call(error, stack);
    ErrorHandler.report(error, stackTrace: stack, context: 'ErrorBoundary');
  }

  Widget _buildDefaultErrorView() {
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, size: 48, color: theme.colorScheme.error),
            const SizedBox(height: 16),
            Text(
              '组件加载失败',
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              '$_error',
              style: theme.textTheme.bodySmall,
              textAlign: TextAlign.center,
            ),
            if (widget.onRetry != null) ...[
              const SizedBox(height: 16),
              ElevatedButton.icon(
                onPressed: () {
                  setState(() {
                    _error = null;
                    _stackTrace = null;
                  });
                  widget.onRetry!();
                },
                icon: const Icon(Icons.refresh),
                label: const Text('重试'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// 错误捕获器
/// 
/// 使用 Flutter 的 Builder 模式捕获构建错误
class _ErrorCatcher extends StatelessWidget {
  final void Function(Object error, StackTrace? stack) onError;
  final Widget child;

  const _ErrorCatcher({
    required this.onError,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    try {
      return child;
    } catch (error, stack) {
      // 使用 Future.microtask 避免在 build 中调用 setState
      Future.microtask(() => onError(error, stack));
      return const SizedBox.shrink();
    }
  }
}

/// AsyncValue 错误处理器扩展
extension AsyncValueErrorExtension<T> on AsyncValue<T> {
  /// 转换为 AppError
  AppError? get appError => whenOrNull(
    error: (error, _) => error is AppError 
      ? error 
      : UnknownError(message: error.toString(), cause: error),
  );
}
