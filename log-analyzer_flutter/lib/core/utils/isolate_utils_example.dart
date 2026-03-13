/// 异步 FFI 封装使用示例
///
/// 本文件展示了如何在应用中使用新的异步 FFI 封装
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:fpdart/fpdart.dart';

import 'isolate_utils.dart';
import '../../data/datasources/ffi_datasource.dart';
import '../../shared/services/ffi_service_async.dart';
import '../../shared/services/generated/ffi/types.dart' as ffi_types;

// ==================== 基本使用示例 ====================

/// 示例 1: 使用 AsyncFfiCall 执行简单查询
Future<void> example1_BasicUsage() async {
  // 创建工作区
  final result = await AsyncFfiCall.query<String>(
    query: () async {
      final service = FfiServiceAsync.instance;
      return service.createWorkspace('my_workspace', '/path/to/logs');
    },
  );

  // 处理结果
  result.when(
    success: (workspaceId) {
      debugPrint('工作区创建成功: $workspaceId');
    },
    error: (error) {
      debugPrint('工作区创建失败: $error');
    },
  );
}

/// 示例 2: 使用函数式风格（fpdart）
Future<void> example2_FunctionalStyle() async {
  final task = AsyncFfiCallFunctional.query<List<ffi_types.WorkspaceData>>(
    query: () async {
      final service = FfiServiceAsync.instance;
      return service.getWorkspaces();
    },
  );

  // 执行并处理结果
  final result = await task.run();
  result.match(
    (error) => debugPrint('获取失败: $error'),
    (workspaces) => debugPrint('获取到 ${workspaces.length} 个工作区'),
  );
}

/// 示例 3: 使用 FfiDataSource 抽象
Future<void> example3_DataSourceUsage() async {
  final dataSource = FfiDataSourceFactory.create();

  final result = await dataSource.queryList<ffi_types.WorkspaceData>(
    () async => FfiServiceAsync.instance.getWorkspaces(),
  );

  final workspaces = result.getOrElse([]);
  debugPrint('工作区数量: ${workspaces.length}');
}

// ==================== Riverpod Provider 示例 ====================

/// FFI 数据源 Provider
final ffiDataSourceProvider = Provider<FfiDataSource>(
  (ref) => FfiDataSourceFactory.create(),
);

/// 工作区列表 Provider（使用异步 FFI）
final workspaceListProvider = FutureProvider<List<ffi_types.WorkspaceData>>(
  (ref) async {
    final dataSource = ref.watch(ffiDataSourceProvider);

    final result = await dataSource.queryList<ffi_types.WorkspaceData>(
      () async => FfiServiceAsync.instance.getWorkspaces(),
    );

    return result.getOrThrow();
  },
);

/// 工作区创建 Notifier
class WorkspaceCreateNotifier extends AsyncNotifier<String> {
  @override
  Future<String> build() async {
    // 初始状态
    return '';
  }

  Future<void> createWorkspace(String name, String path) async {
    state = const AsyncValue.loading();

    final result = await AsyncFfiCall.query<String>(
      query: () => FfiServiceAsync.instance.createWorkspace(name, path),
    );

    state = result.when(
      success: (id) => AsyncValue.data(id),
      error: (error) => AsyncValue.error(error, StackTrace.current),
    );
  }
}

final workspaceCreateProvider =
    AsyncNotifierProvider<WorkspaceCreateNotifier, String>(
  WorkspaceCreateNotifier.new,
);

// ==================== Widget 使用示例 ====================

/// 工作区列表页面
class WorkspacesPage extends ConsumerWidget {
  const WorkspacesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final workspacesAsync = ref.watch(workspaceListProvider);

    return workspacesAsync.when(
      data: (workspaces) => WorkspacesList(workspaces: workspaces),
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (error, stack) => ErrorView(error: error.toString()),
    );
  }
}

/// 工作区列表组件
class WorkspacesList extends StatelessWidget {
  final List<ffi_types.WorkspaceData> workspaces;

  const WorkspacesList({super.key, required this.workspaces});

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: workspaces.length,
      itemBuilder: (context, index) {
        final workspace = workspaces[index];
        return ListTile(
          title: Text(workspace.name),
          subtitle: Text(workspace.path),
        );
      },
    );
  }
}

/// 错误视图
class ErrorView extends StatelessWidget {
  final String error;

  const ErrorView({super.key, required this.error});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(Icons.error_outline, color: Colors.red, size: 48),
          const SizedBox(height: 16),
          Text('发生错误: $error', style: const TextStyle(color: Colors.red)),
        ],
      ),
    );
  }
}

// ==================== 高级用法示例 ====================

/// 示例 4: 并行执行多个 FFI 调用
Future<void> example4_ParallelCalls() async {
  final calls = [
    AsyncFfiCall.query<ffi_types.TaskMetricsData>(
      query: () => FfiServiceAsync.instance.getTaskMetrics(),
    ),
    AsyncFfiCall.queryList<ffi_types.WorkspaceData>(
      query: () => FfiServiceAsync.instance.getWorkspaces(),
    ),
    AsyncFfiCall.query<ffi_types.ConfigData>(
      query: () => FfiServiceAsync.instance.loadConfig(),
    ),
  ];

  final results = await IsolateUtils.parallelCalls<dynamic>(calls);

  for (final result in results) {
    result.when(
      success: (data) => debugPrint('成功: $data'),
      error: (error) => debugPrint('失败: $error'),
    );
  }
}

/// 示例 5: 带重试的 FFI 调用
Future<void> example5_RetryableCall() async {
  final result = await IsolateUtils.retryableCall<String>(
    call: () => FfiServiceAsync.instance.createWorkspace('name', '/path'),
    maxRetries: 3,
    retryDelay: const Duration(milliseconds: 200),
  );

  result.when(
    success: (id) => debugPrint('创建成功: $id'),
    error: (error) => debugPrint('创建失败: $error'),
  );
}

/// 示例 6: 性能监控
Future<void> example6_PerformanceMonitoring() async {
  final result = await FfiPerformanceMonitor.monitor(
    'getWorkspaces',
    () => FfiServiceAsync.instance.getWorkspaces(),
  );

  // 获取平均执行时间
  final avgTime = FfiPerformanceMonitor.getAverageTime('getWorkspaces');
  debugPrint('平均执行时间: ${avgTime?.inMilliseconds}ms');

  // 获取所有指标
  final allMetrics = FfiPerformanceMonitor.getAllAverages();
  for (final entry in allMetrics.entries) {
    debugPrint('${entry.key}: ${entry.value.inMilliseconds}ms');
  }
}

/// 示例 7: 函数式组合
Future<void> example7_FunctionalComposition() async {
  final dataSource = FfiDataSourceFactory.createFunctional();

  // 创建任务链
  final task = dataSource
      .queryTask<List<ffi_types.WorkspaceData>>(
        () => FfiServiceAsync.instance.getWorkspaces(),
      )
      .map((workspaces) => workspaces.length)
      .flatMap((count) => TaskEither<String, String>(() async {
            // 基于工作区数量执行另一个操作
            return Right('工作区数量: $count');
          }));

  final result = await task.run();
  result.match(
    (error) => debugPrint('错误: $error'),
    (message) => debugPrint(message),
  );
}

/// 示例 8: 带自定义超时的调用
Future<void> example8_CustomTimeout() async {
  // 长时间运行的操作，设置 60 秒超时
  final result = await AsyncFfiCall.query(
    query: () => FfiServiceAsync.instance.importFolder('/path', 'workspaceId'),
    timeout: const Duration(seconds: 60),
  );

  result.when(
    success: (taskId) => debugPrint('导入任务: $taskId'),
    error: (error) => debugPrint('导入失败: $error'),
  );
}

/// 示例 9: 搜索操作封装
class SearchService {
  final FfiDataSource _dataSource;

  SearchService(this._dataSource);

  Future<FfiResult<List<ffi_types.FfiSearchResultEntry>>> searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 1000,
  }) async {
    return _dataSource.queryList<ffi_types.FfiSearchResultEntry>(
      () => FfiServiceAsync.instance.searchStructured(
        FfiServiceAsync.instance.buildSearchQuery(
          keywords: [query],
          globalOperator: 'AND',
        ),
        workspaceId: workspaceId,
        maxResults: maxResults,
      ),
    );
  }

  Future<FfiResult<bool>> cancelSearch(String searchId) async {
    return _dataSource.query<bool>(
      () => FfiServiceAsync.instance.cancelSearch(searchId),
    );
  }
}

// ==================== 搜索 Provider 示例 ====================

/// 搜索服务 Provider
final searchServiceProvider = Provider<SearchService>(
  (ref) => SearchService(ref.watch(ffiDataSourceProvider)),
);

/// 搜索结果 StateNotifier
class SearchResultsNotifier
    extends StateNotifier<AsyncValue<List<ffi_types.FfiSearchResultEntry>>> {
  final SearchService _searchService;

  SearchResultsNotifier(this._searchService)
      : super(const AsyncValue.data([]));

  Future<void> search({
    required String query,
    String? workspaceId,
  }) async {
    state = const AsyncValue.loading();

    final result = await _searchService.searchLogs(
      query: query,
      workspaceId: workspaceId,
    );

    state = result.when(
      success: (data) => AsyncValue.data(data),
      error: (error) => AsyncValue.error(Exception(error), StackTrace.current),
    );
  }
}

final searchResultsProvider = StateNotifierProvider<SearchResultsNotifier,
    AsyncValue<List<ffi_types.FfiSearchResultEntry>>>(
  (ref) => SearchResultsNotifier(ref.watch(searchServiceProvider)),
);
