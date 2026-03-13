/// Log Analyzer Flutter - 重构版主入口
/// 
/// 架构改进：
/// 1. 无副作用错误处理
/// 2. AsyncNotifier 状态管理
/// 3. 事件驱动任务更新（替代轮询）
/// 4. Isolate FFI 调用

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'core/errors/error_handler.dart';
import 'core/theme/app_theme.dart';
import 'data/datasources/ffi_datasource.dart';
import 'data/datasources/event_datasource.dart';
import 'presentation/pages/workspaces/workspaces_page.dart';
import 'presentation/pages/tasks/tasks_page.dart';

void main() {
  // 使用 ErrorHandler.runInZone 捕获所有异步错误
  ErrorHandler.runInZone(() {
    // 初始化全局错误处理
    ErrorHandler.initialize(
      config: ErrorHandlerConfig.debug,
    );
    
    runApp(
      ErrorHandler.wrapApp(
        const ProviderScope(
          child: LogAnalyzerApp(),
        ),
      ),
    );
  });
}

/// 应用根组件
class LogAnalyzerApp extends ConsumerStatefulWidget {
  const LogAnalyzerApp({super.key});

  @override
  ConsumerState<LogAnalyzerApp> createState() => _LogAnalyzerAppState();
}

class _LogAnalyzerAppState extends ConsumerState<LogAnalyzerApp> {
  bool _isInitializing = true;
  Object? _initError;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    try {
      // 初始化 FFI
      await FfiDataSource.instance.initialize();
      
      // 初始化事件源
      EventDataSource.instance.initialize();
      
      setState(() {
        _isInitializing = false;
      });
    } catch (e) {
      setState(() {
        _initError = e;
        _isInitializing = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    // 初始化中显示加载界面
    if (_isInitializing) {
      return MaterialApp(
        home: Scaffold(
          body: Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                const CircularProgressIndicator(),
                const SizedBox(height: 16),
                Text(
                  '正在初始化...',
                  style: Theme.of(context).textTheme.bodyLarge,
                ),
              ],
            ),
          ),
        ),
      );
    }

    // 初始化失败显示错误界面
    if (_initError != null) {
      return MaterialApp(
        home: Scaffold(
          body: Center(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const Icon(Icons.error_outline, size: 64, color: Colors.red),
                  const SizedBox(height: 16),
                  Text(
                    '初始化失败',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    _initError.toString(),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 24),
                  ElevatedButton(
                    onPressed: _initialize,
                    child: const Text('重试'),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
    }

    // 正常启动应用
    return MaterialApp(
      title: 'Log Analyzer',
      theme: lightTheme(),
      darkTheme: darkTheme(),
      themeMode: ThemeMode.system,
      debugShowCheckedModeBanner: false,
      home: const MainNavigation(),
    );
  }
}

/// 主导航
class MainNavigation extends StatefulWidget {
  const MainNavigation({super.key});

  @override
  State<MainNavigation> createState() => _MainNavigationState();
}

class _MainNavigationState extends State<MainNavigation> {
  int _selectedIndex = 0;

  final _pages = const [
    WorkspacesPage(),
    TasksPage(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Row(
        children: [
          // 侧边导航
          NavigationRail(
            selectedIndex: _selectedIndex,
            onDestinationSelected: (index) {
              setState(() {
                _selectedIndex = index;
              });
            },
            destinations: const [
              NavigationRailDestination(
                icon: Icon(Icons.folder_outlined),
                selectedIcon: Icon(Icons.folder),
                label: Text('工作区'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.task_outlined),
                selectedIcon: Icon(Icons.task),
                label: Text('任务'),
              ),
            ],
          ),
          
          // 垂直分割线
          const VerticalDivider(thickness: 1, width: 1),
          
          // 页面内容
          Expanded(
            child: _pages[_selectedIndex],
          ),
        ],
      ),
    );
  }
}

/// 简单主题配置
ThemeData lightTheme() {
  return ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: Colors.blue,
      brightness: Brightness.light,
    ),
  );
}

ThemeData darkTheme() {
  return ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: Colors.blue,
      brightness: Brightness.dark,
    ),
  );
}
