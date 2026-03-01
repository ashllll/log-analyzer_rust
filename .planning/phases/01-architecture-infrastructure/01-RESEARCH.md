# Phase 1: 架构基础设施 - Research

**Researched:** 2026-03-01
**Domain:** Flutter FFI + Rust 后端通信基础设施
**Confidence:** HIGH

## Summary

本阶段目标是建立 Flutter 前端与 Rust 后端的通信基础设施。当前项目已有部分基础设施（ApiService、BridgeService、FFI 生成代码），但需要重构为纯 FFI 模式并完善错误处理机制。

**Primary recommendation:** 使用 flutter_rust_bridge 2.x 实现纯 FFI 通信，配合 Riverpod 3.0 的 AsyncNotifier 进行异步状态管理，通过分段错误码（0-999 通用/1000+ 模块特定）实现统一的错误处理。

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

1. **通信模式 (FFI)**
   - 仅使用 FFI 与 Rust 后端通信（不使用 HTTP）
   - 延迟加载 - 首次调用时初始化 FFI
   - 库路径：标准路径加载（Windows: exe 同目录, macOS: Contents/MacOS）
   - 纯 FFI 模式，不保留 HTTP 备选
   - 失败处理：显示错误页面，提供重试按钮

2. **错误处理**
   - 错误码分段设计（0-999 通用错误, 1000+ 模块特定错误）
   - 统一错误页面，显示错误码和解决方案
   - 使用 Sentry 上报错误（Flutter 端已集成）
   - 连接错误：显示"后端未连接"错误页面，提供启动后端指引

3. **Provider 架构**
   - 文件组织：单文件模块（workspace_provider.dart, search_provider.dart 等）
   - 状态类：使用 Freezed 生成不可变状态类
   - 异步模式：使用 AsyncNotifier 管理异步状态
   - 依赖注入：Provider 构造函数注入依赖

4. **启动流程**
   - Splash：显示应用 logo + "正在连接后端..." 文字
   - 检测内容：检测 FFI 库是否可加载
   - 超时时间：10秒
   - 失败处理：显示错误页面，有"重试"按钮

### Claude's Discretion

- FFI 方法通道的具体命名规范
- 错误码的具体分段数值范围
- Splash 界面的具体设计和动画
- Provider 文件的具体拆分粒度

### Deferred Ideas (OUT OF SCOPE)

- HTTP 调试模式 - Phase 1 后如有需要可恢复
- 离线模式支持 - 当前不需要，后端必须可用

---

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| UI-04 | 应用程序可以正常启动 | FFI 初始化 + 启动流程 + 错误页面 |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| flutter_rust_bridge | 2.x | Flutter Rust FFI 桥接 | 官方推荐的 Flutter-Rust 绑定方案，自动生成类型安全代码 |
| flutter_riverpod | 3.0 | 状态管理 | Riverpod 3.0 是 Flutter 官方推荐的状态管理方案 |
| riverpod_annotation | 3.0 | Riverpod 代码生成 | 配合 build_runner 自动生成 Provider |
| freezed_annotation | 3.0 | 不可变状态类 | 生成不可变数据类、copyWith、toJson |
| go_router | 14.x | 路由管理 | Flutter 官方推荐路由方案 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| sentry_flutter | 8.0 | 错误追踪 | 生产环境错误监控 |
| file_picker | 8.0 | 文件选择 | 导入文件夹/文件 |
| fl_chart | 0.70.x | 图表渲染 | 性能监控页面 |
| lucide_icons_flutter | 1.0.x | 图标库 | UI 图标 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| flutter_rust_bridge | 手动 FFI (dart:ffi) | 需要手写类型转换，复杂度高，错误率高 |
| Riverpod 3.0 | Provider / Bloc | Riverpod 3.0 异步支持更好，代码更简洁 |
| go_router | Navigator 2.0 | go_router 提供声明式路由，更易维护 |
| Freezed | manual immutable classes | Freezed 自动生成代码，减少样板 |

**Installation (移除 Dio):**
```bash
# 在 pubspec.yaml 中移除
# dio: ^5.4.0

# 保留的核心依赖
flutter_rust_bridge: ^2.0.0
flutter_riverpod: ^3.0.0
riverpod_annotation: ^3.0.0
freezed_annotation: ^3.0.0
```

---

## Architecture Patterns

### Recommended Project Structure

```
lib/
├── core/
│   ├── constants/           # 常量定义
│   ├── router/              # go_router 配置
│   ├── sentry/              # Sentry 配置
│   └── theme/               # 主题配置
├── features/
│   ├── search/
│   │   ├── domain/         # 领域模型
│   │   └── presentation/   # 页面/组件
│   ├── workspace/
│   ├── keyword/
│   ├── task/
│   └── settings/
├── shared/
│   ├── models/             # 数据模型 (Freezed 生成)
│   ├── providers/          # Riverpod Providers
│   ├── services/           # 服务层
│   │   ├── ffi/            # FFI 桥接
│   │   ├── api_service.dart # API 服务
│   │   └── error_handler.dart # 错误处理
│   └── widgets/            # 共享组件
└── l10n/                  # 国际化
```

### Pattern 1: FFI Bridge Service (延迟加载)

```dart
// lib/shared/services/ffi_bridge_service.dart
import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'generated/frb_generated.dart';

class FfiBridgeService {
  static FfiBridgeService? _instance;
  static bool _isInitialized = false;
  static bool _initFailed = false;

  FfiBridgeService._();

  static FfiBridgeService get instance {
    _instance ??= FfiBridgeService._();
    return _instance!;
  }

  /// 延迟初始化 FFI
  /// 首次调用时触发初始化
  Future<void> initialize() async {
    if (_isInitialized) return;
    if (_initFailed) throw FfiInitializationException('FFI 初始化已失败');

    try {
      await LogAnalyzerBridge.init();
      _isInitialized = true;
      debugPrint('FFI Bridge 初始化成功');
    } catch (e) {
      _initFailed = true;
      debugPrint('FFI Bridge 初始化失败: $e');
      rethrow;
    }
  }

  bool get isInitialized => _isInitialized;
}

/// FFI 初始化异常
class FfiInitializationException implements Exception {
  final String message;
  FfiInitializationException(this.message);
}
```

### Pattern 2: AsyncNotifier for Async State

```dart
// lib/shared/providers/workspace_provider.dart
import 'package:riverpod_annotation/riverpod_annotation.dart';
import '../models/workspace.dart';
import '../services/api_service.dart';

part 'workspace_provider.g.dart';

@riverpod
class Workspace extends _$Workspace {
  @override
  Future<List<WorkspaceData>> build() async {
    final api = ref.watch(apiServiceProvider);
    return api.getWorkspaces();
  }

  Future<void> refresh() async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() => ref.read(apiServiceProvider).getWorkspaces());
  }
}
```

### Pattern 3: Error Code Classification

```dart
// lib/shared/services/error_handler.dart

/// 错误码分类
class ErrorCodes {
  // 通用错误 (0-999)
  static const int unknown = 0;
  static const int networkError = 1;
  static const int timeout = 2;
  static const int invalidParams = 3;
  static const int notFound = 4;
  static const int unauthorized = 5;
  static const int ffiNotInitialized = 10;
  static const int ffiLoadFailed = 11;

  // 模块特定错误 (1000+)
  // 搜索模块 (1000-1099)
  static const int searchFailed = 1000;
  static const int searchCancelled = 1001;
  static const int searchTimeout = 1002;

  // 工作区模块 (1100-1199)
  static const int workspaceNotFound = 1100;
  static const int workspaceCreateFailed = 1101;
  static const int workspaceDeleteFailed = 1102;

  // 导入模块 (1200-1299)
  static const int importFailed = 1200;
  static const int importCancelled = 1201;

  // 文件监听模块 (1300-1399)
  static const int watchFailed = 1300;
}

/// 应用异常
class AppException implements Exception {
  final int code;
  final String message;
  final String? help;
  final dynamic originalError;

  const AppException({
    required this.code,
    required this.message,
    this.help,
    this.originalError,
  });

  String get displayMessage => '[E$code] $message';

  String? get solution => _getSolution(code);

  static String? _getSolution(int code) {
    switch (code) {
      case ErrorCodes.ffiNotInitialized:
        return '请重启应用程序';
      case ErrorCodes.ffiLoadFailed:
        return '请确保 Rust 后端已正确安装';
      case ErrorCodes.networkError:
        return '请检查网络连接';
      case ErrorCodes.timeout:
        return '操作超时，请重试';
      default:
        return null;
    }
  }
}
```

### Pattern 4: Error Page with Retry

```dart
// lib/shared/widgets/error_view.dart
class ErrorView extends StatelessWidget {
  final AppException exception;
  final VoidCallback? onRetry;
  final VoidCallback? onReport;

  const ErrorView({
    super.key,
    required this.exception,
    this.onRetry,
    this.onReport,
  });

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.error_outline, size: 64, color: Colors.red[400]),
          const SizedBox(height: 16),
          Text(exception.displayMessage, style: Theme.of(context).textTheme.titleMedium),
          if (exception.solution != null) ...[
            const SizedBox(height: 8),
            Text(exception.solution!, style: Theme.of(context).textTheme.bodyMedium),
          ],
          const SizedBox(height: 24),
          if (onRetry != null)
            ElevatedButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: const Text('重试'),
            ),
        ],
      ),
    );
  }
}
```

### Pattern 5: Splash Screen with FFI Check

```dart
// lib/features/splash/splash_page.dart
class SplashPage extends ConsumerStatefulWidget {
  const SplashPage({super.key});

  @override
  ConsumerState<SplashPage> createState() => _SplashPageState();
}

class _SplashPageState extends ConsumerState<SplashPage> {
  static const _timeout = Duration(seconds: 10);
  String _status = '正在连接后端...';
  AppException? _error;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    try {
      await ref.read(ffiBridgeProvider).initialize().timeout(_timeout);
      if (mounted) {
        context.go('/home');
      }
    } on TimeoutException {
      setState(() {
        _status = '连接超时';
        _error = const AppException(
          code: ErrorCodes.timeout,
          message: '后端连接超时',
          help: '请检查 Rust 后端是否正在运行',
        );
      });
    } catch (e) {
      setState(() {
        _status = '连接失败';
        _error = AppException(
          code: ErrorCodes.ffiLoadFailed,
          message: '无法加载 Rust 后端',
          originalError: e,
        );
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: _error != null
          ? ErrorView(
              exception: _error!,
              onRetry: () {
                setState(() {
                  _error = null;
                  _status = '正在连接后端...';
                });
                _initialize();
              },
            )
          : Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const Icon(Icons.analytics, size: 80),
                  const SizedBox(height: 24),
                  Text('Log Analyzer', style: Theme.of(context).textTheme.headlineMedium),
                  const SizedBox(height: 16),
                  const CircularProgressIndicator(),
                  const SizedBox(height: 16),
                  Text(_status),
                ],
              ),
            ),
    );
  }
}
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Flutter-Rust 通信 | 手动 FFI 调用 | flutter_rust_bridge | 自动生成类型安全代码，处理内存管理 |
| 状态管理 | Provider/手动状态 | Riverpod 3.0 AsyncNotifier | 更好的异步支持，自动Dispose |
| 不可变类 | 手写 immutable | Freezed | 自动生成 copyWith, toJson, fromJson |
| 路由导航 | Navigator 1.0 | go_router | 声明式路由，深度链接支持 |
| 错误追踪 | print/日志 | Sentry | 生产环境错误收集和分析 |

---

## Common Pitfalls

### Pitfall 1: FFI 库加载路径问题
**What goes wrong:** 在 Windows 上 FFI 库无法加载，提示找不到 .dll 文件
**Why it happens:** 动态库路径不正确，未将库文件复制到正确位置
**How to avoid:**
- 使用 `DynamicLibrary.open()` 指定完整路径
- Windows: 与 exe 同目录
- macOS: Contents/MacOS 目录
- 配置 tauri.conf.json 的 `bundle.resources`
**Warning signs:** `FfiInitializationException: Failed to load dynamic library`

### Pitfall 2: FFI 初始化时序问题
**What goes wrong:** 在 `main()` 中同步调用 FFI 初始化导致闪退
**Why it happens:** FFI 需要在 Flutter 绑定初始化完成后调用
**How to avoid:**
- 使用 `await LogAnalyzerBridge.init()` 在 main 中异步初始化
- 或使用延迟初始化，在首次调用时初始化
**Warning signs:** `Invalid kernel binary format` 或空指针异常

### Pitfall 3: Riverpod 状态泄漏
**What goes wrong:** Provider 中的资源未正确释放，导致内存泄漏
**Why it happens:** 未使用 `ref.onDispose()` 清理资源，或在 build 外持有 ref
**How to avoid:**
- 使用 `ref.onDispose(() => resource.dispose())`
- 避免在 Provider 中存储 context
**Warning signs:** 内存持续增长，组件重建时状态不更新

### Pitfall 4: 错误码冲突
**What goes wrong:** 前后端错误码定义不一致，导致错误处理逻辑错误
**Why it happens:** 未建立统一的错误码规范，各自定义
**How to avoid:**
- 在 CONTEXT.md 中明确定义错误码分段
- 后端 Rust 代码使用相同错误码常量
**Warning signs:** 错误页面显示不正确的解决方案

---

## Code Examples

### Initialize FFI in main.dart

```dart
// Source: flutter_rust_bridge 官方文档
void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // 初始化 Rust FFI
  await LogAnalyzerBridge.init();

  runApp(
    const ProviderScope(
      child: LogAnalyzerApp(),
    ),
  );
}
```

### AsyncNotifier Pattern

```dart
// Source: Riverpod 官方文档
@riverpod
class SearchNotifier extends _$SearchNotifier {
  @override
  Future<SearchResult> build() async {
    return _performSearch();
  }

  Future<SearchResult> _performSearch() async {
    final api = ref.read(apiServiceProvider);
    return api.searchLogs(query: ref.read(searchQueryProvider));
  }

  Future<void> search(String query) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() => _performSearchWithQuery(query));
  }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| HTTP API 调用 | FFI 直接调用 | CONTEXT.md 决策 | 减少网络开销，更低延迟 |
| Provider 2.x | Riverpod 3.0 AsyncNotifier | 2024+ | 更简洁的异步状态管理 |
| 手动状态类 | Freezed 自动生成 | 2022+ | 减少样板代码，更安全 |
| Navigator 1.0 | go_router 14+ | 2023+ | 声明式路由，更易维护 |

**Deprecated/outdated:**
- `dio` HTTP 客户端 - 纯 FFI 模式下不再需要
- `Provider` 组件 - 已迁移到 Riverpod 3.0

---

## Open Questions

1. **FFI 库路径加载策略**
   - What we know: Windows 需与 exe 同目录，macOS 需 Contents/MacOS
   - What's unclear: Linux 上的标准路径和 fallback 策略
   - Recommendation: 参考 Tauri 2.0 官方文档确定路径

2. **错误码具体分段数值**
   - What we know: 0-999 通用，1000+ 模块特定
   - What's unclear: 每个模块的具体范围划分
   - Recommendation: 规划阶段确定，与后端 Rust 代码对齐

3. **Splash 界面设计**
   - What we know: 需要显示 logo + 文字 + 超时处理
   - What's unclear: 具体动画效果，过渡动画时长
   - Recommendation: 参考 Material Design 3 启动页规范

---

## Sources

### Primary (HIGH confidence)
- [flutter_rust_bridge GitHub](https://github.com/fzyzcjy/flutter_rust_bridge) - 官方 FFI 库
- [Riverpod 文档](https://docs-v2.riverpod.dev/docs/providers/notifier_provider) - AsyncNotifier 官方文档
- [Freezed Pub](https://pub.dev/packages/freezed) - 不可变类生成库
- [go_router Pub](https://pub.dev/packages/go_router) - 路由库

### Secondary (MEDIUM confidence)
- [Flutter Riverpod 2025 Complete Guide](https://medium.com/@alokkumarmaurya5556/master-riverpod-in-flutter-2025-a-complete-beginner-friendly-deep-practical-state-management-57536279483f) - Riverpod 最佳实践
- [Error Handling in Riverpod](https://tillitsdone.com/blogs/error-handling-in-riverpod-guide/) - 错误处理模式

### Tertiary (LOW confidence)
- [DhiWise Flutter Rust Bridge Guide](https://www.dhiwise.com/post/enhancing-flutter-apps-with-the-flutter-rust-bridge-package) - FFI 集成教程

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH - 官方推荐方案，已在项目中使用
- Architecture: HIGH - 锁定决策，清晰的方向
- Pitfalls: MEDIUM - 基于社区经验的常见问题

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (30天，Flutter/Rust 生态相对稳定)
