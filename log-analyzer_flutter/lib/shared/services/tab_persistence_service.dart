import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/workspace_tab.dart';

/// 标签页持久化服务
///
/// 使用 shared_preferences 保存和加载标签页列表
class TabPersistenceService {
  static const String _storageKey = 'workspace_tabs';

  final SharedPreferences _prefs;

  /// 构造函数 - 延迟初始化 SharedPreferences
  TabPersistenceService() : _prefs = throw UnimplementedError('Use async factory');

  /// 异步工厂构造函数
  static Future<TabPersistenceService> create() async {
    final prefs = await SharedPreferences.getInstance();
    return TabPersistenceService.withPrefs(prefs);
  }

  /// 构造函数 - 允许传入 SharedPreferences 实例（方便测试）
  TabPersistenceService.withPrefs(this._prefs);

  /// 保存标签页列表
  Future<void> saveTabs(List<WorkspaceTab> tabs) async {
    try {
      final jsonList = tabs.map((t) => t.toJson()).toList();
      await _prefs.setString(_storageKey, jsonEncode(jsonList));
    } catch (e) {
      // 静默失败，不影响用户体验
      debugPrint('TabPersistenceService: 保存标签页失败: $e');
    }
  }

  /// 加载标签页列表
  Future<List<WorkspaceTab>> loadTabs() async {
    try {
      final jsonString = _prefs.getString(_storageKey);
      if (jsonString == null || jsonString.isEmpty) {
        return [];
      }

      final jsonList = jsonDecode(jsonString) as List<dynamic>;
      return jsonList
          .map((json) => WorkspaceTab.fromJson(json as Map<String, dynamic>))
          .toList();
    } catch (e) {
      debugPrint('TabPersistenceService: 加载标签页失败: $e');
      return [];
    }
  }

  /// 清空所有保存的标签页
  Future<void> clearTabs() async {
    await _prefs.remove(_storageKey);
  }
}
