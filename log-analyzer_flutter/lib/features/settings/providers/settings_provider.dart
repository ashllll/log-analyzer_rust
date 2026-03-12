import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../../../shared/services/settings_service.dart';

/// 设置服务 Provider
final settingsServiceProvider = Provider<SettingsService>((ref) {
  throw UnimplementedError('settingsServiceProvider must be overridden');
});

/// 当前选中的设置 Tab 索引 Provider
final settingsTabIndexProvider = StateProvider<int>((ref) => 0);

/// 侧边栏是否展开 Provider
final settingsSidebarExpandedProvider = StateProvider<bool>((ref) => true);

/// 设置状态
class SettingsState {
  final String theme;
  final List<String> recentWorkspaces;
  final int searchHistoryLimit;
  final String? lastWorkspaceId;
  final bool isLoading;
  final String? error;

  const SettingsState({
    this.theme = 'system',
    this.recentWorkspaces = const [],
    this.searchHistoryLimit = 50,
    this.lastWorkspaceId,
    this.isLoading = false,
    this.error,
  });

  SettingsState copyWith({
    String? theme,
    List<String>? recentWorkspaces,
    int? searchHistoryLimit,
    String? lastWorkspaceId,
    bool? isLoading,
    String? error,
  }) {
    return SettingsState(
      theme: theme ?? this.theme,
      recentWorkspaces: recentWorkspaces ?? this.recentWorkspaces,
      searchHistoryLimit: searchHistoryLimit ?? this.searchHistoryLimit,
      lastWorkspaceId: lastWorkspaceId ?? this.lastWorkspaceId,
      isLoading: isLoading ?? this.isLoading,
      error: error,
    );
  }
}

/// 设置状态 Provider（使用 StateProvider）
final settingsProvider = StateProvider<SettingsState>((ref) {
  final service = ref.watch(settingsServiceProvider);
  return SettingsState(
    theme: service.getTheme(),
    recentWorkspaces: service.getRecentWorkspaces(),
    searchHistoryLimit: service.getSearchHistoryLimit(),
    lastWorkspaceId: service.getLastWorkspaceId(),
  );
});

/// 更新主题
Future<void> setTheme(WidgetRef ref, String theme) async {
  final service = ref.read(settingsServiceProvider);
  await service.setTheme(theme);
  ref.read(settingsProvider.notifier).state =
      ref.read(settingsProvider).copyWith(theme: theme);
}

/// 更新搜索历史限制
Future<void> setSearchHistoryLimit(WidgetRef ref, int limit) async {
  final service = ref.read(settingsServiceProvider);
  await service.setSearchHistoryLimit(limit);
  ref.read(settingsProvider.notifier).state =
      ref.read(settingsProvider).copyWith(searchHistoryLimit: limit);
}

/// 添加最近工作区
Future<void> addRecentWorkspace(WidgetRef ref, String id) async {
  final service = ref.read(settingsServiceProvider);
  await service.addRecentWorkspace(id);
  ref.read(settingsProvider.notifier).state = SettingsState(
    theme: service.getTheme(),
    recentWorkspaces: service.getRecentWorkspaces(),
    searchHistoryLimit: service.getSearchHistoryLimit(),
    lastWorkspaceId: id,
  );
}

/// 移除最近工作区
Future<void> removeRecentWorkspace(WidgetRef ref, String id) async {
  final service = ref.read(settingsServiceProvider);
  await service.removeRecentWorkspace(id);
  ref.read(settingsProvider.notifier).state =
      ref.read(settingsProvider).copyWith(
        recentWorkspaces: service.getRecentWorkspaces(),
      );
}

/// 清空最近工作区
Future<void> clearRecentWorkspaces(WidgetRef ref) async {
  final service = ref.read(settingsServiceProvider);
  await service.clearRecentWorkspaces();
  ref.read(settingsProvider.notifier).state =
      ref.read(settingsProvider).copyWith(
        recentWorkspaces: [],
        lastWorkspaceId: null,
      );
}

/// 设置最后工作区 ID
Future<void> setLastWorkspaceId(WidgetRef ref, String? id) async {
  final service = ref.read(settingsServiceProvider);
  await service.setLastWorkspaceId(id);
  ref.read(settingsProvider.notifier).state =
      ref.read(settingsProvider).copyWith(lastWorkspaceId: id);
}
