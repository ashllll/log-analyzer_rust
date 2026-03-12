import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:shared_preferences/shared_preferences.dart';

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

/// SharedPreferences Provider
final sharedPreferencesProvider = Provider<SharedPreferences>((ref) {
  throw UnimplementedError('sharedPreferencesProvider must be overridden');
});

/// 主题模式 Provider（使用 StateProvider）
final themeModeProvider = StateProvider<ThemeMode>((ref) {
  final prefs = ref.watch(sharedPreferencesProvider);
  final themeValue = prefs.getString('settings.theme') ?? 'system';
  return themeModeFromString(themeValue);
});

/// 设置主题
Future<void> setTheme(WidgetRef ref, ThemeMode mode) async {
  final prefs = ref.read(sharedPreferencesProvider);
  await prefs.setString('settings.theme', themeModeToString(mode));
  ref.read(themeModeProvider.notifier).state = mode;
}

/// 切换到浅色主题
Future<void> setLight(WidgetRef ref) => setTheme(ref, ThemeMode.light);

/// 切换到深色主题
Future<void> setDark(WidgetRef ref) => setTheme(ref, ThemeMode.dark);

/// 切换到跟随系统
Future<void> setSystem(WidgetRef ref) => setTheme(ref, ThemeMode.system);

/// 切换主题（在 light/dark 之间切换）
Future<void> toggleTheme(WidgetRef ref) {
  final currentMode = ref.read(themeModeProvider);
  if (currentMode == ThemeMode.light) {
    return setDark(ref);
  } else {
    return setLight(ref);
  }
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
