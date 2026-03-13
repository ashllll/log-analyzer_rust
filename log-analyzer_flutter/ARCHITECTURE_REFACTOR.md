# Flutter 前端架构优化方案

## 优化概述

本方案针对原 Flutter 项目存在的六大问题，提供业内成熟的解决方案：

1. ✅ **同步FFI调用阻塞UI** → 使用 Isolate 异步封装
2. ✅ **Provider初始化模式问题** → 使用 AsyncNotifier 惰性初始化
3. ✅ **页面组件过于臃肿** → 采用 Clean Architecture 分层
4. ✅ **错误边界有副作用** → 使用 Zone 和 PlatformDispatcher 全局处理
5. ✅ **缺乏Repository层** → 添加 Repository 层和 UseCase 层
6. ✅ **轮询机制效率低** → 改为事件驱动架构

---

## 1. 项目结构（Clean Architecture）

```
lib/
├── core/                          # 核心层
│   ├── errors/                    # 错误处理
│   │   ├── app_error.dart         # 错误类型定义（fpdart）
│   │   └── error_handler.dart     # 全局错误处理器（无副作用）
│   ├── usecases/                  # 用例抽象
│   │   └── usecase.dart           # UseCase 基类
│   ├── utils/                     # 工具类
│   │   └── isolate_utils.dart     # Isolate 工具类
│   └── constants/                 # 常量配置
│
├── data/                          # 数据层
│   ├── datasources/               # 数据源
│   │   ├── ffi_datasource.dart    # FFI 数据源（异步封装）
│   │   └── event_datasource.dart  # 事件数据源（事件驱动）
│   ├── models/                    # 数据模型（DTO）
│   ├── mappers/                   # 数据映射器
│   │   ├── workspace_mapper.dart
│   │   └── task_mapper.dart
│   └── repositories/              # 仓库实现
│       ├── workspace_repository_impl.dart
│       └── task_repository_impl.dart
│
├── domain/                        # 领域层（核心业务逻辑）
│   ├── entities/                  # 领域实体
│   │   ├── workspace.dart
│   │   ├── log_entry.dart
│   │   └── task.dart
│   ├── repositories/              # 仓库接口
│   │   ├── workspace_repository.dart
│   │   ├── search_repository.dart
│   │   └── task_repository.dart
│   └── usecases/                  # 用例（可选）
│
├── presentation/                  # 表示层
│   ├── providers/                 # Riverpod Providers
│   │   ├── workspace_provider.dart
│   │   └── task_provider.dart
│   ├── pages/                     # 页面（精简）
│   │   ├── workspaces/
│   │   │   └── workspaces_page.dart
│   │   └── tasks/
│   │       └── tasks_page.dart
│   └── widgets/                   # 组件
│       ├── common/
│       │   ├── async_value_widget.dart
│       │   └── error_view.dart
│       ├── workspace/
│       │   ├── workspace_list_view.dart
│       │   └── create_workspace_dialog.dart
│       └── search/
│
└── main.dart                      # 应用入口
```

---

## 2. 关键技术改进

### 2.1 异步 FFI 调用（Isolate 模式）

**问题**：原代码中 `BridgeService` 直接同步调用 FFI，阻塞 UI 线程

**解决方案**：使用 `compute` 函数将所有 FFI 调用包装到 Isolate 中

```dart
// 优化前（阻塞 UI）
List<WorkspaceData> getWorkspaces() {
  return ffi.getWorkspaces(); // 同步调用，阻塞 UI！
}

// 优化后（异步非阻塞）
AppTask<List<Workspace>> getWorkspaces() {
  return TaskEither(() async {
    final result = await AsyncFfiCall.queryList(
      query: ffi.getWorkspaces,
      timeout: const Duration(seconds: 10),
    );
    return right(result);
  });
}
```

**核心实现**：
- `lib/core/utils/isolate_utils.dart` - Isolate 工具类
- `lib/data/datasources/ffi_datasource.dart` - 异步 FFI 数据源

### 2.2 AsyncNotifier 状态管理

**问题**：原代码使用 `Future.microtask` 在 `build()` 中触发异步初始化，不可靠

**解决方案**：使用 Riverpod 3.0 的 `AsyncNotifier`

```dart
@riverpod
class WorkspaceList extends _$WorkspaceList {
  @override
  Future<List<Workspace>> build() async {
    // Riverpod 自动处理异步加载状态
    final repository = ref.read(workspaceRepositoryProvider);
    final result = await repository.getWorkspaces().run();
    
    return result.fold(
      (error) => throw error,  // 自动转换为 AsyncError
      (workspaces) => workspaces,
    );
  }

  // 刷新方法
  Future<void> refresh() async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      // 重新加载数据
    });
  }
}
```

**使用方式**：
```dart
// Widget 中使用
final workspacesAsync = ref.watch(workspaceListProvider);

return workspacesAsync.when(
  data: (workspaces) => WorkspaceListView(workspaces: workspaces),
  loading: () => SkeletonLoading(),
  error: (error, stack) => ErrorView(error: error, onRetry: ...),
);
```

### 2.3 无副作用错误边界

**问题**：原 `ErrorBoundary` 直接修改 `FlutterError.onError`，有副作用

**解决方案**：使用 Flutter Zone 和 PlatformDispatcher

```dart
// main.dart
void main() {
  ErrorHandler.runInZone(() {
    ErrorHandler.initialize(config: ErrorHandlerConfig.debug);
    
    runApp(
      ErrorHandler.wrapApp(
        const ProviderScope(child: MyApp()),
      ),
    );
  });
}

// 全局错误处理
class ErrorHandler {
  static void initialize({ErrorHandlerConfig config}) {
    FlutterError.onError = _handleFlutterError;
    PlatformDispatcher.instance.onError = _handlePlatformError;
  }
  
  static void runInZone(VoidCallback runApp) {
    runZonedGuarded(runApp, _handleZoneError);
  }
}
```

**特点**：
- 不在 Widget 生命周期中修改全局状态
- 支持同步和异步错误捕获
- 自动上报到 Sentry（可选）

### 2.4 Repository 层和函数式错误处理

**问题**：原代码 `ApiService` 直接暴露 FFI 调用，缺乏抽象

**解决方案**：
1. 添加 Repository 层作为领域层和数据层的桥梁
2. 使用 `fpdart` 的 `TaskEither` 实现函数式错误处理

```dart
// domain/repositories/workspace_repository.dart
abstract class WorkspaceRepository {
  AppTask<List<Workspace>> getWorkspaces();
  AppTask<Workspace> createWorkspace(CreateWorkspaceParams params);
  // ...
}

// data/repositories/workspace_repository_impl.dart
class WorkspaceRepositoryImpl implements WorkspaceRepository {
  @override
  AppTask<List<Workspace>> getWorkspaces() {
    return TaskEither(() async {
      final result = await _ffiDataSource.getWorkspaces().run();
      return result.map(WorkspaceMapper.fromFfiList);
    });
  }
}
```

**函数式错误处理优势**：
- 强制处理错误路径，不会遗漏
- 链式调用支持：`.map()`, `.flatMap()`, `.tap()`
- 类型安全：`Either<AppError, T>`

### 2.5 事件驱动的任务更新

**问题**：原代码使用固定间隔轮询任务状态，效率低

**解决方案**：使用 Stream 和事件总线

```dart
// data/datasources/event_datasource.dart
class EventDataSource {
  final _taskEventController = StreamController<TaskEvent>.broadcast();
  
  Stream<TaskEvent> get taskEvents => _taskEventController.stream;
  
  void publishTaskEvent(TaskEvent event) {
    _taskEventController.add(event);
  }
}

// presentation/providers/task_provider.dart
@riverpod
class TaskList extends _$TaskList {
  StreamSubscription<List<Task>>? _subscription;

  @override
  Future<List<Task>> build() async {
    // 订阅任务事件流
    _subscribeToTaskStream();
    
    final repository = ref.read(taskRepositoryProvider);
    final result = await repository.getTasks().run();
    return result.getOrElse((_) => []);
  }

  void _subscribeToTaskStream() {
    final stream = ref.read(taskStreamProvider);
    _subscription = stream.listen((tasks) {
      state = AsyncValue.data(tasks); // 自动更新 UI
    });
  }
}

// 使用 StreamProvider 直接暴露流
@riverpod
Stream<List<Task>> taskStream(Ref ref) {
  final repository = ref.watch(taskRepositoryProvider);
  return repository.watchTasks(); // 实时流，无轮询
}
```

**优势**：
- 无轮询开销
- 实时更新
- 自动处理重连

---

## 3. 迁移指南

### 3.1 逐步迁移策略

1. **阶段 1**：添加新架构代码，保持旧代码运行
2. **阶段 2**：逐个页面迁移到新架构
3. **阶段 3**：移除旧代码

### 3.2 代码对比

| 场景 | 旧代码 | 新代码 |
|-----|--------|--------|
| FFI 调用 | `FfiService.getWorkspaces()` (同步) | `ffiDataSource.getWorkspaces()` (异步 Isolate) |
| 状态管理 | `Future.microtask(() => init())` | `AsyncNotifier.build()` |
| 错误处理 | `try-catch` + `setState` | `TaskEither` + `AsyncValue.when` |
| 任务更新 | `Timer.periodic(pollTasks)` | `StreamProvider` |
| 错误边界 | 修改 `FlutterError.onError` | `ErrorHandler.runInZone` |

---

## 4. 性能优化

### 4.1 缓存策略

```dart
class WorkspaceRepositoryImpl implements WorkspaceRepository {
  List<Workspace>? _cachedWorkspaces;
  DateTime? _lastCacheTime;
  static const _cacheDuration = Duration(seconds: 30);

  bool get _isCacheValid => 
    _cachedWorkspaces != null && 
    _lastCacheTime != null &&
    DateTime.now().difference(_lastCacheTime!) < _cacheDuration;
}
```

### 4.2 批量处理

```dart
// 使用 IsolateUtils.batchProcess 处理大量数据
final results = await IsolateUtils.batchProcess(
  items: largeDataList,
  processor: processItem,
  chunkSize: 100,
  maxConcurrent: 4,
);
```

---

## 5. 测试策略

### 5.1 单元测试

```dart
// 测试 UseCase
test('getWorkspaces should return list', () async {
  final mockRepo = MockWorkspaceRepository();
  when(mockRepo.getWorkspaces()).thenAnswer(
    (_) => TaskEither.right([Workspace.empty]),
  );
  
  final useCase = GetWorkspacesUseCase(mockRepo);
  final result = await useCase.execute();
  
  expect(result.isRight(), true);
});
```

### 5.2 Widget 测试

```dart
// 测试使用 AsyncValueWidget
testWidgets('should show loading', (tester) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        workspaceListProvider.overrideWith(
          (ref) => const AsyncValue.loading(),
        ),
      ],
      child: const WorkspacesPage(),
    ),
  );
  
  expect(find.byType(CircularProgressIndicator), findsOneWidget);
});
```

---

## 6. 依赖说明

新增/升级依赖：

```yaml
dependencies:
  # 函数式编程
  fpdart: ^1.1.0
  
  # 代码生成（替代部分 freezed 场景）
  dart_mappable: ^4.0.0
  
  # Isolate 管理
  isolate_manager: ^3.0.0
  
  # 事件总线
  event_bus: ^2.0.0
  
  # 依赖注入（可选）
  get_it: ^8.0.0
  injectable: ^2.0.0

dev_dependencies:
  riverpod_lint: ^3.0.0
  dart_mappable_builder: ^4.0.0
  injectable_generator: ^2.0.0
```

---

## 7. 总结

### 改进点总结

| 问题 | 优化方案 | 效果 |
|-----|---------|------|
| 同步 FFI 阻塞 UI | Isolate 异步封装 | UI 流畅度提升 |
| Provider 初始化问题 | AsyncNotifier | 代码更简洁，状态可靠 |
| 页面臃肿 | Clean Architecture 分层 | 可维护性提升 |
| 错误边界副作用 | Zone + PlatformDispatcher | 无副作用，更安全 |
| 缺少 Repository | 添加 Repository + UseCase | 可测试性提升 |
| 轮询效率低 | 事件驱动架构 | CPU/内存占用降低 |

### 代码量对比

- **原代码**：SearchPage 1000+ 行
- **新架构**：
  - workspaces_page.dart: ~150 行
  - workspace_list_view.dart: ~200 行
  - workspace_provider.dart: ~150 行
  - 其他组件拆分：各 ~100 行

### 建议

1. 优先迁移 FFI 调用到异步模式
2. 逐步替换 Provider 为 AsyncNotifier
3. 最后迁移页面组件
4. 保持测试覆盖

---

## 参考资源

- [fpdart 文档](https://pub.dev/packages/fpdart)
- [Riverpod 文档](https://riverpod.dev/)
- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Flutter Isolate 最佳实践](https://docs.flutter.dev/perf/isolates)
