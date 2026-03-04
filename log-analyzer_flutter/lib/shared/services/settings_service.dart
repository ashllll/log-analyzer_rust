import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

/// 设置服务 - SharedPreferences 封装
///
/// 提供命名空间式存储，键名使用 'settings.' 前缀
/// 支持主题设置、工作区设置、搜索设置等功能
class SettingsService {
  /// 命名空间前缀
  static const String _prefix = 'settings.';

  /// 键名常量
  static const String keyTheme = '${_prefix}theme';
  static const String keyRecentWorkspaces = '${_prefix}recent_workspaces';
  static const String keySearchHistoryLimit = '${_prefix}search_history_limit';
  static const String keyLastWorkspaceId = '${_prefix}last_workspace_id';
  static const String keySettingsVersion = '${_prefix}version';

  /// 默认值常量
  static const String defaultTheme = 'system';
  static const int defaultSearchHistoryLimit = 50;
  static const int maxRecentWorkspaces = 5;
  static const String settingsVersion = '1.0.0';

  final SharedPreferences _prefs;

  SettingsService(this._prefs);

  /// 创建 SettingsService 实例
  static Future<SettingsService> create() async {
    final prefs = await SharedPreferences.getInstance();
    return SettingsService(prefs);
  }

  // ========== 主题设置 ==========

  /// 获取主题设置
  /// 返回值: 'light' | 'dark' | 'system'
  String getTheme() => _prefs.getString(keyTheme) ?? defaultTheme;

  /// 设置主题
  Future<bool> setTheme(String value) {
    if (!['light', 'dark', 'system'].contains(value)) {
      throw ArgumentError('Invalid theme value: $value');
    }
    return _prefs.setString(keyTheme, value);
  }

  // ========== 最近工作区 ==========

  /// 获取最近工作区列表（最多5个）
  List<String> getRecentWorkspaces() {
    final json = _prefs.getString(keyRecentWorkspaces);
    if (json == null) return [];
    try {
      final list = jsonDecode(json);
      if (list is List) {
        return List<String>.from(list.take(maxRecentWorkspaces));
      }
      return [];
    } catch (e) {
      return [];
    }
  }

  /// 设置最近工作区列表
  Future<bool> setRecentWorkspaces(List<String> workspaces) {
    final limited = workspaces.take(maxRecentWorkspaces).toList();
    return _prefs.setString(keyRecentWorkspaces, jsonEncode(limited));
  }

  /// 添加最近工作区
  Future<bool> addRecentWorkspace(String id) async {
    final list = getRecentWorkspaces();
    list.remove(id);
    list.insert(0, id);
    return setRecentWorkspaces(list.take(maxRecentWorkspaces).toList());
  }

  /// 移除指定工作区
  Future<bool> removeRecentWorkspace(String id) async {
    final list = getRecentWorkspaces();
    list.remove(id);
    return setRecentWorkspaces(list);
  }

  /// 清空最近工作区
  Future<bool> clearRecentWorkspaces() {
    return _prefs.remove(keyRecentWorkspaces);
  }

  // ========== 搜索历史限制 ==========

  /// 获取搜索历史记录数
  int getSearchHistoryLimit() =>
      _prefs.getInt(keySearchHistoryLimit) ?? defaultSearchHistoryLimit;

  /// 设置搜索历史记录数
  Future<bool> setSearchHistoryLimit(int value) {
    if (value < 10 || value > 200) {
      throw ArgumentError('Search history limit must be between 10 and 200');
    }
    return _prefs.setInt(keySearchHistoryLimit, value);
  }

  // ========== 最后工作区 ID（启动恢复） ==========

  /// 获取最后工作区 ID
  String? getLastWorkspaceId() => _prefs.getString(keyLastWorkspaceId);

  /// 设置最后工作区 ID
  Future<bool> setLastWorkspaceId(String? id) {
    if (id == null) {
      return _prefs.remove(keyLastWorkspaceId);
    }
    return _prefs.setString(keyLastWorkspaceId, id);
  }

  // ========== 数据迁移 ==========

  /// 获取设置版本
  String getSettingsVersion() =>
      _prefs.getString(keySettingsVersion) ?? '0.0.0';

  /// 检查是否需要迁移
  bool needsMigration() {
    return getSettingsVersion() != settingsVersion;
  }

  /// 执行数据迁移
  Future<bool> migrate() async {
    final currentVersion = getSettingsVersion();

    // 如果是首次设置版本，直接设置版本号
    if (currentVersion == '0.0.0') {
      await _prefs.setString(keySettingsVersion, settingsVersion);
      return true;
    }

    // 这里可以添加更多迁移逻辑
    // 当前版本到 1.0.0 无需特殊迁移

    await _prefs.setString(keySettingsVersion, settingsVersion);
    return true;
  }

  // ========== 导出/导入 ==========

  /// 导出所有设置到 JSON
  Map<String, dynamic> exportSettings() {
    return {
      'theme': getTheme(),
      'recent_workspaces': getRecentWorkspaces(),
      'search_history_limit': getSearchHistoryLimit(),
      'last_workspace_id': getLastWorkspaceId(),
      'exported_at': DateTime.now().toIso8601String(),
      'version': settingsVersion,
    };
  }

  /// 从 JSON 导入设置
  Future<bool> importSettings(Map<String, dynamic> data) async {
    try {
      if (data.containsKey('theme')) {
        await setTheme(data['theme']);
      }
      if (data.containsKey('recent_workspaces')) {
        final list = data['recent_workspaces'];
        if (list is List) {
          await setRecentWorkspaces(List<String>.from(list));
        }
      }
      if (data.containsKey('search_history_limit')) {
        await setSearchHistoryLimit(data['search_history_limit']);
      }
      if (data.containsKey('last_workspace_id')) {
        final id = data['last_workspace_id'];
        await setLastWorkspaceId(id);
      }
      return true;
    } catch (e) {
      return false;
    }
  }

  /// 清空所有设置
  Future<bool> clearAll() {
    return _prefs.remove(keyTheme).then((_) {
      return _prefs.remove(keyRecentWorkspaces);
    }).then((_) {
      return _prefs.remove(keySearchHistoryLimit);
    }).then((_) {
      return _prefs.remove(keyLastWorkspaceId);
    }).then((_) {
      return _prefs.remove(keySettingsVersion);
    });
  }
}
