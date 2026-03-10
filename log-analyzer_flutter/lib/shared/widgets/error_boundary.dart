// lib/shared/widgets/error_boundary.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// 错误边界组件
///
/// 捕获子组件的异常并显示友好的错误界面
/// 类似 React 的 ErrorBoundary
class ErrorBoundary extends ConsumerStatefulWidget {
  /// 子组件
  final Widget child;

  /// 自定义错误回调
  final void Function(Object error, StackTrace stackTrace)? onError;

  /// 自定义错误界面构建器
  final Widget Function(
    BuildContext context,
    Object error,
    VoidCallback? retry,
  )?
  errorBuilder;

  const ErrorBoundary({
    super.key,
    required this.child,
    this.onError,
    this.errorBuilder,
  });

  @override
  ConsumerState<ErrorBoundary> createState() => _ErrorBoundaryState();
}

class _ErrorBoundaryState extends ConsumerState<ErrorBoundary> {
  Object? _error;
  StackTrace? _stackTrace;

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      // 已捕获错误，显示错误界面
      if (widget.errorBuilder != null) {
        return widget.errorBuilder!(context, _error!, _resetError);
      }
      return _buildDefaultErrorView(context);
    }

    return widget.child;
  }

  /// 重置错误状态
  void _resetError() {
    setState(() {
      _error = null;
      _stackTrace = null;
    });
  }

  /// 构建默认错误界面
  Widget _buildDefaultErrorView(BuildContext context) {
    final theme = Theme.of(context);
    final errorMessage = _error?.toString() ?? '发生未知错误';

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24.0),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            // 错误图标
            Icon(Icons.error_outline, size: 64, color: theme.colorScheme.error),
            const SizedBox(height: 16),

            // 错误标题
            Text(
              '出现错误',
              style: theme.textTheme.titleLarge?.copyWith(
                color: theme.colorScheme.error,
              ),
            ),
            const SizedBox(height: 8),

            // 错误消息
            Text(
              errorMessage,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 24),

            // 操作按钮
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                // 重试按钮
                ElevatedButton.icon(
                  onPressed: _resetError,
                  icon: const Icon(Icons.refresh),
                  label: const Text('重试'),
                ),
                const SizedBox(width: 12),
                // 报告按钮
                OutlinedButton.icon(
                  onPressed: () => _reportError(context),
                  icon: const Icon(Icons.bug_report),
                  label: const Text('报告问题'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  /// 报告错误
  void _reportError(BuildContext context) {
    if (_error == null) return;

    // 显示错误详情对话框
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('错误详情'),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                '错误类型: ${_error.runtimeType}',
                style: const TextStyle(fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 8),
              Text('错误信息: $_error'),
              if (_stackTrace != null) ...[
                const SizedBox(height: 16),
                const Text(
                  '堆栈跟踪:',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 8),
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: Theme.of(
                      context,
                    ).colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    _stackTrace.toString(),
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 11,
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('关闭'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(context);
              // TODO: 实现错误上报
              ScaffoldMessenger.of(
                context,
              ).showSnackBar(const SnackBar(content: Text('感谢您的反馈')));
            },
            child: const Text('上报'),
          ),
        ],
      ),
    );
  }

  /// 捕获异常
  void catchError(Object error, StackTrace stackTrace) {
    // 调用自定义错误回调
    widget.onError?.call(error, stackTrace);

    setState(() {
      _error = error;
      _stackTrace = stackTrace;
    });
  }
}

/// 异步错误边界
///
/// 用于捕获异步操作中的错误
class AsyncErrorBoundary extends ConsumerWidget {
  /// 异步值
  final AsyncValue<void> asyncValue;

  /// 子组件构建器
  final Widget Function(BuildContext context) builder;

  /// 错误界面构建器
  final Widget Function(BuildContext context, Object error, VoidCallback retry)?
  errorBuilder;

  /// 加载中界面构建器
  final Widget Function(BuildContext context)? loadingBuilder;

  const AsyncErrorBoundary({
    super.key,
    required this.asyncValue,
    required this.builder,
    this.errorBuilder,
    this.loadingBuilder,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return asyncValue.when(
      data: (_) => builder(context),
      loading: () =>
          loadingBuilder?.call(context) ?? _buildDefaultLoading(context),
      error: (error, stack) =>
          errorBuilder?.call(context, error, () {
            // 触发重试 - 通过 key 强制重建
          }) ??
          _buildDefaultError(context, error),
    );
  }

  Widget _buildDefaultLoading(BuildContext context) {
    return const Center(child: CircularProgressIndicator());
  }

  Widget _buildDefaultError(BuildContext context, Object error) {
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24.0),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.error_outline, size: 64, color: theme.colorScheme.error),
            const SizedBox(height: 16),
            Text(
              error.toString(),
              style: theme.textTheme.bodyMedium,
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}
