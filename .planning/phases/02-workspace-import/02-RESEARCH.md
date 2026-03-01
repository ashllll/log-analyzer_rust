# Phase 2: 工作区与文件导入 - Research

**Researched:** 2026-03-01
**Domain:** Flutter Desktop UI + Rust FFI Integration
**Confidence:** HIGH

## Summary

Phase 2 focuses on workspace management (create, open, delete) and file/archive import functionality. The project already has substantial Rust backend implementation (workspace.rs, import.rs, archive module) that handles the core logic. The main work is integrating these with the Flutter frontend using Riverpod state management and flutter_rust_bridge FFI.

**Primary recommendation:** Leverage existing Rust commands (workspace.rs, import.rs) via FFI, implement Flutter UI components following CONTEXT.md specifications, use Riverpod for state management with task progress streaming.

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**工作区管理 UI:**
- 卡片布局，每个工作区作为卡片展示
- 完整信息（名称、创建时间、文件数量、索引状态、总大小）
- 空状态引导创建
- 删除确认对话框
- 状态指示器（就绪/索引中/错误）
- 名称创建时自定义，可包含中文
- 最近优先排序（最近打开的3个显示在最前）
- 左右布局（左侧工作区列表，右侧内容）
- Material Icons 图标风格
- 悬停显示操作按钮
- 边框+背景高亮选中状态
- 支持按名称搜索
- 自动适配深色/浅色主题
- 键盘导航支持（上下键选择，回车打开）
- 标准快捷键（Ctrl+N 新建, Ctrl+O 打开, Delete 删除）

**文件导入流程:**
- 同时支持拖放和按钮选择
- 多选支持
- 压缩包自动检测并提示
- 重复文件提示用户选择
- 指定拖放区域
- 视觉反馈（高亮边框和提示文字）
- 大文件流式处理显示进度
- 仅允许日志相关文件（.log, .txt, .json等）
- 复制到工作区存储
- 解压后导入
- 错误跳过继续，结束后报告
- 支持撤销和取消
- 符号链接跳过，空文件夹提示用户
- 导入完成显示通知

**导入进度显示:**
- 模态对话框展示
- 详细信息（已处理/总文件数、当前文件、预估时间）
- 单文件解压/处理进度
- 取消按钮
- 圆形进度条样式
- 错误列表显示
- 摘要报告
- 预估剩余时间和处理速度
- 实时更新
- 支持暂停和继续

**压缩包处理:**
- 全部格式支持（ZIP, TAR, GZ, RAR, 7Z）
- RAR 使用 Rust 原生支持
- 7Z 使用 sevenz-rust 库
- 密码压缩提示输入
- 损坏报告但继续处理
- 工作区目录下临时文件夹解压
- 导入前可浏览内容
- 流式解压大压缩包
- 递归解压嵌套压缩
- 不删除原压缩包
- 自动检测编码，失败用 UTF-8
- 支持仅解压选择的文件

### Claude's Discretion
- 加载骨架设计
- 错误状态处理
- 主题细节（颜色、阴影）
- 具体组件实现细节

### Deferred Ideas (OUT OF SCOPE)
- 搜索结果展示（时间轴排序、正则匹配等）— Phase 3

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WORK-01 | 用户可以创建新的工作区 | Rust workspace.rs 有 load_workspace, 需要新增 create_workspace 命令；Flutter UI 已部分实现 |
| WORK-02 | 用户可以打开已有工作区 | Rust workspace.rs load_workspace 可用；Flutter 已实现工作区列表 |
| WORK-03 | 用户可以删除工作区 | Rust workspace.rs delete_workspace 可用 |
| WORK-04 | 用户可以查看工作区状态 (文件数、索引状态) | Rust metadata_store 可获取文件数；需要索引状态集成 |
| FILE-01 | 用户可以导入文件夹 | Rust import.rs import_folder 已实现 |
| FILE-02 | 支持导入 ZIP 压缩包 | Rust archive/zip_handler.rs 已实现 |
| FILE-03 | 支持导入 TAR 压缩包 | Rust archive/tar_handler.rs 已实现 |
| FILE-04 | 支持导入 GZIP 压缩包 | Rust archive/gz_handler.rs 已实现 |
| FILE-05 | 支持导入 RAR 压缩包 | Rust archive/rar_handler.rs 已实现 |
| FILE-06 | 支持导入 7Z 压缩包 | Rust archive/sevenz_handler.rs 已实现 |
| FILE-07 | 显示文件导入进度 | TaskManager 支持进度更新；需要 Flutter 进度 UI |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `flutter_riverpod` | 3.0.0 | 状态管理 | Flutter 官方推荐，Flutter 3.x 最佳实践 |
| `flutter_rust_bridge` | 2.0.0 | Flutter-Rust FFI | 项目已启用，与 Rust 后端通信 |
| `file_picker` | 8.0.0 | 文件/文件夹选择 | 跨平台支持，已在 pubspec.yaml |
| `go_router` | 14.0.0 | 路由管理 | Flutter 桌面应用推荐方案 |

### Supporting (Flutter)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `freezed_annotation` | 3.0.0 | 不可变数据模型 | 工作区状态、导入进度模型 |
| `json_annotation` | 4.9.0 | JSON 序列化 | FFI 数据传输 |
| `desktop_drop` | latest | 拖放支持 | 桌面拖放功能（需添加） |

### Supporting (Rust Backend - Already Available)

| Library/Module | Purpose | Status |
|----------------|---------|--------|
| `commands/workspace.rs` | 工作区 CRUD | 已实现 load/delete |
| `commands/import.rs` | 文件导入 | 已实现 import_folder |
| `archive/zip_handler.rs` | ZIP 解压 | 已实现 |
| `archive/tar_handler.rs` | TAR 解压 | 已实现 |
| `archive/gz_handler.rs` | GZ 解压 | 已实现 |
| `archive/rar_handler.rs` | RAR 解压 | 已实现 |
| `archive/sevenz_handler.rs` | 7Z 解压 | 已实现 |
| `task_manager/` | 异步任务进度 | 已实现 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `desktop_drop` | 原生 DragTarget | desktop_drop 更成熟，跨平台兼容更好 |
| Riverpod | Provider / Bloc | Riverpod 3.0 是 Flutter 官方推荐，类型安全更好 |
| FFI | HTTP API | 项目已确定 FFI 优先，延迟更低 |

**Installation:**
```bash
cd log-analyzer_flutter
flutter pub add desktop_drop
```

---

## Architecture Patterns

### Recommended Project Structure

```
log-analyzer_flutter/lib/
├── features/workspace/
│   ├── domain/
│   │   └── models/
│   │       └── workspace_model.dart      # 工作区数据模型
│   └── presentation/
│       ├── pages/
│       │   └── workspaces_page.dart      # 工作区列表页
│       ├── widgets/
│       │   ├── workspace_card.dart       # 工作区卡片组件
│       │   ├── create_workspace_dialog.dart  # 创建对话框
│       │   ├── import_progress_dialog.dart   # 导入进度对话框
│       │   └── drop_zone.dart            # 拖放区域
│       └── providers/
│           └── workspace_providers.dart  # 工作区相关 providers
├── features/import/
│   ├── domain/
│   │   └── models/
│   │       └── import_task.dart          # 导入任务模型
│   └── presentation/
│       └── providers/
│           └── import_progress_provider.dart  # 导入进度状态
└── shared/
    ├── services/
    │   └── ffi_bridge.dart               # FFI 桥接封装
    └── providers/
        └── workspace_provider.dart       # 已存在的工作区 provider
```

### Pattern 1: Riverpod State Management for Import Progress

**What:** 使用 Riverpod 的 AsyncValue 状态流处理导入进度

**When to use:** 文件导入、压缩包解压等长时间操作

**Example:**
```dart
// 使用 Riverpod 管理导入进度
import 'package:flutter_riverpod/flutter_riverpod.dart';

// 导入任务状态
class ImportProgress {
  final int totalFiles;
  final int processedFiles;
  final String currentFile;
  final double progressPercent;
  final List<String> errors;
  final ImportStatus status;
}

enum ImportStatus { idle, importing, paused, completed, cancelled }

// Provider 定义
final importProgressProvider = StateNotifierProvider<ImportProgressNotifier, ImportProgress>((ref) {
  return ImportProgressNotifier();
});

class ImportProgressNotifier extends StateNotifier<ImportProgress> {
  ImportProgressNotifier() : super(ImportProgress(
    totalFiles: 0,
    processedFiles: 0,
    currentFile: '',
    progressPercent: 0.0,
    errors: [],
    status: ImportStatus.idle,
  ));

  void updateProgress(int processed, int total, String currentFile) {
    state = state.copyWith(
      processedFiles: processed,
      totalFiles: total,
      currentFile: currentFile,
      progressPercent: total > 0 ? processed / total : 0.0,
    );
  }
}
```

### Pattern 2: FFI Bridge Integration

**What:** 通过 flutter_rust_bridge 调用 Rust 命令

**When to use:** 所有与 Rust 后端通信

**Example:**
```dart
// lib/shared/services/ffi_bridge.dart
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

// 假设的 FFI 桥接（基于项目现有模式）
class FfiBridge {
  final api = getFFIApi(); // flutter_rust_bridge 生成

  // 创建工作区
  Future<String> createWorkspace(String name, String path) async {
    return await api.createWorkspace(name: name, path: path);
  }

  // 导入文件夹
  Future<String> importFolder(String workspaceId, String path) async {
    return await api.importFolder(workspaceId: workspaceId, path: path);
  }

  // 导入压缩包
  Future<String> importArchive(String workspaceId, String path) async {
    return await api.importArchive(workspaceId: workspaceId, path: path);
  }

  // 监听任务进度
  Stream<TaskUpdate> watchTask(String taskId) {
    return api.watchTask(taskId: taskId);
  }
}
```

### Pattern 3: Drag and Drop Zone

**What:** 使用 desktop_drop 实现拖放区域

**When to use:** 文件导入拖放功能

**Example:**
```dart
import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/material.dart';

class DropZoneWidget extends StatefulWidget {
  final Function(List<String> paths) onFilesDropped;
  final Widget child;

  const DropZoneWidget({
    super.key,
    required this.onFilesDropped,
    required this.child,
  });
}

class _DropZoneWidgetState extends State<DropZoneWidget> {
  bool _isDragging = false;

  @override
  Widget build(BuildContext context) {
    return DropTarget(
      onDragDone: (detail) {
        final paths = detail.files.map((f) => f.path).toList();
        widget.onFilesDropped(paths);
      },
      onDragEntered: (_) => setState(() => _isDragging = true),
      onDragExited: (_) => setState(() => _isDragging = false),
      child: Container(
        decoration: BoxDecoration(
          border: _isDragging
              ? Border.all(color: Colors.blue, width: 2)
              : null,
          borderRadius: BorderRadius.circular(8),
        ),
        child: widget.child,
      ),
    );
  }
}
```

### Anti-Patterns to Avoid

- **直接调用 Rust 命令而不封装:** 应该通过 FfiBridge 服务类封装，保持 UI 层干净
- **在 UI 线程执行耗时操作:** 所有 Rust 调用都是异步的，确保使用 await
- **忽略错误处理:** Rust 命令可能失败，需要 try-catch 和用户友好的错误提示
- **状态管理过度复杂:** 使用简单的 StateNotifier 即可，不需要过度工程

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 文件选择 | 手写系统对话框 | `file_picker` | 跨平台支持，成熟稳定 |
| 拖放功能 | 原生实现 | `desktop_drop` | Flutter 桌面官方推荐 |
| 状态管理 | 手写 ChangeNotifier | Riverpod | 官方推荐，类型安全 |
| 压缩包解压 | 手写解压逻辑 | Rust archive 模块 | 后端已完整实现 |
| 进度追踪 | 手写进度流 | TaskManager + Stream | 后端已支持 |

**Key insight:** 后端 Rust 已实现完整的压缩包处理、工作区管理、任务进度追踪，前端只需调用 FFI 接口并实现 UI。

---

## Common Pitfalls

### Pitfall 1: FFI 调用阻塞 UI
**What goes wrong:** 同步调用 Rust 命令导致 UI 卡顿
**Why it happens:** Flutter 单线程模型，FFI 调用需要 await
**How to avoid:** 所有 FFI 调用使用 async/await，必要时显示 loading 状态
**Warning signs:** UI 响应缓慢、点击无反应

### Pitfall 2: 进度更新丢失
**What goes wrong:** 后端发送的进度事件前端未接收
**Why it happens:** Stream 未正确订阅或监听器未初始化
**How to avoid:** 在组件挂载时订阅，进度更新时使用 ref.read/notifier
**Warning signs:** 进度条不动，但导入实际在进行

### Pitfall 3: 大文件内存问题
**What goes wrong:** 大压缩包导入时内存溢出
**Why it happens:** 未使用流式处理
**How to avoid:** 后端已实现流式处理，前端只需正确处理 Stream 事件
**Warning signs:** 导入大文件时应用崩溃

### Pitfall 4: 路径编码问题
**What goes wrong:** 中文路径显示乱码或无法访问
**Why it happens:** Windows/macOS/Linux 路径编码差异
**How to avoid:** 使用 Rust 的 dunce crate 处理路径，前端传递标准字符串
**Warning signs:** 创建工作区失败，提示路径无效

---

## Code Examples

### Workspace Card Widget
```dart
// Source: 基于现有 workspaces_page.dart 模式扩展
class WorkspaceCard extends StatelessWidget {
  final Workspace workspace;
  final bool isSelected;
  final VoidCallback onTap;
  final VoidCallback onDelete;

  const WorkspaceCard({
    super.key,
    required this.workspace,
    required this.isSelected,
    required this.onTap,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: isSelected ? 4 : 1,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: isSelected
            ? BorderSide(color: AppColors.primary, width: 2)
            : BorderSide.none,
      ),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(
                    _getStatusIcon(workspace.status),
                    color: _getStatusColor(workspace.status),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      workspace.name,
                      style: const TextStyle(
                        fontWeight: FontWeight.w600,
                        fontSize: 16,
                      ),
                    ),
                  ),
                  if (isSelected)
                    IconButton(
                      icon: const Icon(Icons.delete_outline),
                      onPressed: onDelete,
                    ),
                ],
              ),
              const SizedBox(height: 8),
              Text('文件数: ${workspace.fileCount}'),
              Text('大小: ${_formatSize(workspace.totalSize)}'),
              Text('创建时间: ${workspace.createdAt}'),
            ],
          ),
        ),
      ),
    );
  }
}
```

### Import Progress Dialog
```dart
// Source: 基于现有 task_provider.dart 模式
class ImportProgressDialog extends ConsumerWidget {
  final String taskId;

  const ImportProgressDialog({super.key, required this.taskId});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final progress = ref.watch(importProgressProvider(taskId));

    return AlertDialog(
      title: const Text('导入进度'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: 100,
            height: 100,
            child: CircularProgressIndicator(
              value: progress.progressPercent,
              strokeWidth: 8,
            ),
          ),
          const SizedBox(height: 16),
          Text('${progress.processedFiles} / ${progress.totalFiles}'),
          Text(progress.currentFile),
          if (progress.errors.isNotEmpty)
            Text('错误: ${progress.errors.length}'),
        ],
      ),
      actions: [
        if (progress.status == ImportStatus.importing)
          TextButton(
            onPressed: () => ref.read(importProgressProvider(taskId).notifier).cancel(),
            child: const Text('取消'),
          ),
        if (progress.status == ImportStatus.paused)
          TextButton(
            onPressed: () => ref.read(importProgressProvider(taskId).notifier).resume(),
            child: const Text('继续'),
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('关闭'),
        ),
      ],
    );
  }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri WebView | Flutter FFI | Phase 1 | 更现代化 UI，开发效率更高 |
| React 组件 | Flutter Widget | Phase 1 | 跨平台桌面支持 |
| HTTP API | FFI Bridge | Phase 1 | 更低延迟，更好的类型安全 |

**Deprecated/outdated:**
- Tauri 1.x WebView 前端 - 已迁移到 Flutter
- React 组件 - 不再维护

---

## Open Questions

1. **是否需要新增 create_workspace Rust 命令？**
   - What we know: workspace.rs 有 load_workspace 和 delete_workspace，但 create_workspace 逻辑分散在 import_folder 中
   - What's unclear: 创建工作区的完整流程是否需要独立命令
   - Recommendation: 先使用现有 import_folder 流程，评估是否需要独立创建

2. **拖放区域是否需要在工作区列表和详情页都实现？**
   - What we know: CONTEXT.md 说"导入目标: 当前工作区"
   - What's unclear: 是否在打开工作区后也能导入
   - Recommendation: 在工作区详情页实现拖放导入

3. **任务进度 Stream 如何从 Rust 传递到 Flutter？**
   - What we know: TaskManager 有事件系统，FFI 支持 Stream
   - What's unclear: 具体的事件类型和序列化格式
   - Recommendation: 参考现有 task_provider.dart 的实现模式

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | flutter_test (Dart) |
| Config file | `log-analyzer_flutter/test/` |
| Quick run command | `flutter test` |
| Full suite command | `flutter test --coverage` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WORK-01 | 创建工作区 | unit | `flutter test test/workspace/` | 待创建 |
| WORK-02 | 打开工作区 | unit | `flutter test test/workspace/` | 待创建 |
| WORK-03 | 删除工作区 | unit | `flutter test test/workspace/` | 待创建 |
| WORK-04 | 查看工作区状态 | unit | `flutter test test/workspace/` | 待创建 |
| FILE-01 | 导入文件夹 | integration | `flutter test test/import/` | 待创建 |
| FILE-02~06 | 导入压缩包 | integration | `flutter test test/import/` | 待创建 |
| FILE-07 | 导入进度显示 | widget | `flutter test test/widgets/` | 待创建 |

### Sampling Rate
- **Per task commit:** `flutter test`
- **Per wave merge:** `flutter test --coverage`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test/workspace/workspace_provider_test.dart` — 覆盖 WORK-01~04
- [ ] `test/import/import_service_test.dart` — 覆盖 FILE-01~06
- [ ] `test/widgets/import_progress_test.dart` — 覆盖 FILE-07
- [ ] `test/helpers/test_fixtures.dart` — 共享测试 fixtures
- [ ] Framework install: `flutter pub get` — 已配置

---

## Sources

### Primary (HIGH confidence)
- 项目现有代码: `log-analyzer_flutter/lib/features/workspace/presentation/workspaces_page.dart`
- 项目现有代码: `log-analyzer/src-tauri/src/commands/workspace.rs`
- 项目现有代码: `log-analyzer/src-tauri/src/commands/import.rs`
- pubspec.yaml: Flutter 依赖配置

### Secondary (MEDIUM confidence)
- Flutter Riverpod 文档 (基于项目现有使用模式)
- desktop_drop 包文档 (已列入待添加)

### Tertiary (LOW confidence)
- N/A

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - 项目已确定技术栈，Flutter + Riverpod + FFI
- Architecture: HIGH - 后端已实现核心功能，前端只需集成
- Pitfalls: MEDIUM - 基于常见 Flutter 桌面问题

**Research date:** 2026-03-01
**Valid until:** 2026-03-31 (技术栈稳定，短期内无重大变化)
