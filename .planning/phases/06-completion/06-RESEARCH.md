# Phase 6: 完成与优化 - Research

**Researched:** 2026-03-03
**Domain:** Flutter 桌面应用设置与用户体验优化
**Confidence:** HIGH

## Summary

Phase 6 focuses on completing the user experience by implementing settings functionality, workspace auto-restore, and UX polishing. The Flutter project already has a settings page (but needs restructuring per requirements), splash screen, and uses SharedPreferences for persistence (as seen in workspace_provider.dart). Key implementation will involve restructuring the settings page to match the left-navigation layout with four categories, implementing theme switching with real-time effect, and adding workspace auto-restore on app startup.

**Primary recommendation:** 使用现有的 shared_preferences ^2.3.0 实现设置持久化，重构 settings_page.dart 为左侧导航布局，扩展 splash_page.dart 添加工作区恢复逻辑。

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **布局**: 左侧导航式（分类列表在左，内容在右），窗口变窄时自动折叠
- **分类**: 四个分类项 — 基础设置、工作区设置、搜索设置、关于
- **侧边栏**: 图标 + 文字标签显示
- **入口**: 固定入口（齿轮图标），位于标题栏或侧边栏
- **基础设置**: 主题切换（三选项：浅色/深色/跟随系统），实时生效
- **工作区设置**: 自动保存/恢复最近 5 个工作区
- **搜索设置**: 搜索历史记录数（默认 50 条）
- **关于页面**: 基本信息（应用名称、版本号、版权信息）
- **默认分类**: 打开设置时默认显示「基础设置」
- **存储方案**: SharedPreferences
- **保存时机**: 立即保存（设置项变化时立即保存）
- **错误处理**: 保存失败时显示 Toast 提示用户
- **初始化**: 首次启动时使用预设的默认值
- **键名命名**: 命名空间式（如 settings.theme, settings.workspace）
- **数据迁移**: 应用升级时自动迁移
- **备份恢复**: 支持导入导出 JSON 格式的设置文件
- **启动画面**: 显示简洁品牌展示（应用图标 + 名称）
- **工作区恢复**: 启动时自动加载上次的工作区
- **异常处理**: 上次工作区不存在时显示工作区列表

### Claude's Discretion
- 具体的骨架屏设计样式
- 空状态图标的风格选择
- 进度条的具体颜色和样式
- 键盘快捷键的具体绑定

### Deferred Ideas (OUT OF SCOPE)
None - discussion stayed within phase scope

</user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UI-04 | 应用程序可以正常启动 | 需要实现完整的启动流程：SplashScreen -> 工作区恢复 -> 主页面路由 |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `flutter_riverpod` | ^3.0.0 | 状态管理 | 项目已采用 Riverpod 3.0，与现有架构一致 |
| `go_router` | ^14.0.0 | 声明式路由 | 项目已采用 go_router，支持路由守卫和重定向 |
| `shared_preferences` | ^2.3.0 | 本地持久化 | 项目已包含，用于工作区最近打开时间存储 |
| `flutter_hooks` | ^0.21.0 | 函数式组件状态 | 项目已采用，配合 Riverpod 使用 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `flutter_animate` | ^4.5.0 | 过渡动画 | 实现 200-300ms 过渡效果 |
| `shimmer` | ^3.0.0 | 骨架屏占位 | 加载状态显示 |
| `hotkey_manager` | ^0.2.0 | 键盘快捷键 | 键盘导航支持 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `shared_preferences` | `hive` / `sqflite` | Hive 更适合复杂对象，但 SharedPreferences 已集成且满足需求 |
| `flutter_animate` | `flutter_staggered_animations` | flutter_animate 更轻量，API 更现代 |
| 手动实现主题切换 | `flex_color_scheme` | 手动实现可完全控制，符合现有 theme.dart 结构 |

---

## Architecture Patterns

### Recommended Project Structure

```
lib/
├── core/
│   ├── router/
│   │   └── app_router.dart          # 路由配置（已存在）
│   ├── theme/
│   │   ├── app_theme.dart           # 主题定义（已存在）
│   │   └── theme_provider.dart      # 新增：主题状态管理
│   └── constants/
│       └── app_constants.dart       # 常量定义（已存在）
├── features/
│   ├── settings/
│   │   ├── providers/
│   │   │   └── settings_provider.dart    # 新增：设置状态管理
│   │   ├── models/
│   │   │   └── settings_model.dart       # 新增：设置数据模型
│   │   └── presentation/
│   │       ├── settings_page.dart        # 重构：左侧导航布局
│   │       └── widgets/
│   │           ├── settings_sidebar.dart      # 新增：侧边栏组件
│   │           ├── basic_settings_tab.dart     # 新增：基础设置
│   │           ├── workspace_settings_tab.dart # 新增：工作区设置
│   │           ├── search_settings_tab.dart    # 新增：搜索设置
│   │           └── about_tab.dart              # 新增：关于页面
│   ├── splash/
│   │   └── splash_page.dart             # 扩展：添加工作区恢复
│   └── home/
│       └── home_page.dart               # 新增：主页面容器（侧边栏+内容）
├── shared/
│   ├── services/
│   │   └── settings_service.dart        # 新增：SharedPreferences 封装
│   └── providers/
│       ├── app_provider.dart            # 扩展：添加设置相关状态
│       └── workspace_provider.dart      # 扩展：添加工作区恢复相关
└── main.dart                            # 扩展：应用启动初始化
```

### Pattern 1: 设置页面左侧导航布局

**What:** 使用 Row 布局，左侧为 NavigationRail（可折叠），右侧为设置内容区域

**When to Use:** 需要分类导航的设置页面

**Example:**
```dart
class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedIndex = ref.watch(settingsTabProvider);
    final isExpanded = ref.watch(settingsSidebarExpandedProvider);

    return Scaffold(
      body: Row(
        children: [
          // 左侧导航 - 可折叠
          NavigationRail(
            extended: isExpanded,
            minExtendedWidth: 200,
            selectedIndex: selectedIndex,
            onDestinationSelected: (index) {
              ref.read(settingsTabProvider.notifier).state = index;
            },
            leading: IconButton(
              icon: Icon(isExpanded ? Icons.menu_open : Icons.menu),
              onPressed: () {
                ref.read(settingsSidebarExpandedProvider.notifier).state = !isExpanded;
              },
            ),
            destinations: const [
              NavigationRailDestination(
                icon: Icon(Icons.tune),
                selectedIcon: Icon(Icons.tune),
                label: Text('基础设置'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.folder_outlined),
                selectedIcon: Icon(Icons.folder),
                label: Text('工作区设置'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.search_outlined),
                selectedIcon: Icon(Icons.search),
                label: Text('搜索设置'),
              ),
              NavigationRailDestination(
                icon: Icon(Icons.info_outline),
                selectedIcon: Icon(Icons.info),
                label: Text('关于'),
              ),
            ],
          ),
          const VerticalDivider(thickness: 1, width: 1),
          // 右侧内容
          Expanded(
            child: _buildContent(selectedIndex),
          ),
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
```

### Pattern 2: SharedPreferences 命名空间式存储

**What:** 使用前缀区分不同类型的设置，便于迁移和备份

**When to Use:** 需要结构化存储多种设置

**Example:**
```dart
class SettingsService {
  static const String _prefix = 'settings.';

  // 键名常量
  static const String keyTheme = '${_prefix}theme';           // 'light' | 'dark' | 'system'
  static const String keyRecentWorkspaces = '${_prefix}recent_workspaces';  // JSON 数组
  static const String keySearchHistoryLimit = '${_prefix}search_history_limit';
  static const String keyLastWorkspaceId = '${_prefix}last_workspace_id';

  final SharedPreferences _prefs;

  SettingsService(this._prefs);

  // 主题设置
  String getTheme() => _prefs.getString(keyTheme) ?? 'system';
  Future<bool> setTheme(String value) => _prefs.setString(keyTheme, value);

  // 最近工作区（最多5个）
  List<String> getRecentWorkspaces() {
    final json = _prefs.getString(keyRecentWorkspaces);
    if (json == null) return [];
    return List<String>.from(jsonDecode(json));
  }

  Future<bool> setRecentWorkspaces(List<String> workspaces) {
    final limited = workspaces.take(5).toList();
    return _prefs.setString(keyRecentWorkspaces, jsonEncode(limited));
  }

  Future<bool> addRecentWorkspace(String id) async {
    final list = getRecentWorkspaces();
    list.remove(id); // 去除重复
    list.insert(0, id); // 添加到最前
    return setRecentWorkspaces(list.take(5).toList());
  }

  // 搜索历史限制
  int getSearchHistoryLimit() => _prefs.getInt(keySearchHistoryLimit) ?? 50;
  Future<bool> setSearchHistoryLimit(int value) => _prefs.setInt(keySearchHistoryLimit, value);

  // 最后工作区 ID（用于启动恢复）
  String? getLastWorkspaceId() => _prefs.getString(keyLastWorkspaceId);
  Future<bool> setLastWorkspaceId(String? id) {
    if (id == null) return _prefs.remove(keyLastWorkspaceId);
    return _prefs.setString(keyLastWorkspaceId, id);
  }

  // 导出设置到 JSON
  Map<String, dynamic> exportSettings() => {
    'theme': getTheme(),
    'recent_workspaces': getRecentWorkspaces(),
    'search_history_limit': getSearchHistoryLimit(),
    'exported_at': DateTime.now().toIso8601String(),
    'version': '1.0.0',
  };

  // 从 JSON 导入设置
  Future<bool> importSettings(Map<String, dynamic> data) async {
    if (data.containsKey('theme')) await setTheme(data['theme']);
    if (data.containsKey('recent_workspaces')) {
      await setRecentWorkspaces(List<String>.from(data['recent_workspaces']));
    }
    if (data.containsKey('search_history_limit')) {
      await setSearchHistoryLimit(data['search_history_limit']);
    }
    return true;
  }
}
```

### Pattern 3: 主题实时切换

**What:** 使用 Riverpod 管理主题状态，ThemeMode 实时响应

**When to Use:** 设置中切换主题需要立即生效

**Example:**
```dart
// theme_provider.dart
final themeModeProvider = StateNotifierProvider<ThemeModeNotifier, ThemeMode>((ref) {
  return ThemeModeNotifier(ref);
});

class ThemeModeNotifier extends StateNotifier<ThemeMode> {
  final Ref _ref;

  ThemeModeNotifier(this._ref) : super(ThemeMode.system) {
    _loadTheme();
  }

  Future<void> _loadTheme() async {
    final prefs = await SharedPreferences.getInstance();
    final themeValue = prefs.getString('settings.theme') ?? 'system';
    state = _themeFromString(themeValue);
  }

  ThemeMode _themeFromString(String value) {
    switch (value) {
      case 'light':
        return ThemeMode.light;
      case 'dark':
        return ThemeMode.dark;
      default:
        return ThemeMode.system;
    }
  }

  String _themeToString(ThemeMode mode) {
    switch (mode) {
      case ThemeMode.light:
        return 'light';
      case ThemeMode.dark:
        return 'dark';
      case ThemeMode.system:
        return 'system';
    }
  }

  Future<void> setTheme(ThemeMode mode) async {
    state = mode;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('settings.theme', _themeToString(mode));
  }
}

// main.dart 中使用
MaterialApp.router(
  themeMode: ref.watch(themeModeProvider), // 实时响应
  // ...
)
```

### Pattern 4: 启动流程工作区恢复

**What:** Splash 页面初始化后，检查最后工作区并自动恢复

**When to Use:** 应用启动时自动加载上次工作区

**Example:**
```dart
class SplashPage extends ConsumerStatefulWidget {
  const SplashPage({super.key});

  @override
  ConsumerState<SplashPage> createState() => _SplashPageState();
}

class _SplashPageState extends ConsumerState<SplashPage> {
  Future<void> _initialize() async {
    // 1. 初始化 FFI
    await BridgeService.instance.initialize().timeout(_timeout);

    // 2. 加载配置（主题等）
    ref.read(appStateProvider.notifier).loadConfig();

    // 3. 尝试恢复最后工作区
    final workspaceState = ref.read(workspaceStateProvider.notifier);
    final prefs = await SharedPreferences.getInstance();
    final lastWorkspaceId = prefs.getString('settings.last_workspace_id');

    if (lastWorkspaceId != null) {
      // 检查工作区是否存在
      final workspaces = ref.read(workspaceStateProvider);
      final exists = workspaces.any((w) => w.id == lastWorkspaceId);

      if (exists) {
        // 自动恢复工作区
        final success = await workspaceState.loadWorkspaceById(lastWorkspaceId);
        if (success && mounted) {
          // 恢复成功，跳转到搜索页面
          context.go('/search');
          return;
        }
      }
    }

    // 4. 无法恢复，跳转到工作区列表
    if (mounted) {
      context.go('/workspaces');
    }
  }
}
```

### Anti-Patterns to Avoid

- **在 build() 方法中直接调用异步方法**: 应使用 `Future.microtask()` 或在 initState 中调用
- **手动实现设置存储而不使用 SharedPreferences**: 项目已集成 shared_preferences，应统一使用
- **不处理 SharedPreferences 初始化失败**: 必须使用 try-catch 处理可能的异常
- **设置变化时不立即保存**: CONTEXT.md 要求立即保存，应使用 `onChanged` 回调实时保存

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 本地持久化 | 自定义 JSON 文件存储 | SharedPreferences ^2.3.0 | 项目已集成，多平台支持完善 |
| 路由守卫 | 手写路由状态检查 | go_router redirect | go_router 内置支持，更清晰 |
| 主题切换 | 重新构建整个 MaterialApp | ThemeMode + Riverpod | Flutter 官方推荐，性能更好 |
| 骨架屏 | 手动实现占位符 | shimmer ^3.0.0 | 成熟方案，视觉效果好 |
| 动画 | 使用 AnimationController | flutter_animate ^4.5.0 | 声明式 API，更易用 |

---

## Common Pitfalls

### Pitfall 1: SharedPreferences 异步初始化
**What goes wrong:** 在同步上下文中调用 await SharedPreferences.getInstance() 导致阻塞
**Why it happens:** SharedPreferences.getInstance() 是异步的，但在 build() 中被同步调用
**How to avoid:** 使用 `Future.microtask()` 延迟初始化，或创建全局单例
**Warning signs:** Widget 渲染时出现 Future 警告

### Pitfall 2: 主题切换后状态不更新
**What goes wrong:** 主题已保存但 UI 未响应
**Why it happens:** 未正确监听 ThemeMode 状态变化
**How to avoid:** 使用 Riverpod provider 监听主题状态变化，在 MaterialApp 中使用 watch
**Warning signs:** 主题设置保存成功但界面未变化

### Pitfall 3: 工作区恢复时工作区已删除
**What goes wrong:** 最后工作区 ID 存在但对应工作区已被删除，导致启动失败
**Why it happens:** 未验证工作区是否存在就尝试加载
**How to avoid:** 在恢复前检查工作区列表，失败则跳转工作区列表
**Warning signs:** 启动时出现错误但无提示

### Pitfall 4: 设置导入导出 JSON 格式不兼容
**What goes wrong:** 版本升级后导入旧版本设置导致崩溃
**Why it happens:** 未处理版本字段和字段变更
**How to avoid:** 在导入时检查 version 字段，必要时进行数据迁移
**Warning signs:** 导入设置后应用崩溃

---

## Code Examples

### 主题切换 SegmentedButton 实现

```dart
// Source: Material 3 官方设计
SegmentedButton<String>(
  segments: const [
    ButtonSegment<String>(
      value: 'light',
      icon: Icon(Icons.light_mode),
      label: Text('浅色'),
    ),
    ButtonSegment<String>(
      value: 'dark',
      icon: Icon(Icons.dark_mode),
      label: Text('深色'),
    ),
    ButtonSegment<String>(
      value: 'system',
      icon: Icon(Icons.settings_brightness),
      label: Text('跟随系统'),
    ),
  ],
  selected: {currentTheme},
  onSelectionChanged: (Set<String> selection) {
    final newTheme = selection.first;
    ref.read(themeModeProvider.notifier).setTheme(_themeFromString(newTheme));
  },
)
```

### 空状态组件

```dart
class EmptyStateWidget extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? description;
  final String? actionLabel;
  final VoidCallback? onAction;

  const EmptyStateWidget({
    super.key,
    required this.icon,
    required this.title,
    this.description,
    this.actionLabel,
    this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 64, color: AppColors.textMuted),
          const SizedBox(height: 16),
          Text(
            title,
            style: const TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.w600,
              color: AppColors.textPrimary,
            ),
          ),
          if (description != null) ...[
            const SizedBox(height: 8),
            Text(
              description!,
              style: const TextStyle(color: AppColors.textSecondary),
              textAlign: TextAlign.center,
            ),
          ],
          if (actionLabel != null && onAction != null) ...[
            const SizedBox(height: 24),
            ElevatedButton(
              onPressed: onAction,
              child: Text(actionLabel!),
            ),
          ],
        ],
      ),
    );
  }
}
```

### 骨架屏加载占位

```dart
// 使用 shimmer 包
Shimmer.fromColors(
  baseColor: AppColors.bgCard,
  highlightColor: AppColors.bgHover,
  child: ListView.builder(
    itemCount: 5,
    itemBuilder: (context, index) {
      return ListTile(
        leading: CircleAvatar(
          backgroundColor: Colors.white,
          radius: 20,
        ),
        title: Container(
          height: 16,
          color: Colors.white,
          width: double.infinity,
        ),
        subtitle: Container(
          height: 12,
          color: Colors.white,
          width: 100,
        ),
      );
    },
  ),
)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Provider 2.x | Riverpod 3.0 | 项目迁移时 | 更现代的 API，更好的性能 |
| Provider 路由 | go_router | 项目迁移时 | 声明式路由，更好的类型安全 |
| 手动 JSON 存储 | SharedPreferences | Phase 2 | 统一持久化方案 |
| 无动画 | flutter_animate | 新增 | 平滑过渡体验 |

**Deprecated/outdated:**
- **Provider 2.x**: 项目已迁移到 Riverpod 3.0
- **React 前端**: 项目已从 React 迁移到 Flutter

---

## Open Questions

1. **骨架屏设计细节**
   - What we know: 需要在加载状态显示占位内容
   - What's unclear: 具体的颜色和动画效果
   - Recommendation: 使用 shimmer 包的标准实现，参考 Material 3 设计指南

2. **空状态图标风格**
   - What we know: 需要覆盖所有空状态场景
   - What's unclear: 图标风格选择（线性/填充）
   - Recommendation: 项目已使用 lucide_icons_flutter，保持一致

3. **键盘快捷键绑定**
   - What we know: 需要支持 Tab/Enter 导航
   - What's unclear: 具体快捷键配置
   - Recommendation: 使用 hotkey_manager，默认支持系统快捷键

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (内置) |
| Config file | flutter_test 配置在 pubspec.yaml |
| Quick run command | `flutter test test/settings_test.dart -x` |
| Full suite command | `flutter test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|---------------|
| UI-04 | 应用正常启动 | integration | `flutter test test/splash_test.dart` | 需新建 |
| UI-04 | 工作区自动恢复 | integration | `flutter test test/workspace_restore_test.dart` | 需新建 |
| UI-04 | 设置持久化 | unit | `flutter test test/settings_service_test.dart` | 需新建 |

### Sampling Rate
- **Per task commit:** `flutter test test/settings_test.dart -x`
- **Per wave merge:** `flutter test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/settings_service_test.dart` - 设置服务单元测试
- [ ] `test/settings_provider_test.dart` - 设置状态管理测试
- [ ] `test/splash_restore_test.dart` - 启动恢复集成测试
- [ ] `test/theme_switch_test.dart` - 主题切换测试

---

## Sources

### Primary (HIGH confidence)
- Flutter 官方文档 - Settings 最佳实践
- go_router 官方文档 - 路由配置
- Riverpod 官方文档 - 状态管理
- shared_preferences 包文档

### Secondary (MEDIUM confidence)
- Material Design 3 主题指南
- Flutter 动画最佳实践

### Tertiary (LOW confidence)
- 社区 Flutter 设置页面实现模式

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - 项目已有 shared_preferences、go_router、riverpod
- Architecture: HIGH - Flutter 官方推荐模式，与项目现有架构一致
- Pitfalls: HIGH - 基于 Flutter 常见问题和项目代码分析

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (30 days for stable domain)
