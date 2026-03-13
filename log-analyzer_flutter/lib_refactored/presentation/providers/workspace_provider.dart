/// 工作区状态管理
/// 
/// 使用 Riverpod AsyncNotifier 实现异步状态管理
/// 替代传统的 Future.microtask 初始化模式

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../../core/errors/app_error.dart';
import '../../domain/entities/workspace.dart';
import '../../domain/repositories/workspace_repository.dart';
import '../../data/repositories/workspace_repository_impl.dart';

part 'workspace_provider.g.dart';

// ==================== Repository Provider ====================

/// 工作区仓库 Provider
/// 
/// 使用单例模式
@Riverpod(keepAlive: true)
WorkspaceRepository workspaceRepository(Ref ref) {
  return WorkspaceRepositoryImpl();
}

// ==================== AsyncNotifier Providers ====================

/// 工作区列表状态
/// 
/// 使用 AsyncNotifier 自动处理加载、错误状态
@riverpod
class WorkspaceList extends _$WorkspaceList {
  @override
  Future<List<Workspace>> build() async {
    // 自动获取仓库并加载数据
    final repository = ref.read(workspaceRepositoryProvider);
    
    // 监听仓库事件流实现实时更新
    _listenToWorkspaceUpdates();
    
    // 初始加载
    final result = await repository.getWorkspaces().run();
    return result.fold(
      (error) => throw error, // AsyncValue 会自动捕获为 error 状态
      (workspaces) => workspaces,
    );
  }

  /// 刷新工作区列表
  Future<void> refresh() async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      final repository = ref.read(workspaceRepositoryProvider);
      final result = await repository.getWorkspaces().run();
      return result.getOrElse((_) => []);
    });
  }

  /// 创建工作区
  Future<void> createWorkspace(CreateWorkspaceParams params) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      final repository = ref.read(workspaceRepositoryProvider);
      final result = await repository.createWorkspace(params).run();
      
      return result.fold(
        (error) => throw error,
        (newWorkspace) async {
          // 重新加载列表
          final listResult = await repository.getWorkspaces().run();
          return listResult.getOrElse((_) => []);
        },
      );
    });
  }

  /// 删除工作区
  Future<void> deleteWorkspace(String id) async {
    final repository = ref.read(workspaceRepositoryProvider);
    final result = await repository.deleteWorkspace(id).run();
    
    result.fold(
      (error) => throw error,
      (_) => refresh(),
    );
  }

  /// 监听工作区更新
  void _listenToWorkspaceUpdates() {
    ref.listen(workspaceRepositoryProvider, (previous, next) {
      // 可以在这里处理仓库变化
    });
    
    // 定期刷新（可选）
    // ref.onDispose(() {});
  }
}

/// 当前选中的工作区
@riverpod
class SelectedWorkspace extends _$SelectedWorkspace {
  @override
  Workspace? build() {
    // 初始状态为 null
    return null;
  }

  void select(Workspace? workspace) {
    state = workspace;
  }

  void selectById(String id) {
    final workspacesAsync = ref.read(workspaceListProvider);
    workspacesAsync.whenData((workspaces) {
      state = workspaces.where((w) => w.id == id).firstOrNull;
    });
  }
}

/// 工作区统计状态
@riverpod
class WorkspaceStatsNotifier extends _$WorkspaceStatsNotifier {
  @override
  Future<WorkspaceStats> build(String workspaceId) async {
    final repository = ref.read(workspaceRepositoryProvider);
    final result = await repository.getWorkspaceStats(workspaceId).run();
    
    return result.fold(
      (error) => throw error,
      (stats) => stats,
    );
  }

  Future<void> refresh() async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      final repository = ref.read(workspaceRepositoryProvider);
      final result = await repository.getWorkspaceStats(
        (state.value as WorkspaceStats?)?.toString() ?? '',
      ).run();
      return result.getOrElse((_) => WorkspaceStats.empty);
    });
  }
}

/// 工作区刷新状态
/// 
/// 用于跟踪刷新任务的状态
@riverpod
class WorkspaceRefreshState extends _$WorkspaceRefreshState {
  @override
  AsyncValue<String?> build() {
    // 初始状态为 null（没有正在进行的刷新）
    return const AsyncValue.data(null);
  }

  Future<void> refresh(String workspaceId, String path) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      final repository = ref.read(workspaceRepositoryProvider);
      final result = await repository.refreshWorkspace(
        RefreshWorkspaceParams(workspaceId: workspaceId, path: path),
      ).run();
      
      return result.fold(
        (error) => throw error,
        (taskId) => taskId,
      );
    });
  }
}

// ==================== 派生状态 ====================

/// 工作区数量
@riverpod
int workspaceCount(Ref ref) {
  final workspacesAsync = ref.watch(workspaceListProvider);
  return workspacesAsync.when(
    data: (workspaces) => workspaces.length,
    loading: () => 0,
    error: (_, __) => 0,
  );
}

/// 是否有工作区
@riverpod
bool hasWorkspaces(Ref ref) {
  return ref.watch(workspaceCountProvider) > 0;
}

/// 就绪的工作区
@riverpod
List<Workspace> readyWorkspaces(Ref ref) {
  final workspacesAsync = ref.watch(workspaceListProvider);
  return workspacesAsync.when(
    data: (workspaces) => workspaces.where((w) => w.isReady).toList(),
    loading: () => [],
    error: (_, __) => [],
  );
}

/// 忙状态的工作区
@riverpod
List<Workspace> busyWorkspaces(Ref ref) {
  final workspacesAsync = ref.watch(workspaceListProvider);
  return workspacesAsync.when(
    data: (workspaces) => workspaces.where((w) => w.isBusy).toList(),
    loading: () => [],
    error: (_, __) => [],
  );
}
