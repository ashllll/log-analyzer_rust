import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../shared/services/settings_service.dart';

/// 设置服务 Provider
final settingsServiceProvider = FutureProvider<SettingsService>((ref) async {
  return SettingsService.create();
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

/// 设置状态管理 Provider
class SettingsNotifier extends StateNotifier<SettingsState> {
  final SettingsService _service;

  SettingsNotifier(this._service) : super(const SettingsState()) {
    _loadSettings();
  }

  void _loadSettings() {
    state = SettingsState(
      theme: _service.getTheme(),
      recentWorkspaces: _service.getRecentWorkspaces(),
      searchHistoryLimit: _service.getSearchHistoryLimit(),
      lastWorkspaceId: _service.getLastWorkspaceId(),
    );
  }

  /// 更新主题
  Future<void> setTheme(String theme) async {
    await _service.setTheme(theme);
    state = state.copyWith(theme: theme);
  }

  /// 更新搜索历史限制
  Future<void> setSearchHistoryLimit(int limit) async {
    await _service.setSearchHistoryLimit(limit);
    state = state.copyWith(searchHistoryLimit: limit);
  }

  /// 添加最近工作区
  Future<void> addRecentWorkspace(String id) async {
    await _service.addRecentWorkspace(id);
    state = state.copyWith(
      recentWorkspaces: _service.getRecentWorkspaces(),
      lastWorkspaceId: id,
    );
  }

  /// 移除最近工作区
  Future<void> removeRecentWorkspace(String id) async {
    await _service.removeRecentWorkspace(id);
    state = state.copyWith(recentWorkspaces: _service.getRecentWorkspaces());
  }

  /// 清空最近工作区
  Future<void> clearRecentWorkspaces() async {
    await _service.clearRecentWorkspaces();
    state = state.copyWith(recentWorkspaces: [], lastWorkspaceId: null);
  }

  /// 设置最后工作区 ID
  Future<void> setLastWorkspaceId(String? id) async {
    await _service.setLastWorkspaceId(id);
    state = state.copyWith(lastWorkspaceId: id);
  }

  /// 导出设置
  Map<String, dynamic> exportSettings() {
    return _service.exportSettings();
  }

  /// 导入设置
  Future<bool> importSettings(Map<String, dynamic> data) async {
    final success = await _service.importSettings(data);
    if (success) {
      _loadSettings();
    }
    return success;
  }
}

/// 设置状态 Provider
final settingsProvider = StateNotifierProvider<SettingsNotifier, SettingsState>(
  (ref) {
    throw UnimplementedError('settingsProvider must be overridden');
  },
);

/// 初始化设置 Provider 的辅助函数
Provider<SettingsNotifier> createSettingsProvider(SettingsService service) {
  return Provider<SettingsNotifier>((ref) {
    return SettingsNotifier(service);
  });
}
