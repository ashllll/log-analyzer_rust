import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/workspace_tab.dart';
import '../services/tab_persistence_service.dart';

part 'workspace_tab_provider.g.dart';

/// 存储键前缀
const String _activeTabIdKey = 'active_tab_id';

/// 活动标签页 ID Provider
@riverpod
class ActiveTabId extends _$ActiveTabId {
  @override
  String? build() {
    // 从持久化加载活动标签
    Future.microtask(() => _loadActiveTabId());
    return null;
  }

  Future<void> _loadActiveTabId() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      state = prefs.getString(_activeTabIdKey);
    } catch (e) {
      debugPrint('ActiveTabId: 加载活动标签失败: $e');
    }
  }

  Future<void> setActive(String? tabId) async {
    state = tabId;
    if (tabId != null) {
      try {
        final prefs = await SharedPreferences.getInstance();
        await prefs.setString(_activeTabIdKey, tabId);
      } catch (e) {
        debugPrint('ActiveTabId: 保存活动标签失败: $e');
      }
    }
  }
}

/// TabManager Provider - 管理所有标签页
@riverpod
class TabManager extends _$TabManager {
  TabPersistenceService? _persistenceService;

  @override
  List<WorkspaceTab> build() {
    // 延迟加载保存的标签页
    Future.microtask(() => _initAndLoadTabs());
    return [];
  }

  /// 初始化并加载标签页
  Future<void> _initAndLoadTabs() async {
    try {
      _persistenceService = await TabPersistenceService.create();
      await _loadTabs();
    } catch (e) {
      debugPrint('TabManager: 初始化失败: $e');
    }
  }

  /// 从持久化加载标签页
  Future<void> _loadTabs() async {
    if (_persistenceService == null) return;

    final tabs = await _persistenceService!.loadTabs();
    if (tabs.isNotEmpty) {
      state = tabs;
      // 设置活动标签
      final activeTabId = ref.read(activeTabIdProvider);
      if (activeTabId == null && tabs.isNotEmpty) {
        ref.read(activeTabIdProvider.notifier).setActive(tabs.first.id);
      }
    }
  }

  /// 保存标签页到持久化
  Future<void> _saveTabs() async {
    if (_persistenceService == null) return;
    await _persistenceService!.saveTabs(state);
  }

  /// 打开新标签页
  ///
  /// [workspaceId] 工作区 ID
  /// [title] 显示标题
  /// 返回新创建的标签页 ID
  String openTab(String workspaceId, String title) {
    // 检查是否已存在相同工作区的标签页
    final existingIndex = state.indexWhere((t) => t.workspaceId == workspaceId);
    if (existingIndex != -1) {
      // 切换到已存在的标签页
      final existing = state[existingIndex];
      ref.read(activeTabIdProvider.notifier).setActive(existing.id);
      return existing.id;
    }

    // 创建新标签页
    final newTab = WorkspaceTab(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      workspaceId: workspaceId,
      title: title,
      openedAt: DateTime.now(),
    );

    state = [...state, newTab];
    ref.read(activeTabIdProvider.notifier).setActive(newTab.id);
    _saveTabs();

    return newTab.id;
  }

  /// 关闭标签页
  ///
  /// [tabId] 要关闭的标签页 ID
  void closeTab(String tabId) {
    final tabIndex = state.indexWhere((t) => t.id == tabId);
    if (tabIndex == -1) return;

    // 获取当前活动标签
    final activeTabId = ref.read(activeTabIdProvider);

    // 如果关闭的是活动标签，需要切换到其他标签
    if (activeTabId == tabId) {
      if (state.length > 1) {
        // 切换到相邻标签
        final newActiveIndex = tabIndex > 0 ? tabIndex - 1 : 1;
        ref.read(activeTabIdProvider.notifier).setActive(state[newActiveIndex].id);
      } else {
        // 没有其他标签
        ref.read(activeTabIdProvider.notifier).setActive(null);
      }
    }

    state = state.where((t) => t.id != tabId).toList();
    _saveTabs();
  }

  /// 切换到指定标签页
  void switchTab(String tabId) {
    if (state.any((t) => t.id == tabId)) {
      ref.read(activeTabIdProvider.notifier).setActive(tabId);
    }
  }

  /// 重排标签页顺序
  ///
  /// [oldIndex] 原位置
  /// [newIndex] 新位置
  void reorderTabs(int oldIndex, int newIndex) {
    if (oldIndex < newIndex) {
      newIndex -= 1;
    }
    final tabs = List<WorkspaceTab>.from(state);
    final tab = tabs.removeAt(oldIndex);
    tabs.insert(newIndex, tab);
    state = tabs;
    _saveTabs();
  }

  /// 固定/取消固定标签页
  void togglePin(String tabId) {
    state = state.map((t) {
      if (t.id == tabId) {
        return t.copyWith(isPinned: !t.isPinned);
      }
      return t;
    }).toList();
    _saveTabs();
  }

  /// 关闭所有标签页
  void closeAllTabs() {
    state = [];
    ref.read(activeTabIdProvider.notifier).setActive(null);
    _saveTabs();
  }

  /// 关闭其他标签页
  void closeOtherTabs(String keepTabId) {
    state = state.where((t) => t.id == keepTabId).toList();
    ref.read(activeTabIdProvider.notifier).setActive(keepTabId);
    _saveTabs();
  }
}
