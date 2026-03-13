/// 异步值组件
/// 
/// 统一处理 AsyncValue 的加载、错误、数据状态

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/errors/error_handler.dart';
import 'error_view.dart';

/// 异步值构建器
class AsyncValueWidget<T> extends StatelessWidget {
  final AsyncValue<T> value;
  final Widget Function(T data) data;
  final Widget Function()? loading;
  final Widget Function(Object error, StackTrace? stack)? error;
  final VoidCallback? onRetry;

  const AsyncValueWidget({
    super.key,
    required this.value,
    required this.data,
    this.loading,
    this.error,
    this.onRetry,
  });

  @override
  Widget build(BuildContext context) {
    return value.when(
      data: data,
      loading: loading ?? () => const Center(child: CircularProgressIndicator()),
      error: (e, stack) {
        // 自动报告错误
        ErrorHandler.report(e, stackTrace: stack, context: 'AsyncValueWidget');
        
        if (error != null) {
          return error!(e, stack);
        }
        return ErrorView(
          error: e,
          stackTrace: stack,
          onRetry: onRetry,
        );
      },
    );
  }
}

/// 简化版的异步值构建器
class AsyncValueSliverWidget<T> extends StatelessWidget {
  final AsyncValue<T> value;
  final Widget Function(T data) data;
  final Widget Function()? loading;
  final Widget Function(Object error)? error;

  const AsyncValueSliverWidget({
    super.key,
    required this.value,
    required this.data,
    this.loading,
    this.error,
  });

  @override
  Widget build(BuildContext context) {
    return value.when(
      data: data,
      loading: loading ?? () => const SliverFillRemaining(
        child: Center(child: CircularProgressIndicator()),
      ),
      error: (e, _) => error != null 
        ? error!(e)
        : SliverFillRemaining(
            child: Center(child: Text('Error: $e')),
          ),
    );
  }
}

/// 异步值监听器
/// 
/// 用于处理副作用（如显示 SnackBar、导航等）
class AsyncValueListener<T> extends ConsumerWidget {
  final ProviderListenable<AsyncValue<T>> provider;
  final void Function(BuildContext context, T data)? onData;
  final void Function(BuildContext context, Object error)? onError;
  final Widget child;

  const AsyncValueListener({
    super.key,
    required this.provider,
    this.onData,
    this.onError,
    required this.child,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    ref.listen<AsyncValue<T>>(provider, (previous, next) {
      next.whenOrNull(
        data: (data) => onData?.call(context, data),
        error: (error, _) => onError?.call(context, error),
      );
    });

    return child;
  }
}
