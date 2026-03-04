# 代码生成和构建指南

## 概述

本文档描述如何运行代码生成和构建 Flutter 桌面应用。

## 前置条件

1. **Flutter SDK 已安装**: `F:\flutter_windows_3.38.9-stable\flutter\bin`
2. **Rust 工具链已安装**: MSVC + Windows SDK
3. **依赖已安装**: `flutter pub get` 已运行

## 步骤 1: 代码生成

### 1.1 运行 build_runner

在 Flutter 项目根目录（`log-analyzer_flutter/`）运行：

```bash
cd log-analyzer_flutter

# Windows (CMD 或 PowerShell)
dart run build_runner build --delete-conflicting-outputs

# 或使用 flutter 命令
flutter pub run build_runner build --delete-conflicting-outputs
```

### 1.2 生成内容

build_runner 将生成以下文件：

```
lib/shared/models/
├── common.freezed.dart       # Freezed 生成的代码
├── common.g.dart             # JSON 序列化代码
├── search.freezed.dart       # (如果有 search.dart)
├── search.g.dart
├── keyword.freezed.dart
├── keyword.g.dart
└── app_state.g.dart          # Riverpod provider 生成代码

lib/shared/providers/
├── app_provider.freezed.dart
├── app_provider.g.dart
├── workspace_provider.freezed.dart
├── workspace_provider.g.dart
├── task_provider.freezed.dart
├── task_provider.g.dart
├── keyword_provider.freezed.dart
└── keyword_provider.g.dart
```

### 1.3 处理常见错误

#### 错误: `Could not resolve .freezed.dart`

**原因**: `part` 声明指向不存在的文件

**解决**:
1. 确认每个 `@freezed` 类都有对应的 `part` 声明
2. 运行代码生成后，生成的 `.freezed.dart` 和 `.g.dart` 文件应该出现

#### 错误: `Type 'X' is not a subtype of type 'Y'`

**原因**: Freezed 生成的代码类型不匹配

**解决**:
```bash
# 清理并重新生成
flutter clean
dart run build_runner build --delete-conflicting-outputs
```

## 步骤 2: 分析代码

运行代码生成后，检查代码质量：

```bash
# 运行静态分析
flutter analyze

# 运行 Lint（如果配置了）
dart lint
```

### 2.1 常见分析问题

#### 未使用导入警告

**示例**: `The import 'package:xxx/xxx.dart' is unused`

**解决**: 删除未使用的导入或使用 `// ignore: unused_import`

#### 缺少返回类型

**示例**: `The return type 'X' is not specified`

**解决**: 添加返回类型或使用 `Future<void>`

## 步骤 3: 构建应用

### 3.1 开发构建

```bash
# 检查可用的设备
flutter devices

# Windows 桌面构建
flutter build windows --debug

# macOS
flutter build macos --debug

# Linux
flutter build linux --debug
```

输出位置：
- **Windows**: `build\windows\runner\Debug\`
- **macOS**: `build\macos\Build\Products\Debug\`
- **Linux**: `build\linux\build\bundle\`

### 3.2 运行应用

```bash
# Windows
build\windows\runner\Debug\log_analyzer_flutter.exe

# macOS
open build/macos/Build/Products/Debug/log_analyzer_flutter.app

# Linux
./build/linux/bundle/log_analyzer_flutter
```

## 步骤 4: 生产构建

### 4.1 发布模式构建

```bash
# Windows
flutter build windows --release

# macOS
flutter build macos --release

# Linux
flutter build linux --release
```

### 4.2 输出位置

- **Windows**: `build\windows\runner\Release\`
- **macOS**: `build\macos\Build\Products\Release\`
- **Linux**: `build\linux\build\bundle\`

## 验证清单

### 代码质量

- [ ] 无 `flutter analyze` 错误
- [ ] 无 Lint 警告（或已审查并确认可忽略）
- [ ] Freezed 生成文件存在（`.freezed.dart`, `.g.dart`）

### 功能验证

- [ ] 应用正常启动
- [ ] 所有页面可访问
- [ ] Riverpod providers 正常工作
- [ ] 路由导航正常
- [ ] 主题样式正确应用

### 性能验证

- [ ] 首屏加载时间 < 3 秒
- [ ] 搜索页面滚动流畅（60FPS）
- [ ] 内存使用正常

## 故障排查

### 问题: Freezed 代码未生成

**症状**: 运行 build_runner 后 `.freezed.dart` 文件不存在

**解决**:
1. 检查 `pubspec.yaml` 中 freezed 版本
2. 确认所有 `part` 语句正确
3. 尝试清理输出: `flutter clean`
4. 重新运行 build_runner

### 问题: Riverpod 生成失败

**症状**: `app_provider.g.dart` 生成错误

**解决**:
1. 检查 `@riverpod` 注解语法
2. 确认 `riverpod_annotation` 和 `flutter_riverpod` 版本兼容
3. 检查是否有循环依赖

### 问题: Windows 构建失败

**症状**: `The code for the Runner is already signed`

**解决**:
```bash
# 清理构建缓存
flutter clean

# 删除签名配置（如果有）
# 在 build\windows\runner\ 目录下删除相关配置文件
```

## CI/CD 集成

### GitHub Actions 配置

创建 `.github/workflows/flutter-build.yml`:

```yaml
name: Flutter Build

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: windows-latest

    steps:
    - uses: actions/checkout@v4

    - name: Setup Flutter
      uses: subosito/flutter-action@v2
      with:
        channel: 'stable'
        flutter-version: '3.38.9'

    - name: Install dependencies
      run: flutter pub get

    - name: Generate code
      run: dart run build_runner build --delete-conflicting-outputs

    - name: Analyze code
      run: flutter analyze

    - name: Build Windows
      run: flutter build windows --release

    - name: Upload artifact
      uses: actions/upload-artifact@v4
      with:
        name: windows-release
        path: build\windows\runner\Release\
```

## 参考

- [Flutter 构建官方文档](https://docs.flutter.dev/build/introduction)
- [Freezed 使用指南](https://pub.dev/packages/freezed)
- [Riverpod 代码生成](https://riverpod.dev/docs/concepts/generating_code)
