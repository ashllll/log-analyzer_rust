import 'package:flutter/material.dart';
import '../services/error_handler.dart';

/// 错误视图组件
///
/// 显示错误码、消息和解决方案
/// 支持无障碍访问
class ErrorView extends StatelessWidget {
  final AppException exception;
  final VoidCallback? onRetry;
  final VoidCallback? onReport;
  final bool showBackButton;

  const ErrorView({
    super.key,
    required this.exception,
    this.onRetry,
    this.onReport,
    this.showBackButton = false,
  });

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: '错误: ${exception.displayMessage}',
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // 错误图标
              Icon(
                _getIcon(),
                size: 64,
                color: _getColor(),
              ),
              const SizedBox(height: 16),

              // 错误码和消息
              Text(
                exception.displayMessage,
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      color: _getColor(),
                    ),
                textAlign: TextAlign.center,
              ),

              // 解决方案
              if (exception.solution != null) ...[
                const SizedBox(height: 8),
                Text(
                  exception.solution!,
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: Colors.grey[400],
                      ),
                  textAlign: TextAlign.center,
                ),
              ],

              const SizedBox(height: 24),

              // 操作按钮
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  if (onRetry != null)
                    Semantics(
                      button: true,
                      label: '重试',
                      child: ElevatedButton.icon(
                        onPressed: onRetry,
                        icon: const Icon(Icons.refresh),
                        label: const Text('重试'),
                      ),
                    ),
                  if (showBackButton)
                    Semantics(
                      button: true,
                      label: '返回',
                      child: TextButton.icon(
                        onPressed: () => Navigator.of(context).pop(),
                        icon: const Icon(Icons.arrow_back),
                        label: const Text('返回'),
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  IconData _getIcon() {
    if (exception.code == ErrorCodes.ffiLoadFailed ||
        exception.code == ErrorCodes.ffiNotInitialized) {
      return Icons.cloud_off;
    }
    return Icons.error_outline;
  }

  Color _getColor() {
    if (exception.code == ErrorCodes.ffiLoadFailed ||
        exception.code == ErrorCodes.ffiNotInitialized) {
      return Colors.orange[400]!;
    }
    return Colors.red[400]!;
  }
}

/// 错误页面（完整页面）
class ErrorPage extends StatelessWidget {
  final AppException exception;
  final VoidCallback? onRetry;
  final String? title;

  const ErrorPage({
    super.key,
    required this.exception,
    this.onRetry,
    this.title,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: title != null
          ? AppBar(title: Text(title!))
          : null,
      body: ErrorView(
        exception: exception,
        onRetry: onRetry,
        showBackButton: title != null,
      ),
    );
  }
}
