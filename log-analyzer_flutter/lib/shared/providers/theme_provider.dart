// lib/shared/providers/theme_provider.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 主题模式存储键
const String themeModeKey = 'settings.theme_mode';

/// ThemeMode 枚举的字符串映射
const Map<ThemeMode, String> _themeModeToString = {
  ThemeMode.system: 'system',
  ThemeMode.light: 'light',
  ThemeMode.dark: 'dark',
};

/// 字符串到 ThemeMode 的映射
final Map<String, ThemeMode> _stringToThemeMode = {
  'system': ThemeMode.system,
  'light': ThemeMode.light,
  'dark': ThemeMode.dark,
};

/// ThemeMode 枚举的显示名称
const Map<ThemeMode, String> themeModeDisplayNames = {
  ThemeMode.system: '跟随系统',
  ThemeMode.light: '浅色模式',
  ThemeMode.dark: '深色模式',
};

/// 主题模式 Provider (简化版)
///
/// 负责管理应用主题模式，支持亮色/暗色/跟随系统三种模式
/// 主题设置持久化到 SharedPreferences
class ThemeNotifier extends ChangeNotifier {
  ThemeMode _themeMode = ThemeMode.dark;

  ThemeMode get themeMode => _themeMode;

  ThemeNotifier() {
    _loadThemeMode();
  }

  /// 异步加载保存的主题模式
  Future<void> _loadThemeMode() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final themeString = prefs.getString(themeModeKey);
      if (themeString != null && _stringToThemeMode.containsKey(themeString)) {
        _themeMode = _stringToThemeMode[themeString]!;
        notifyListeners();
      }
    } catch (e) {
      debugPrint('ThemeNotifier: 加载主题模式失败: $e');
    }
  }

  /// 设置主题模式
  ///
  /// [mode] 主题模式
  Future<void> setThemeMode(ThemeMode mode) async {
    _themeMode = mode;
    notifyListeners();
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(themeModeKey, _themeModeToString[mode] ?? 'dark');
      debugPrint('ThemeNotifier: 主题模式已设置为 ${_themeModeToString[mode]}');
    } catch (e) {
      debugPrint('ThemeNotifier: 保存主题模式失败: $e');
    }
  }

  /// 切换到亮色主题
  Future<void> setLightMode() => setThemeMode(ThemeMode.light);

  /// 切换到暗色主题
  Future<void> setDarkMode() => setThemeMode(ThemeMode.dark);

  /// 切换到跟随系统
  Future<void> setSystemMode() => setThemeMode(ThemeMode.system);
}

/// 主题模式 Provider
final themeProvider = Provider<ThemeNotifier>((ref) {
  return ThemeNotifier();
});
