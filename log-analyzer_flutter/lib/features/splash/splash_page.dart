// lib/features/splash/splash_page.dart
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../shared/providers/app_provider.dart';
import '../../shared/providers/workspace_provider.dart';
import '../../shared/services/bridge_service.dart';
import '../../shared/services/error_handler.dart';
import '../../shared/widgets/error_view.dart';

/// Splash Screen 页面
///
/// 负责 FFI 初始化检测、工作区自动恢复和启动流程
class SplashPage extends ConsumerStatefulWidget {
  const SplashPage({super.key});

  @override
  ConsumerState<SplashPage> createState() => _SplashPageState();
}

class _SplashPageState extends ConsumerState<SplashPage> {
  static const _timeout = Duration(seconds: 10);
  static const _lastWorkspaceIdKey = 'settings.last_workspace_id';
  String _status = '正在连接后端...';
  AppException? _error;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    try {
      // 1. 初始化 FFI 桥接，带超时
      await BridgeService.instance.initialize().timeout(_timeout);

      if (!mounted) return;

      // 2. 初始化成功后加载配置
      ref.read(appStateProvider.notifier).loadConfig();

      // 3. 尝试恢复上次工作区
      final workspaceRestored = await _tryRestoreLastWorkspace();

      if (!mounted) return;

      // 4. 根据恢复结果跳转到对应页面
      if (workspaceRestored) {
        // 工作区恢复成功，跳转到搜索页
        context.go('/search');
      } else {
        // 无法恢复，跳转到工作区列表
        context.go('/workspaces');
      }
    } on TimeoutException {
      if (!mounted) return;
      setState(() {
        _status = '连接超时';
        _error = const AppException(
          code: ErrorCodes.timeout,
          message: '后端连接超时',
          help: '请检查 Rust 后端是否正在运行',
        );
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _status = '连接失败';
        _error = AppException(
          code: ErrorCodes.ffiLoadFailed,
          message: '无法加载 Rust 后端',
          originalError: e,
        );
      });
    }
  }

  /// 尝试恢复上次工作区
  ///
  /// 返回 true 表示恢复成功，false 表示无法恢复
  Future<bool> _tryRestoreLastWorkspace() async {
    try {
      // 获取上次工作区 ID
      final prefs = await SharedPreferences.getInstance();
      final lastWorkspaceId = prefs.getString(_lastWorkspaceIdKey);

      if (lastWorkspaceId == null) {
        debugPrint('SplashPage: 没有上次工作区记录');
        return false;
      }

      debugPrint('SplashPage: 尝试恢复工作区 $lastWorkspaceId');

      // 加载工作区列表
      await ref.read(workspaceStateProvider.notifier).loadWorkspaces();

      // 检查工作区是否存在于列表中
      final workspaces = ref.read(workspaceStateProvider);
      final exists = workspaces.any((w) => w.id == lastWorkspaceId);

      if (!exists) {
        debugPrint('SplashPage: 上次工作区不存在于列表中');
        return false;
      }

      // 尝试加载工作区
      final success = await ref
          .read(workspaceStateProvider.notifier)
          .loadWorkspaceById(lastWorkspaceId);

      if (success) {
        debugPrint('SplashPage: 工作区恢复成功');
        return true;
      } else {
        debugPrint('SplashPage: 工作区加载失败');
        return false;
      }
    } catch (e) {
      debugPrint('SplashPage: 恢复工作区时发生错误: $e');
      return false;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: _error != null
          ? ErrorView(
              exception: _error!,
              onRetry: () {
                setState(() {
                  _error = null;
                  _status = '正在连接后端...';
                });
                _initialize();
              },
            )
          : Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  // 应用图标
                  Icon(
                    Icons.analytics,
                    size: 80,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                  const SizedBox(height: 24),

                  // 应用名称
                  Text(
                    'Log Analyzer',
                    style: Theme.of(context).textTheme.headlineMedium,
                  ),
                  const SizedBox(height: 16),

                  // 加载指示器
                  const CircularProgressIndicator(),
                  const SizedBox(height: 16),

                  // 状态文字
                  Text(
                    _status,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: Colors.grey[400],
                        ),
                  ),
                ],
              ),
            ),
    );
  }
}
