# Log Analyzer Flutter

从 React + Tauri 迁移到 Flutter + Rust FFI 的日志分析工具。

## 项目状态

**当前阶段**: 阶段 0 完成 - 项目结构初始化

### 已完成

- [x] Flutter 项目目录结构
- [x] 依赖配置 (pubspec.yaml)
- [x] 核心基础设施层
  - [x] 应用入口 (main.dart)
  - [x] 路由配置 (app_router.dart)
  - [x] 主题配置 (app_theme.dart) - 与 React 版本 Tailwind 配色一致
  - [x] 常量定义 (app_constants.dart)
- [x] 数据模型层 (shared/models/)
  - [x] 通用模型 (common.dart) - LogEntry, Workspace, TaskProgress 等
  - [x] 搜索模型 (search.dart) - SearchQuery, SearchTerm 等
  - [x] 关键词模型 (keyword.dart) - KeywordGroup 等
  - [x] 应用状态模型 (app_state.dart)
- [x] 状态管理层 (shared/providers/)
  - [x] AppState (应用全局状态)
  - [x] WorkspaceState (工作区状态)
  - [x] TaskState (任务状态 - 包含幂等性检查)
  - [x] KeywordState (关键词状态)
- [x] 服务层 (shared/services/)
  - [x] ApiService (API 服务 - 框架代码，待实现 FFI)
  - [x] EventStreamService (事件流服务)
  - [x] EventBus (事件总线 - 包含幂等性保证)
  - [x] SearchQueryBuilder (搜索查询构建器)
- [x] 国际化 (l10n/)
  - [x] app_en.arb (英文)
  - [x] app_zh.arb (中文)
- [x] 页面占位 (features/*/presentation/)
  - [x] SearchPage (搜索页面)
  - [x] WorkspacesPage (工作区页面)
  - [x] KeywordsPage (关键词页面)
  - [x] TasksPage (任务页面)
  - [x] SettingsPage (设置页面)
  - [x] PerformancePage (性能页面)

### 待完成

#### 阶段 1：核心基础设施完善
- [ ] 实现 flutter_rust_bridge FFI 集成
- [ ] 创建共享 UI 组件库
- [ ] 实现虚拟滚动组件
- [ ] 完善事件流监听机制

#### 阶段 2：核心功能实现
- [ ] SearchPage 完整实现
  - [ ] 虚拟滚动日志列表
  - [ ] 关键词高亮渲染
  - [ ] 搜索过滤面板
  - [ ] 统计面板
- [ ] WorkspacesPage 完整实现
  - [ ] 工作区列表
  - [ ] 导入文件夹对话框
  - [ ] 工作区操作

#### 阶段 3：管理功能实现
- [ ] KeywordsPage 完整实现
- [ ] TasksPage 完整实现

#### 阶段 4：辅助功能实现
- [ ] SettingsPage 完整实现
- [ ] PerformancePage 完整实现 (图表集成)

#### 阶段 5：测试与部署
- [ ] 单元测试
- [ ] 集成测试
- [ ] 桌面应用打包

## 技术栈

| 类别 | 技术选型 |
|------|---------|
| **前端框架** | Flutter 3.27+ |
| **开发语言** | Dart 3.6+ |
| **状态管理** | Riverpod 3.0 |
| **路由** | go_router 14+ |
| **国际化** | flutter_localizations + intl |
| **虚拟滚动** | flutter_virtual_scroll |
| **图表** | fl_chart |
| **图标** | lucide_icons_flutter |
| **代码生成** | freezed + json_serializable |
| **后端集成** | flutter_rust_bridge (待实现) |

## 项目结构

```
log-analyzer_flutter/
├── lib/
│   ├── main.dart                 # 应用入口
│   ├── core/                     # 核心基础设施
│   │   ├── router/               # 路由配置
│   │   ├── theme/                # 主题配置
│   │   └── constants/            # 常量定义
│   ├── features/                 # 功能模块
│   │   ├── search/               # 搜索功能
│   │   ├── workspace/            # 工作区管理
│   │   ├── keyword/              # 关键词管理
│   │   ├── task/                 # 任务管理
│   │   ├── settings/             # 设置
│   │   └── performance/          # 性能监控
│   ├── shared/                   # 共享代码
│   │   ├── models/               # 数据模型
│   │   ├── providers/            # Riverpod providers
│   │   ├── services/             # API 服务
│   │   └── widgets/              # 通用组件
│   └── l10n/                     # 国际化
├── test/                         # 测试
├── pubspec.yaml                  # 依赖配置
└── analysis_options.yaml         # 分析配置
```

## 开发环境要求

- **Flutter SDK**: 3.27+
- **Dart SDK**: 3.6+
- **Rust**: 1.70+ (用于 flutter_rust_bridge)
- **Desktop Build Tools**:
  - Windows: MSVC + Windows SDK
  - macOS: Xcode Command Line Tools
  - Linux: GTK3/GTK4 开发库

## 快速开始

### 1. 安装 Flutter

```bash
# 下载 Flutter SDK
# https://flutter.dev/docs/get-started/install

# 验证安装
flutter doctor

# 安装桌面构建工具
flutter doctor --verbose
```

### 2. 安装依赖

```bash
cd log-analyzer_flutter
flutter pub get
```

### 3. 运行开发版本

```bash
# Windows
flutter run -d windows

# macOS
flutter run -d macos

# Linux
flutter run -d linux
```

### 4. 代码生成

```bash
# 生成 Freezed/JSON 序列化代码
flutter pub run build_runner build --delete-conflicting-outputs

# 生成 Riverpod Provider 代码
flutter pub run build_runner build --delete-conflicting-outputs
```

### 5. 构建生产版本

```bash
# Windows
flutter build windows --release

# macOS
flutter build macos --release

# Linux
flutter build linux --release
```

## 后端集成 (flutter_rust_bridge)

### 配置步骤

1. **在 Rust 后端添加 flutter_rust_bridge**

```toml
# Cargo.toml
[dependencies]
flutter_rust_bridge = "2.0.0"
```

2. **创建 Bridge 模块**

```rust
// src-tauri/src/bridge.rs
use flutter_rust_bridge::frb;

#[frb(sync)] // 同步方法
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[frb(async)] // 异步方法
pub async fn search_logs(
    query: String,
    workspace_id: Option<String>,
    max_results: usize,
) -> Result<String, String> {
    // 实现搜索逻辑
    Ok("search_id".to_string())
}
```

3. **生成 Dart 代码**

```bash
# 安装 flutter_rust_bridge CLI
cargo install flutter_rust_bridge_codegen

# 生成 Dart 绑定
flutter_rust_bridge_codegen \
  --rust-input src-tauri/src/bridge.rs \
  --dart-output ./lib/core/bridge/bridge_generated.dart
```

4. **在 Flutter 中使用**

```dart
import 'package:log_analyzer_flutter/core/bridge/bridge_generated.dart';

final bridge = RustBridgeImpl();

// 同步调用
final greeting = bridge.greet("World");

// 异步调用
final searchId = await bridge.searchLogs(
  query: "error",
  workspaceId: null,
  maxResults: 10000,
);
```

## 关键设计决策

### 1. 状态管理选择 Riverpod

- 2026 年 Flutter 社区推荐方案
- 无需 BuildContext 即可访问状态
- 自动内存管理
- 编译时安全

### 2. 后端集成选择 flutter_rust_bridge

- 原生 FFI 性能，避免 JSON 序列化开销
- 类型安全的代码生成
- 直接调用 Rust 函数

### 3. 事件幂等性保证

使用版本号机制（与 React 版本一致）：
- 每个事件携带单调递增的 `version` 字段
- EventBus 记录已处理事件的版本号
- 跳过旧版本或重复事件

## 与 React 版本的对应关系

| React 文件 | Flutter 文件 |
|-----------|-------------|
| `src/stores/appStore.ts` | `lib/shared/providers/app_provider.dart` |
| `src/stores/workspaceStore.ts` | `lib/shared/providers/workspace_provider.dart` |
| `src/stores/taskStore.ts` | `lib/shared/providers/task_provider.dart` |
| `src/stores/keywordStore.ts` | `lib/shared/providers/keyword_provider.dart` |
| `src/services/api.ts` | `lib/shared/services/api_service.dart` |
| `src/services/SearchQueryBuilder.ts` | `lib/shared/services/search_query_builder.dart` |
| `src/events/EventBus.ts` | `lib/shared/services/event_bus.dart` |
| `src/types/common.ts` | `lib/shared/models/common.dart` |
| `src/types/search.ts` | `lib/shared/models/search.dart` |
| `src/pages/SearchPage.tsx` | `lib/features/search/presentation/search_page.dart` |

## 贡献指南

1. 遵循 Flutter/Dart 代码规范
2. 运行 `flutter analyze` 检查代码
3. 运行 `flutter test` 执行测试
4. 使用 `flutter pub run build_runner build` 生成代码

## 许可证

MIT License
