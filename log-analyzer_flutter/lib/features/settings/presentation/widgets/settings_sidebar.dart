import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

import '../../providers/settings_provider.dart';

/// 设置页面侧边栏
///
/// 显示四个分类的导航项：基础设置、工作区设置、搜索设置、关于
class SettingsSidebar extends ConsumerWidget {
  const SettingsSidebar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedIndex = ref.watch(settingsTabIndexProvider);
    final isExpanded = ref.watch(settingsSidebarExpandedProvider);

    return NavigationRail(
      extended: isExpanded,
      minExtendedWidth: 200,
      selectedIndex: selectedIndex,
      onDestinationSelected: (index) {
        ref.read(settingsTabIndexProvider.notifier).state = index;
      },
      leading: Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: IconButton(
          icon: Icon(
            isExpanded ? LucideIcons.panelLeftClose : LucideIcons.panelLeft,
          ),
          onPressed: () {
            ref.read(settingsSidebarExpandedProvider.notifier).state =
                !isExpanded;
          },
          tooltip: isExpanded ? '收起侧边栏' : '展开侧边栏',
        ),
      ),
      destinations: const [
        NavigationRailDestination(
          icon: Icon(LucideIcons.settings),
          selectedIcon: Icon(LucideIcons.settings),
          label: Text('基础设置'),
        ),
        NavigationRailDestination(
          icon: Icon(LucideIcons.folderOpen),
          selectedIcon: Icon(LucideIcons.folderOpen),
          label: Text('工作区设置'),
        ),
        NavigationRailDestination(
          icon: Icon(LucideIcons.search),
          selectedIcon: Icon(LucideIcons.search),
          label: Text('搜索设置'),
        ),
        NavigationRailDestination(
          icon: Icon(LucideIcons.info),
          selectedIcon: Icon(LucideIcons.info),
          label: Text('关于'),
        ),
      ],
    );
  }
}
