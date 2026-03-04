import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/app_state.dart';
import '../services/api_service.dart';
import '../../core/constants/app_constants.dart' show AppPage, ToastType;

part 'app_provider.g.dart';

/// 应用状态 Provider
///
/// 对应 React 版本的 appStore.ts
/// 管理应用级别的全局状态
///
/// 注意：FFI 初始化现在由 SplashPage 处理（延迟加载）
/// 此 Provider 负责配置加载
@riverpod
class AppState extends _$AppState {
  @override
  AppModel build() {
    // 初始化应用（延迟执行避免在 build 中直接调用异步方法）
    // 注意：FFI 初始化现在由 SplashPage 处理
    Future.microtask(() => _initializeApp());
    return const AppModel();
  }

  /// 初始化应用
  ///
  /// 只负责加载配置（FFI 初始化由 SplashPage 处理）
  /// 工作区和关键词组的加载由各自的 Provider 自行处理
  Future<void> _initializeApp() async {
    try {
      debugPrint('AppState: 开始初始化应用...');

      // FFI 初始化现在由 SplashPage 处理
      // 此处只需要加载配置

      // 加载配置
      await _loadConfig();

      // 标记初始化完成
      state = state.copyWith(isInitialized: true);
      debugPrint('AppState: 应用初始化完成');
    } catch (e) {
      debugPrint('AppState: 应用初始化失败: $e');
      state = state.copyWith(
        isInitialized: false,
        initializationError: e.toString(),
      );
    }
  }

  /// 加载配置
  ///
  /// 在 FFI 初始化完成后调用
  Future<void> loadConfig() async {
    await _loadConfig();
  }

  /// 加载配置（内部方法）
  Future<void> _loadConfig() async {
    try {
      final apiService = ref.read(apiServiceProvider);

      if (!apiService.isFfiAvailable) {
        debugPrint('AppState: FFI 桥接不可用，跳过加载配置');
        return;
      }

      // 加载配置并打印日志
      await apiService.loadConfig();
      debugPrint('AppState: 配置加载完成');
    } catch (e) {
      debugPrint('AppState: 加载配置失败: $e');
      // 配置加载失败不影响应用初始化
    }
  }

  /// 切换页面
  ///
  /// 对应 React 版本的 setPage()
  void setPage(AppPage page) {
    state = state.copyWith(currentPage: page);
  }

  /// 添加 Toast 消息
  ///
  /// 对应 React 版本的 addToast()
  void addToast(ToastType type, String message, {int? duration}) {
    final toast = Toast(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      type: type,
      message: message,
      duration: duration ?? 3000,
    );
    state = state.copyWith(toasts: [...state.toasts, toast]);

    // 自动移除 Toast
    if (toast.duration != null) {
      Future.delayed(Duration(milliseconds: toast.duration!), () {
        removeToast(toast.id);
      });
    }
  }

  /// 移除 Toast 消息
  void removeToast(String id) {
    state = state.copyWith(
      toasts: state.toasts.where((t) => t.id != id).toList(),
    );
  }

  /// 设置活动工作区
  ///
  /// 对应 React 版本的 setActiveWorkspace()
  void setActiveWorkspace(String? id) {
    state = state.copyWith(activeWorkspaceId: id);
  }

  /// 清除所有 Toast
  void clearAllToasts() {
    state = state.copyWith(toasts: []);
  }

  /// 重新初始化应用
  ///
  /// 用于刷新所有数据
  Future<void> reinitialize() async {
    state = state.copyWith(
      isInitialized: false,
      initializationError: null,
    );
    await _initializeApp();
  }
}
