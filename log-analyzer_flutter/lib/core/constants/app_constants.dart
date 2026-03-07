/// 应用常量定义
///
/// 对应 React 版本中的各种常量
class AppConstants {
  // 应用信息
  static const String appName = 'Log Analyzer';
  static const String appVersion = '1.0.0';

  // 搜索默认值
  static const int defaultMaxResults = 10000;
  static const int searchDebounceMs = 300;
  static const Duration searchDebounce = Duration(milliseconds: searchDebounceMs);

  // 虚拟滚动
  static const double logItemHeight = 32.0;
  static const int logItemCacheExtent = 50;

  // 任务管理
  static const Duration taskCleanupInterval = Duration(minutes: 5);
  static const Duration completedTaskTtl = Duration(minutes: 10);

  // WebSocket
  static const Duration wsReconnectDelay = Duration(seconds: 2);
  static const Duration wsReconnectMaxDelay = Duration(seconds: 30);
  static const Duration wsHeartbeatInterval = Duration(seconds: 30);

  // 缓存
  static const int maxCacheSize = 1000;
  static const Duration cacheTtl = Duration(hours: 1);

  // 文件树缓存
  static const int fileTreeCacheMaxSize = 100;
  static const Duration fileTreeCacheTtl = Duration(minutes: 10);

  // 搜索结果缓存
  static const int searchResultCacheMaxSize = 50;
  static const Duration searchResultCacheTtl = Duration(minutes: 5);

  // 性能目标
  static const int searchLatencyTargetMs = 200;
  static const int fileTreeLoadTargetMs = 500;
  static const int virtualScrollFpsTarget = 30;

  // UI
  static const double borderRadius = 8.0;
  static const double borderRadiusLarge = 12.0;
  static const double borderRadiusSmall = 4.0;

  // 动画
  static const Duration animationFast = Duration(milliseconds: 150);
  static const Duration animationNormal = Duration(milliseconds: 250);
  static const Duration animationSlow = Duration(milliseconds: 400);

  // 存储
  static const String workspaceStorageKey = 'workspaces';
  static const String keywordsStorageKey = 'keywords';
  static const String configStorageKey = 'config';
}

/// 工作区状态枚举
///
/// 对应 React 版本的 Workspace['status']
enum WorkspaceStatus {
  ready('READY'),
  scanning('SCANNING'),
  offline('OFFLINE'),
  processing('PROCESSING');

  final String value;
  const WorkspaceStatus(this.value);

  static WorkspaceStatus fromValue(String value) {
    return WorkspaceStatus.values.firstWhere(
      (e) => e.value == value,
      orElse: () => WorkspaceStatus.offline,
    );
  }
}

/// 任务状态枚举
///
/// 对应 React 版本的 Task['status']
enum TaskStatus {
  running('RUNNING'),
  completed('COMPLETED'),
  failed('FAILED'),
  stopped('STOPPED');

  final String value;
  const TaskStatus(this.value);

  static TaskStatus fromValue(String value) {
    return TaskStatus.values.firstWhere(
      (e) => e.value == value,
      orElse: () => TaskStatus.failed,
    );
  }
}

/// 查询操作符枚举
///
/// 对应 React 版本的 QueryOperator
enum QueryOperator {
  and('AND'),
  or('OR'),
  not('NOT');

  final String value;
  const QueryOperator(this.value);

  static QueryOperator fromValue(String value) {
    return QueryOperator.values.firstWhere(
      (e) => e.value == value,
      orElse: () => QueryOperator.or,
    );
  }
}

/// 术语来源枚举
///
/// 对应 React 版本的 TermSource
enum TermSource {
  user('user'),
  preset('preset');

  final String value;
  const TermSource(this.value);

  static TermSource fromValue(String value) {
    return TermSource.values.firstWhere(
      (e) => e.value == value,
      orElse: () => TermSource.user,
    );
  }
}

/// 颜色键枚举
///
/// 对应 React 版本的 ColorKey
enum ColorKey {
  blue,
  green,
  red,
  orange,
  purple;

  static ColorKey fromValue(String value) {
    return ColorKey.values.firstWhere(
      (e) => e.name == value,
      orElse: () => ColorKey.blue,
    );
  }
}

/// 过滤器模式枚举
enum FilterMode {
  allowlist('allowlist'),
  blocklist('blocklist');

  final String value;
  const FilterMode(this.value);

  static FilterMode fromValue(String value) {
    return FilterMode.values.firstWhere(
      (e) => e.value == value,
      orElse: () => FilterMode.allowlist,
    );
  }
}

/// Toast 类型枚举
///
/// 对应 React 版本的 ToastType
enum ToastType {
  success,
  error,
  info,
  warning,
}

/// 应用页面枚举
///
/// 对应 React 版本的导航页面
enum AppPage {
  search,
  workspaces,
  keywords,
  tasks,
  settings,
  performance,
}

/// 页面扩展方法
extension AppPageExtension on AppPage {
  /// 获取路由路径
  String get path {
    switch (this) {
      case AppPage.search:
        return '/search';
      case AppPage.keywords:
        return '/keywords';
      case AppPage.workspaces:
        return '/workspaces';
      case AppPage.tasks:
        return '/tasks';
      case AppPage.performance:
        return '/performance';
      case AppPage.settings:
        return '/settings';
    }
  }

  /// 从路径获取页面
  static AppPage fromPath(String path) {
    switch (path) {
      case '/search':
        return AppPage.search;
      case '/keywords':
        return AppPage.keywords;
      case '/workspaces':
        return AppPage.workspaces;
      case '/tasks':
        return AppPage.tasks;
      case '/performance':
        return AppPage.performance;
      case '/settings':
        return AppPage.settings;
      default:
        return AppPage.search;
    }
  }
}
