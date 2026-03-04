# Stack Research

**Domain:** Flutter Desktop Application (Log Analyzer)
**Researched:** 2026-03-04
**Confidence:** HIGH

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Flutter SDK | >=3.8.0 <4.0.0 | 桌面 UI 框架 | 官方支持 Windows/macOS/Linux 桌面，成熟稳定 |
| Riverpod | 3.0.0 | 状态管理 | 2026 年推荐方案，编译时安全，样板代码最少，异步支持优秀 |
| flutter_rust_bridge | 2.x | FFI 桥接 | 项目已采用，生成类型安全绑定，支持异步 |
| Dio | 5.4.0 | HTTP 客户端 | 拦截器、自动 JSON 解码、请求取消、进度监控 |
| go_router | 14.0.0 | 声明式路由 | 官方推荐，深度链接支持，类型安全 |
| freezed | 3.2.3 | 数据类生成 | 不可变数据类，withCopyWith，equals 自动实现 |
| json_serializable | 6.11.2 | JSON 序列化 | 与 freezed 配合，自动生成序列化代码 |

### State Management Deep Dive

| Library | Version | Use Case | Why |
|---------|---------|----------|-----|
| flutter_riverpod | ^3.0.0 | 主状态管理 | 编译时安全，内置离线持久化，样板代码最少 |
| riverpod_annotation | ^3.0.0 | 代码生成 | @riverpod 注解自动生成 Provider |
| hooks_riverpod | ^3.0.0 | React Hooks 风格 | 适合从 React 迁移的开发者 |
| riverpod_lint | ^3.0.0 | Lint 规则 | 静态分析，捕获常见错误 |

**Why Riverpod 3.0 over BLoC (2026):**
- 更少的样板代码 (BLoC 需要 Event/State/Bloc 三个类)
- 编译时安全 (BLoC 依赖运行时反射)
- 内置异步支持 (FutureProvider, StreamProvider)
- Flutter 官方推荐

### FFI Bridge Options

| Option | Version | Pros | Cons |
|--------|---------|------|------|
| **flutter_rust_bridge** | 2.x | 类型安全，自动生成，异步支持 | 需要代码生成 |
| **dart:ffi** | 内置 | 无依赖，完全控制 | 手动内存管理，易出错 |
| **uniffi** | 最新 | 多平台支持 | Flutter 生态不如 frb 成熟 |

**Recommendation:** 继续使用 `flutter_rust_bridge` (已有项目基础)

### HTTP Client Options

| Option | Pros | Cons |
|--------|------|------|
| **Dio** | 拦截器、缓存、进度、错误处理完善 | 包体积稍大 |
| **http** | 轻量、内置 | 功能有限 |
| **dio** | 桌面端支持良好 | — |

**Recommendation:** 继续使用 `dio` (已有项目基础)

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| fl_chart | ^0.70.0 | 图表渲染 | 性能监控页面 |
| file_picker | ^8.0.0 | 文件选择 | 工作区管理 |
| sentry_flutter | ^8.0.0 | 错误追踪 | 生产环境监控 |
| uuid | ^4.0.0 | UUID 生成 | 任务 ID |
| collection | ^1.18.0 | 集合工具 | 高级 List/Map 操作 |
| flutter_hooks | ^0.21.0 | Hooks 模式 | 适合 React 背景开发者 |
| flutter_virtual_scroll | ^0.1.0 | 虚拟滚动 | 大量日志数据渲染 |

### UI/Icons

| Library | Version | Purpose |
|---------|---------|---------|
| lucide_icons_flutter | ^1.0.0 | 与 React 版本一致的图标 |

### Code Generation

| Tool | Version | Purpose |
|------|---------|---------|
| build_runner | ^2.4.0 | 代码生成运行器 |
| freezed | ^3.2.3 | 不可变数据类 |
| json_serializable | ^6.11.2 | JSON 序列化 |
| riverpod_generator | ^3.0.0 | Provider 生成 |

## v1.1 高级搜索与虚拟文件系统 (新增)

### 新增库推荐

| 库 | 版本 | 用途 | 为什么推荐 |
|---|------|------|-----------|
| flutter_fancy_tree_view2 | ^1.6.3 | 虚拟文件树 UI | 基于 Sliver，性能好，支持大量数据懒加载 |
| rich_text_controller | ^3.0.1 | 正则表达式语法高亮 | 支持自定义正则模式的内联样式 |
| Dart RegExp | 内置 | 正则表达式验证 | Dart 标准库，无需额外包 |

### 已实现的后端功能 (无需开发)

| 功能 | Rust 模块 | 状态 |
|-----|----------|------|
| 正则搜索 | `search_engine/advanced_features.rs` | ✅ FilterEngine 带编译缓存 |
| 布尔 AND/OR/NOT | `search_engine/boolean_query_processor.rs` | ✅ Tantivy BooleanQuery |
| 搜索历史 | `commands/search_history.rs` | ✅ add/get/clear 命令 |
| 多关键词 | `services/pattern_matcher.rs` | ✅ Aho-Corasick + QueryOperator |

### 需添加的 FFI 命令

现有 Rust 后端已完整实现 v1.1 所需功能，只需通过 FFI 暴露给 Flutter：

```rust
// commands/search_history.rs - 已实现
add_search_history(workspace_id, query, timestamp)
get_search_history(workspace_id, limit?)
clear_search_history(workspace_id?)

// commands/search.rs - 已支持
// SearchQuery 已支持:
// - terms: Vec<SearchTerm> 带 operator (AND/OR/NOT)
// - is_regex: bool 每个 term
// - global_operator: QueryOperator
```

### Flutter 端需开发

1. **TreeView 状态管理** - FileTreeNotifier (Riverpod)
2. **搜索查询构建器** - SearchQueryNotifier (Riverpod)
3. **搜索历史集成** - 连接现有 FFI 命令

## Installation

```bash
# 进入 Flutter 项目目录
cd log-analyzer_flutter

# v1.1 新增依赖
flutter pub add flutter_fancy_tree_view2:^1.6.3
flutter pub add rich_text_controller:^3.0.1

# 安装依赖
flutter pub get

# 代码生成 (首次设置)
dart run build_runner build

# 或增量生成
dart run build_runner build --delete-conflicting-outputs
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Riverpod 3.0 | BLoC | 需要严格审计追踪 (金融/医疗)，团队已有 BLoC 经验 |
| flutter_rust_bridge | dart:ffi | 极致性能需求，愿意手写内存管理 |
| Dio | http 包 | 极简场景，只需 GET/POST 且不需要拦截器 |
| go_router | auto_route | 需要更高级的路由动画，复杂嵌套路由 |
| freezed | built_value | 需要更严格的序列化控制 |
| flutter_fancy_tree_view2 | animated_tree_view | 需要展开/收起动画效果 |
| flutter_fancy_tree_view2 | flutter_simple_treeview | 需要更简单 API，更小包体积 |
| Dart RegExp | regex 包 | 需要 Rust 级别正则 (后端已有) |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Provider (1.x/2.x) | Riverpod 3.0 是官方推荐升级方向 | flutter_rust_bridge ^3.0.0 |
| GetX | 状态管理与路由耦合 | go_router + Riverpod |
| BLoC (除非企业需求) | 样板代码多，无编译时安全 | Riverpod 3.0 |
| setState | 无法管理复杂状态 | Riverpod |
| any 作为类型 | 类型不安全 | 具体类型 + freezed |
| recursive_regex | 针对 Flutter web/mobile 边缘情况 | Dart 内置 RegExp 足够 |
| regexpattern | 预构建验证模式，过度 | Dart RegExp 自定义模式 |
| animated_tree_view | 文件树动画开销 | flutter_fancy_tree_view2 (无动画，基于 Sliver) |
| json_view | JSON 专用树视图 | flutter_fancy_tree_view2 (通用，可定制) |

## Architecture Patterns

### Clean Architecture (Recommended)

```
lib/
├── core/                    # 核心共享
│   ├── constants/           # 常量定义
│   ├── theme/              # 主题配置
│   └── utils/              # 工具函数
├── features/                # 功能模块 (按业务划分)
│   ├── search/             # 搜索功能
│   │   ├── data/          # 数据层 (repositories, data sources)
│   │   ├── domain/        # 领域层 (entities, use cases)
│   │   └── presentation/  # 表现层 (pages, widgets, providers)
│   │       ├── providers/ # Riverpod providers
│   │       ├── widgets/   # 搜索组件 (SearchBar, QueryBuilder, HistoryDropdown)
│   │       └── pages/     # 搜索页面
│   ├── file_tree/          # 虚拟文件系统 (v1.1 新增)
│   │   ├── data/
│   │   ├── domain/
│   │   └── presentation/
│   │       ├── providers/ # FileTreeNotifier
│   │       └── widgets/   # FileTreeView
│   ├── workspace/          # 工作区管理
│   ├── archive/           # 压缩包处理
│   └── settings/          # 设置
├── shared/                  # 共享组件
│   ├── models/            # 共享数据模型
│   ├── providers/         # 共享 Providers
│   ├── services/          # API 服务
│   └── widgets/           # 共享 Widgets
└── main.dart
```

### Communication with Rust Backend

**FFI 优先模式:**
```
Flutter (Dart)  <--frb-->  Rust (lib)  <--->  SQLite/Tantivy
```

**HTTP API 备选模式:**
```
Flutter (Dart)  <--HTTP-->  Rust (axum)  <--->  SQLite/Tantivy
```

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| flutter_rust_bridge ^2.0.0 | Rust backend 2.x | 已有项目基础 |
| riverpod ^3.0.0 | Dart SDK >=3.0.0 | 需要 Dart 3.0+ |
| dio ^5.4.0 | Flutter 3.x | 桌面端全支持 |
| go_router ^14.0.0 | Flutter 3.x | 深度链接支持 |
| freezed ^3.2.3 | Dart SDK >=3.0.0 | 需要代码生成 |
| fl_chart ^0.70.0 | Flutter 3.x | 图表渲染 |
| flutter_fancy_tree_view2 ^1.6.3 | Flutter 3.8+ | 使用 Sliver，需 >=3.7 |
| rich_text_controller ^3.0.1 | Flutter 3.8+ | 文本编辑控制器 |

## Sources

- [Flutter 官方架构建议](https://docs.flutter.dev/app-architecture/recommendations) — 官方推荐的架构模式
- [Riverpod 3.0](https://pub.dev/packages/flutter_rust_bridge) — 状态管理推荐
- [flutter_rust_bridge](https://pub.dev/packages/flutter_rust_bridge) — FFI 桥接
- [Dio HTTP](https://pub.dev/packages/dio) — HTTP 客户端
- [Riverpod vs BLoC 2026](https://flutterstudio.dev/blog/bloc-vs-riverpod-flutter-state-management-2026.html) — 状态管理对比
- [Flutter Clean Architecture](https://medium.com/@flutter-app/clean-architecture-in-flutter-full-guide-with-examples-d647a9a4fe52) — 架构模式
- [pub.dev flutter_fancy_tree_view2](https://pub.dev/packages/flutter_fancy_tree_view2) — TreeView 组件
- [pub.dev rich_text_controller](https://pub.dev/packages/rich_text_controller) — 正则高亮
- Rust `commands/search_history.rs` — 搜索历史命令实现
- Rust `search_engine/boolean_query_processor.rs` — 布尔查询实现
- Rust `search_engine/advanced_features.rs` — 正则过滤器引擎

---

*Stack research for: Flutter Desktop Log Analyzer v1.1*
*Researched: 2026-03-04*
