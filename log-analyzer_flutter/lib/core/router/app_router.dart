import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:sentry_flutter/sentry_flutter.dart';

import '../../features/splash/splash_page.dart';
import '../../features/search/presentation/search_page.dart';
import '../../features/workspace/presentation/workspaces_page.dart';
import '../../features/keyword/presentation/keywords_page.dart';
import '../../features/task/presentation/tasks_page.dart';
import '../../features/settings/presentation/settings_page.dart';
import '../../features/performance/presentation/performance_page.dart';
import '../../features/archive_browsing/presentation/pages/archive_browser_page.dart';

/// 路由配置 Provider
///
/// 对应 React 版本的状态驱动路由 (appStore.page)
/// 使用 go_router 实现声明式路由
/// 集成 Sentry 导航观察器用于追踪页面访问
final routerProvider = Provider<GoRouter>((ref) {
  return GoRouter(
    initialLocation: '/splash',
    debugLogDiagnostics: true,
    routes: $routes,
    // 添加 Sentry 导航观察器 - 自动记录页面导航
    observers: [SentryNavigatorObserver()],
  );
});

/// 路由配置列表
///
/// 支持的路由：
/// - /splash -> 启动画面（初始路由）
/// - /search -> 搜索页面
/// - /workspaces -> 工作区管理
/// - /keywords -> 关键词管理
/// - /tasks -> 任务管理
/// - /settings -> 设置
/// - /performance -> 性能监控
final $routes = [
  GoRoute(path: '/', redirect: (_, __) => '/splash'),
  // Splash 页面 - 应用启动时显示
  GoRoute(
    path: '/splash',
    name: 'splash',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const SplashPage()),
  ),
  GoRoute(path: '/home', name: 'home', redirect: (_, __) => '/search'),
  GoRoute(
    path: '/search',
    name: 'search',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const SearchPage()),
  ),
  GoRoute(
    path: '/workspaces',
    name: 'workspaces',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const WorkspacesPage()),
  ),
  GoRoute(
    path: '/keywords',
    name: 'keywords',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const KeywordsPage()),
  ),
  GoRoute(
    path: '/tasks',
    name: 'tasks',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const TasksPage()),
  ),
  GoRoute(
    path: '/settings',
    name: 'settings',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const SettingsPage()),
  ),
  GoRoute(
    path: '/performance',
    name: 'performance',
    pageBuilder: (context, state) =>
        MaterialPage(key: state.pageKey, child: const PerformancePage()),
  ),
  // 压缩包浏览页面
  GoRoute(
    path: '/archive-browser',
    name: 'archive-browser',
    pageBuilder: (context, state) {
      final archivePath = state.uri.queryParameters['path'] ?? '';
      return MaterialPage(
        key: state.pageKey,
        child: ArchiveBrowserPage(
          archivePath: Uri.decodeComponent(archivePath),
        ),
      );
    },
  ),
];

// 类型安全的路由类（代码生成占位符）
class RootRoute extends GoRouteData {
  RootRoute();
}

class SplashRoute extends GoRouteData {
  SplashRoute();
}

class HomeRoute extends GoRouteData {
  HomeRoute();
}

class SearchRoute extends GoRouteData {
  SearchRoute();
}

class WorkspacesRoute extends GoRouteData {
  WorkspacesRoute();
}

class KeywordsRoute extends GoRouteData {
  KeywordsRoute();
}

class TasksRoute extends GoRouteData {
  TasksRoute();
}

class SettingsRoute extends GoRouteData {
  SettingsRoute();
}

class PerformanceRoute extends GoRouteData {
  PerformanceRoute();
}

// AppPage 枚举定义已移至 core/constants/app_constants.dart
// 为避免重复定义，此处仅保留路由逻辑
// 如需使用 AppPage，请导入: import 'package:log_analyzer_flutter/core/constants/app_constants.dart';
