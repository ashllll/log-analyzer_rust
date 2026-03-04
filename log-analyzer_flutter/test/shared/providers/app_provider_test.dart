import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/app_provider.dart';
import 'package:log_analyzer_flutter/core/constants/app_constants.dart';

void main() {
  group('AppStateProvider', () {
    test('初始状态应该正确', () {
      final container = ProviderContainer();
      final state = container.read(appStateProvider);

      expect(state.currentPage, AppPage.search);
      expect(state.toasts, isEmpty);
      expect(state.activeWorkspaceId, isNull);
      expect(state.isInitialized, false);

      container.dispose();
    });

    test('setPage 应该更新当前页面', () {
      final container = ProviderContainer();
      final notifier = container.read(appStateProvider.notifier);

      notifier.setPage(AppPage.keywords);

      expect(container.read(appStateProvider).currentPage, AppPage.keywords);

      container.dispose();
    });

    test('addToast 应该添加 Toast 消息', () {
      final container = ProviderContainer();
      final notifier = container.read(appStateProvider.notifier);

      notifier.addToast(ToastType.success, '测试消息');

      final toasts = container.read(appStateProvider).toasts;
      expect(toasts, hasLength(1));
      expect(toasts.first.type, ToastType.success);
      expect(toasts.first.message, '测试消息');

      container.dispose();
    });

    test('setActiveWorkspace 应该更新活动工作区', () {
      final container = ProviderContainer();
      final notifier = container.read(appStateProvider.notifier);

      notifier.setActiveWorkspace('test-workspace-id');

      expect(container.read(appStateProvider).activeWorkspaceId, 'test-workspace-id');

      container.dispose();
    });

    test('removeToast 应该移除指定的 Toast', () {
      final container = ProviderContainer();
      final notifier = container.read(appStateProvider.notifier);

      notifier.addToast(ToastType.success, '消息1');
      notifier.addToast(ToastType.error, '消息2');

      var toasts = container.read(appStateProvider).toasts;
      expect(toasts, hasLength(2));

      final firstToastId = toasts.first.id;
      notifier.removeToast(firstToastId);

      toasts = container.read(appStateProvider).toasts;
      expect(toasts, hasLength(1));

      container.dispose();
    });
  });
}
