import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../services/settings_service.dart';

/// 主题模式字符串转换
ThemeMode themeModeFromString(String value) {
  switch (value) {
    case 'light':
      return ThemeMode.light;
    case 'dark':
      return ThemeMode.dark;
    default:
      return ThemeMode.system;
  }
}

/// 主题模式转字符串
String themeModeToString(ThemeMode mode) {
  switch (mode) {
    case ThemeMode.light:
      return 'light';
    case ThemeMode.dark:
      return 'dark';
    case ThemeMode.system:
      return 'system';
  }
}

/// 主题模式状态管理
class ThemeModeNotifier extends StateNotifier<ThemeMode> {
  final SharedPreferences _prefs;
  static const String _keyTheme = 'settings.theme';

  ThemeModeNotifier(this._prefs) : super(ThemeMode.system) {
    _loadTheme();
  }

  void _loadTheme() {
    final themeValue = _prefs.getString(_keyTheme) ?? 'system';
    state = themeModeFromString(themeValue);
  }

  /// 设置主题
  Future<void> setTheme(ThemeMode mode) async {
    state = mode;
    await _prefs.setString(_keyTheme, themeModeToString(mode));
  }

  /// 切换到浅色主题
  Future<void> setLight() => setTheme(ThemeMode.light);

  /// 切换到深色主题
  Future<void> setDark() => setTheme(ThemeMode.dark);

  /// 切换到跟随系统
  Future<void> setSystem() => setTheme(ThemeMode.system);

  /// 切换主题（在 light/dark 之间切换）
  Future<void> toggle() {
    if (state == ThemeMode.light) {
      return setDark();
    } else {
      return setLight();
    }
  }
}

/// 主题模式 Provider
final themeModeProvider =
    StateNotifierProvider<ThemeModeNotifier, ThemeMode>((ref) {
  throw UnimplementedError('themeModeProvider must be overridden');
});

/// 创建主题 Provider 的辅助函数
Provider<ThemeModeNotifier> createThemeModeProvider(
    SharedPreferences prefs) {
  return Provider<ThemeModeNotifier>((ref) {
    return ThemeModeNotifier(prefs);
  });
}

/// 主题名称 Provider（用于 UI 显示）
final themeNameProvider = Provider<String>((ref) {
  final themeMode = ref.watch(themeModeProvider);
  switch (themeMode) {
    case ThemeMode.light:
      return '浅色';
    case ThemeMode.dark:
      return '深色';
    case ThemeMode.system:
      return '跟随系统';
  }
});

/// 主题图标 Provider（用于 UI 显示）
final themeIconProvider = Provider<IconData>((ref) {
  final themeMode = ref.watch(themeModeProvider);
  switch (themeMode) {
    case ThemeMode.light:
      return Icons.light_mode;
    case ThemeMode.dark:
      return Icons.dark_mode;
    case ThemeMode.system:
      return Icons.settings_brightness;
  }
});
