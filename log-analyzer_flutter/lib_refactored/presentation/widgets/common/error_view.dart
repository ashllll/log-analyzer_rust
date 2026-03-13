/// 错误视图
/// 
/// 统一的错误显示组件

import 'package:flutter/material.dart';

import '../../../core/errors/app_error.dart';

/// 错误视图
class ErrorView extends StatelessWidget {
  final Object error;
  final StackTrace? stackTrace;
  final VoidCallback? onRetry;
  final bool showDetails;

  const ErrorView({
    super.key,
    required this.error,
    this.stackTrace,
    this.onRetry,
    this.showDetails = false,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final appError = error is AppError ? error as AppError : null;

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              _getErrorIcon(),
              size: 64,
              color: theme.colorScheme.error,
            ),
            const SizedBox(height: 16),
            Text(
              _getErrorTitle(),
              style: theme.textTheme.titleLarge?.copyWith(
                color: theme.colorScheme.error,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              _getErrorMessage(),
              style: theme.textTheme.bodyMedium,
              textAlign: TextAlign.center,
            ),
            if (showDetails && stackTrace != null) ...[
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: theme.colorScheme.errorContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                constraints: const BoxConstraints(maxHeight: 200),
                child: SingleChildScrollView(
                  child: Text(
                    stackTrace.toString(),
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ),
            ],
            if (onRetry != null) ...[
              const SizedBox(height: 24),
              ElevatedButton.icon(
                onPressed: onRetry,
                icon: const Icon(Icons.refresh),
                label: const Text('重试'),
              ),
            ],
          ],
        ),
      ),
    );
  }

  IconData _getErrorIcon() {
    if (error is NetworkError) return Icons.wifi_off;
    if (error is FfiError) return Icons.memory;
    if (error is NotFoundError) return Icons.search_off;
    if (error is ValidationError) return Icons.warning;
    return Icons.error_outline;
  }

  String _getErrorTitle() {
    return switch (error) {
      AppError e => switch (e.type) {
        ErrorType.network => '网络错误',
        ErrorType.ffi => '系统错误',
        ErrorType.notFound => '未找到',
        ErrorType.validation => '输入错误',
        ErrorType.timeout => '请求超时',
        _ => '发生错误',
      },
      _ => '发生错误',
    };
  }

  String _getErrorMessage() {
    if (error is AppError) {
      return (error as AppError).message;
    }
    return error.toString();
  }
}

/// 内联错误视图
class InlineErrorView extends StatelessWidget {
  final Object error;
  final VoidCallback? onRetry;

  const InlineErrorView({
    super.key,
    required this.error,
    this.onRetry,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final message = error is AppError 
      ? (error as AppError).message 
      : error.toString();

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: theme.colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Icon(
            Icons.error_outline,
            color: theme.colorScheme.error,
            size: 20,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              message,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
          ),
          if (onRetry != null)
            TextButton(
              onPressed: onRetry,
              child: const Text('重试'),
            ),
        ],
      ),
    );
  }
}
