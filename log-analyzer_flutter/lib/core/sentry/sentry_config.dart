/// Sentry 错误追踪配置
///
/// 提供统一的 Sentry 初始化和错误捕获机制
/// 仅在生产环境启用，DSN 从环境变量读取
library;

import 'package:flutter/foundation.dart';
import 'package:sentry_flutter/sentry_flutter.dart';

/// Sentry 配置常量
class SentryConfig {
  SentryConfig._();

  /// 环境变量名称 - Sentry DSN
  static const String dsnEnvKey = 'SENTRY_DSN';

  /// 默认采样率 - 100% 捕获错误
  static const double defaultTracesSampleRate = 1.0;

  /// 默认 profile 采样率 - 仅在生产环境启用
  static const double defaultProfilesSampleRate = 0.2;

  /// 环境名称
  static const String environmentProd = 'production';
  static const String environmentDev = 'development';

  /// 判断是否应启用 Sentry
  ///
  /// 仅在 Release 模式且 DSN 已配置时启用
  static bool get shouldEnable {
    // 仅在 Release 模式启用
    if (kDebugMode) {
      return false;
    }
    return dsn.isNotEmpty;
  }

  /// 获取 Sentry DSN
  ///
  /// 从环境变量读取，如果未配置则返回空字符串
  static String get dsn {
    // 从编译时环境变量读取
    // 使用方式: flutter build --dart-define=SENTRY_DSN=your_dsn
    return const String.fromEnvironment(
      'SENTRY_DSN',
      defaultValue: '',
    );
  }

  /// 获取当前环境名称
  static String get environment {
    return kReleaseMode ? environmentProd : environmentDev;
  }
}

/// Sentry 初始化器
///
/// 负责配置和初始化 Sentry SDK
class SentryInitializer {
  SentryInitializer._();

  /// 初始化 Sentry
  ///
  /// 应在 main() 中调用，在 runApp() 之前
  /// 如果不应启用，则直接执行 runApp
  static Future<void> initialize({
    required void Function() runApp,
  }) async {
    // 检查是否应启用 Sentry
    if (!SentryConfig.shouldEnable) {
      debugPrint('[Sentry] 未启用: ${kDebugMode ? "调试模式" : "DSN 未配置"}');
      runApp();
      return;
    }

    debugPrint('[Sentry] 正在初始化...');

    await SentryFlutter.init(
      (options) {
        // 基础配置
        options.dsn = SentryConfig.dsn;
        options.environment = SentryConfig.environment;

        // 采样率配置
        options.tracesSampleRate = SentryConfig.defaultTracesSampleRate;
        options.profilesSampleRate = SentryConfig.defaultProfilesSampleRate;

        // 附加调试信息（仅在调试模式）
        options.debug = kDebugMode;

        // 设置发布版本
        options.release = 'log-analyzer@1.0.0';

        // 设置会话追踪间隔
        options.autoSessionTrackingInterval = const Duration(minutes: 5);

        // 过滤敏感信息
        options.beforeSend = _beforeSendCallback;

        // 过滤面包屑中的敏感信息
        options.beforeBreadcrumb = _beforeBreadcrumbCallback;
      },
      appRunner: runApp,
    );
  }

  /// 发送前回调 - 过滤敏感信息
  static SentryEvent? _beforeSendCallback(SentryEvent event, Hint hint) {
    // 过滤可能包含敏感信息的字段
    // 例如：过滤请求头中的 Authorization
    final headers = event.request?.headers;
    if (headers != null && headers.isNotEmpty) {
      final filteredHeaders = Map<String, String>.from(headers);
      filteredHeaders.remove('authorization');
      filteredHeaders.remove('cookie');
      filteredHeaders.remove('set-cookie');

      return event.copyWith(
        request: event.request?.copyWith(headers: filteredHeaders),
      );
    }

    return event;
  }

  /// 面包屑过滤回调
  ///
  /// 注意: Sentry 8.x 的签名是 (Breadcrumb?, Hint) -> Breadcrumb?
  static Breadcrumb? _beforeBreadcrumbCallback(Breadcrumb? breadcrumb, Hint hint) {
    // 如果没有面包屑，直接返回
    if (breadcrumb == null) {
      return null;
    }

    // 过滤包含敏感关键词的面包屑
    final message = breadcrumb.message?.toLowerCase() ?? '';
    const sensitiveKeywords = ['password', 'token', 'secret', 'key', 'credential'];

    for (final keyword in sensitiveKeywords) {
      if (message.contains(keyword)) {
        return null; // 丢弃此面包屑
      }
    }

    return breadcrumb;
  }
}

/// Sentry 工具类
///
/// 提供便捷的错误捕获和上下文设置方法
class SentryUtils {
  SentryUtils._();

  /// 捕获异常
  ///
  /// 用于在 try-catch 中手动捕获异常
  static Future<SentryId> captureException(
    dynamic exception, {
    StackTrace? stackTrace,
    dynamic hint,
  }) async {
    return Sentry.captureException(
      exception,
      stackTrace: stackTrace,
      hint: hint,
    );
  }

  /// 捕获消息
  ///
  /// 用于记录非异常级别的错误或警告
  static Future<SentryId> captureMessage(
    String message, {
    SentryLevel level = SentryLevel.info,
  }) async {
    return Sentry.captureMessage(message, level: level);
  }

  /// 设置用户信息
  ///
  /// 用于关联错误与特定用户
  static Future<void> setUser({
    String? id,
    String? email,
    String? username,
    String? ipAddress,
    Map<String, dynamic>? data,
  }) async {
    if (id == null && email == null && username == null) {
      // 清除用户信息
      Sentry.configureScope((scope) {
        scope.setUser(null);
      });
      return;
    }

    Sentry.configureScope((scope) {
      scope.setUser(SentryUser(
        id: id,
        email: email,
        username: username,
        ipAddress: ipAddress,
        data: data,
      ));
    });
  }

  /// 添加面包屑（用户操作记录）
  ///
  /// 用于记录导致错误发生的用户操作序列
  static void addBreadcrumb({
    required String message,
    String? category,
    String? type,
    Map<String, dynamic>? data,
    SentryLevel? level,
  }) {
    Sentry.addBreadcrumb(Breadcrumb(
      message: message,
      category: category,
      type: type,
      data: data,
      level: level,
    ));
  }

  /// 设置上下文标签
  ///
  /// 用于添加额外的上下文信息
  static Future<void> setTag(String key, String value) async {
    Sentry.configureScope((scope) {
      scope.setTag(key, value);
    });
  }

  /// 设置上下文数据
  ///
  /// 用于添加结构化的额外信息
  static Future<void> setContext(
    String key,
    Map<String, dynamic> value,
  ) async {
    Sentry.configureScope((scope) {
      scope.setContexts(key, value);
    });
  }

  /// 清除所有上下文
  ///
  /// 用于用户登出时清除敏感信息
  static Future<void> clearContext() async {
    await setUser(); // 清除用户信息
    Sentry.configureScope((scope) {
      scope.clear();
    });
  }
}

/// 错误捕获包装器
///
/// 用于包装可能抛出异常的代码块
class ErrorCapture {
  ErrorCapture._();

  /// 包装异步操作，自动捕获异常
  ///
  /// 示例:
  /// ```dart
  /// await ErrorCapture.wrapAsync(
  ///   () => someRiskyOperation(),
  ///   operationName: 'load_workspace',
  /// );
  /// ```
  static Future<T?> wrapAsync<T>(
    Future<T> Function() operation, {
    String? operationName,
    Map<String, dynamic>? context,
    T? defaultValue,
  }) async {
    try {
      // 添加操作面包屑
      if (operationName != null) {
        SentryUtils.addBreadcrumb(
          message: '开始操作: $operationName',
          category: 'operation',
          data: context,
        );
      }

      final result = await operation();

      // 记录成功
      if (operationName != null) {
        SentryUtils.addBreadcrumb(
          message: '操作成功: $operationName',
          category: 'operation',
          data: context,
        );
      }

      return result;
    } catch (exception, stackTrace) {
      // 记录失败
      if (operationName != null) {
        SentryUtils.addBreadcrumb(
          message: '操作失败: $operationName',
          category: 'operation',
          level: SentryLevel.error,
          data: {
            ...?context,
            'error': exception.toString(),
          },
        );
      }

      // 设置上下文并发送到 Sentry
      if (context != null) {
        await SentryUtils.setContext('error_context', context);
      }

      await SentryUtils.captureException(
        exception,
        stackTrace: stackTrace,
        hint: {'operation': operationName},
      );

      // 返回默认值或重新抛出
      if (defaultValue != null) {
        return defaultValue;
      }

      rethrow;
    }
  }

  /// 包装同步操作，自动捕获异常
  static T? wrapSync<T>(
    T Function() operation, {
    String? operationName,
    Map<String, dynamic>? context,
    T? defaultValue,
  }) {
    try {
      return operation();
    } catch (exception, stackTrace) {
      // 异步发送到 Sentry
      SentryUtils.captureException(
        exception,
        stackTrace: stackTrace,
        hint: {'operation': operationName, ...?context},
      );

      if (defaultValue != null) {
        return defaultValue;
      }

      rethrow;
    }
  }
}
