import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core/router/app_router.dart';
import 'core/sentry/sentry_config.dart';
import 'core/theme/app_theme.dart';
import 'l10n/app_localizations.dart';
import 'shared/providers/theme_provider.dart';

/// Log Analyzer Flutter 主入口
///
/// 从 React + Tauri 迁移到 Flutter + Rust FFI
/// 架构：Riverpod 状态管理 + go_router 路由 + Material 3 设计
///
/// Sentry 错误追踪:
/// - 仅在 Release 模式启用
/// - DSN 通过环境变量 SENTRY_DSN 配置
/// - 自动捕获 Flutter 错误和未处理异常
///
/// 注意：FFI 初始化现在由 SplashPage 延迟执行
void main() async {
  // 确保 Flutter 绑定初始化
  WidgetsFlutterBinding.ensureInitialized();

  // FFI 初始化现在由 SplashPage 延迟执行
  // 这样可以显示启动画面和错误状态

  // 使用 Sentry 初始化器包装应用启动
  // 如果 Sentry 未启用（调试模式或 DSN 未配置），则直接启动应用
  SentryInitializer.initialize(
    runApp: () => runApp(const ProviderScope(child: LogAnalyzerApp())),
  );
}

/// 应用根组件
///
/// 对应 React 版本的 src/App.tsx
/// 集成 Sentry 错误追踪和性能监控
class LogAnalyzerApp extends ConsumerWidget {
  const LogAnalyzerApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider);
    final themeNotifier = ref.watch(themeProvider);

    return MaterialApp.router(
      title: 'Log Analyzer',

      // 主题配置 - 支持亮色/暗色/跟随系统三种模式
      theme: lightTheme(),
      darkTheme: darkTheme(),
      themeMode: themeNotifier.themeMode,

      // 路由配置 - 已在 routerProvider 中配置 Sentry 导航观察器
      routerConfig: router,

      // 国际化配置
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: const [
        Locale('zh'), // 中文
        Locale('en'), // 英文
      ],

      // 调试横幅
      debugShowCheckedModeBanner: false,
    );
  }
}
