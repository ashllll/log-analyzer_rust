import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../shared/services/settings_service.dart';
import '../providers/settings_provider.dart';
import '../providers/theme_provider.dart';
import 'widgets/settings_sidebar.dart';
import 'widgets/basic_settings_tab.dart';
import 'widgets/workspace_settings_tab.dart';
import 'widgets/search_settings_tab.dart';
import 'widgets/about_tab.dart';

/// SharedPreferences 实例 Provider
final sharedPreferencesProvider = Provider<SharedPreferences>((ref) {
  throw UnimplementedError('SharedPreferences provider must be initialized');
});

/// SettingsPage 页面专用的设置服务 Provider
/// 用于页面初始化时基于 SharedPreferences 实例创建服务
final settingsPageServiceProvider = Provider<SettingsService>((ref) {
  final prefs = ref.watch(sharedPreferencesProvider);
  return SettingsService(prefs);
});

/// 包装组件：初始化 Providers
class SettingsPageWrapper extends ConsumerStatefulWidget {
  const SettingsPageWrapper({super.key});

  @override
  ConsumerState<SettingsPageWrapper> createState() =>
      _SettingsPageWrapperState();
}

class _SettingsPageWrapperState extends ConsumerState<SettingsPageWrapper> {
  late Future<SharedPreferences> _prefsFuture;

  @override
  void initState() {
    super.initState();
    _prefsFuture = SharedPreferences.getInstance();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<SharedPreferences>(
      future: _prefsFuture,
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.waiting) {
          return const Scaffold(
            body: Center(child: CircularProgressIndicator()),
          );
        }

        if (snapshot.hasError) {
          return Scaffold(
            body: Center(child: Text('初始化失败: ${snapshot.error}')),
          );
        }

        final prefs = snapshot.data!;

        return ProviderScope(
          overrides: [
            sharedPreferencesProvider.overrideWithValue(prefs),
            settingsProvider.overrideWith(
              (ref) => SettingsNotifier(SettingsService(prefs)),
            ),
            themeModeProvider.overrideWith((ref) => ThemeModeNotifier(prefs)),
          ],
          child: const SettingsPage(),
        );
      },
    );
  }
}

/// 设置页面
///
/// 使用左侧 NavigationRail 导航布局，包含四个分类：
/// - 基础设置：主题切换
/// - 工作区设置：最近工作区列表
/// - 搜索设置：搜索历史记录数
/// - 关于：应用信息
class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedIndex = ref.watch(settingsTabIndexProvider);

    return Scaffold(
      body: Row(
        children: [
          // 左侧导航
          const SettingsSidebar(),
          const VerticalDivider(thickness: 1, width: 1),
          // 右侧内容
          Expanded(child: _buildContent(selectedIndex)),
        ],
      ),
    );
  }

  Widget _buildContent(int index) {
    switch (index) {
      case 0:
        return const BasicSettingsTab();
      case 1:
        return const WorkspaceSettingsTab();
      case 2:
        return const SearchSettingsTab();
      case 3:
        return const AboutTab();
      default:
        return const BasicSettingsTab();
    }
  }
}
