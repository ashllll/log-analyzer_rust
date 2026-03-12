import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/common.dart';
import '../services/api_service.dart';
import '../../core/constants/app_constants.dart';
import 'app_provider.dart';

part 'workspace_provider.g.dart';

/// 用于存储最近打开时间的键前缀
const String _lastOpenedKeyPrefix = 'workspace_last_opened_';

/// 工作区状态 Provider
///
/// 对应 React 版本的 workspaceStore.ts
/// 管理工作区列表和状态
@riverpod
class WorkspaceState extends _$WorkspaceState {
  @override
  List<Workspace> build() {
    // 初始加载工作区（延迟执行避免在 build 中直接调用异步方法）
    Future.microtask(() => loadWorkspaces());
    return [];
  }

  /// 按最近打开时间排序工作区
  ///
  /// 最近打开的 3 个工作区显示在最前，其余按创建时间排序
  List<Workspace> _sortByRecentFirst(List<Workspace> workspaces) {
    // 分离有最近打开时间和无最近打开时间的工作区
    final withRecent = <Workspace>[];
    final withoutRecent = <Workspace>[];

    for (final workspace in workspaces) {
      if (workspace.lastOpenedAt != null) {
        withRecent.add(workspace);
      } else {
        withoutRecent.add(workspace);
      }
    }

    // 按最近打开时间降序排序
    withRecent.sort((a, b) {
      final aTime = a.lastOpenedAt ?? DateTime(1970);
      final bTime = b.lastOpenedAt ?? DateTime(1970);
      return bTime.compareTo(aTime);
    });

    // 按创建时间降序排序（最新的在前）
    withoutRecent.sort((a, b) {
      final aTime = a.createdAt ?? DateTime(1970);
      final bTime = b.createdAt ?? DateTime(1970);
      return bTime.compareTo(aTime);
    });

    // 最近打开的放最前面，最多 3 个
    final recentLimit = withRecent.length > 3 ? 3 : withRecent.length;
    return [...withRecent.take(recentLimit), ...withoutRecent];
  }

  /// 加载工作区列表
  ///
  /// 对应 React 版本的 loadWorkspaces()
  /// 从 Rust 后端获取所有工作区
  Future<void> loadWorkspaces() async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 检查 API 服务是否可用
      if (!apiService.isFfiAvailable) {
        debugPrint('WorkspaceState: FFI 桥接不可用，跳过加载');
        return;
      }

      // 调用后端 API 获取工作区列表
      final workspaces = await apiService.getWorkspaces();

      // 从本地存储加载最近打开时间并更新工作区
      final prefs = await SharedPreferences.getInstance();
      final updatedWorkspaces = workspaces.map((w) {
        final lastOpenedStr = prefs.getString('$_lastOpenedKeyPrefix${w.id}');
        if (lastOpenedStr != null) {
          final lastOpened = DateTime.tryParse(lastOpenedStr);
          if (lastOpened != null) {
            return w.copyWith(lastOpenedAt: lastOpened);
          }
        }
        return w;
      }).toList();

      // 按最近优先排序
      final sortedWorkspaces = _sortByRecentFirst(updatedWorkspaces);

      // 更新状态
      state = sortedWorkspaces;

      debugPrint('WorkspaceState: 已加载 ${sortedWorkspaces.length} 个工作区');
    } catch (e) {
      debugPrint('WorkspaceState: 加载工作区失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '加载工作区失败: $e');
    }
  }

  /// 创建新工作区
  ///
  /// 对应 React 版本的 createWorkspace()
  /// [name] 工作区名称
  /// [path] 工作区路径
  /// 返回创建的工作区 ID
  Future<String?> createWorkspace({
    required String name,
    required String path,
  }) async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 调用后端 API 创建工作区
      final workspaceId = await apiService.createWorkspace(
        name: name,
        path: path,
      );

      // 设置创建时间和最近打开时间
      final now = DateTime.now();
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(
        '$_lastOpenedKeyPrefix$workspaceId',
        now.toIso8601String(),
      );

      // 重新加载工作区列表
      await loadWorkspaces();

      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.success, '工作区 "$name" 创建成功');

      return workspaceId;
    } catch (e) {
      debugPrint('WorkspaceState: 创建工作区失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '创建工作区失败: $e');
      return null;
    }
  }

  /// 删除工作区
  ///
  /// 对应 React 版本的 deleteWorkspace()
  /// [id] 工作区 ID
  Future<bool> deleteWorkspace(String id) async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 先从本地状态移除（乐观更新）
      state = state.where((w) => w.id != id).toList();

      // 调用后端 API 删除工作区
      await apiService.deleteWorkspace(id);

      ref.read(appStateProvider.notifier).addToast(ToastType.success, '工作区已删除');

      return true;
    } catch (e) {
      debugPrint('WorkspaceState: 删除工作区失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '删除工作区失败: $e');
      return false;
    }
  }

  /// 刷新工作区
  ///
  /// 对应 React 版本的 refreshWorkspace()
  /// 触发后端重新扫描工作区文件
  /// 返回任务 ID 用于跟踪进度
  Future<String?> refreshWorkspace(String id, String path) async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 调用后端 API 刷新工作区
      final taskId = await apiService.refreshWorkspace(id, path);

      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.info, '正在刷新工作区...');

      return taskId;
    } catch (e) {
      debugPrint('WorkspaceState: 刷新工作区失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '刷新工作区失败: $e');
      return null;
    }
  }

  /// 加载工作区（切换活动工作区）
  ///
  /// 对应 React 版本的 loadWorkspace()
  /// 加载指定工作区的索引数据
  Future<bool> loadWorkspaceById(String workspaceId) async {
    try {
      final apiService = ref.read(apiServiceProvider);

      // 调用后端 API 加载工作区
      final response = await apiService.loadWorkspace(workspaceId);

      // 更新活动工作区
      ref.read(appStateProvider.notifier).setActiveWorkspace(workspaceId);

      // 更新最近打开时间
      final now = DateTime.now();
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(
        '$_lastOpenedKeyPrefix$workspaceId',
        now.toIso8601String(),
      );

      // 更新本地状态中对应工作区的信息
      state = state.map((w) {
        if (w.id == workspaceId) {
          return w.copyWith(
            status: WorkspaceStatusData(value: response.status),
            files: response.fileCount,
            size: response.totalSize,
            lastOpenedAt: now,
          );
        }
        return w;
      }).toList();

      // 重新排序（最近打开的移到最后面）
      state = _sortByRecentFirst(state);

      debugPrint(
        'WorkspaceState: 工作区 $workspaceId 加载成功，状态: ${response.status}',
      );
      return true;
    } catch (e) {
      debugPrint('WorkspaceState: 加载工作区失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '加载工作区失败: $e');
      return false;
    }
  }

  /// 获取工作区状态
  ///
  /// 从后端获取指定工作区的最新状态
  Future<WorkspaceStatusData?> getWorkspaceStatus(String workspaceId) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      final response = await apiService.getWorkspaceStatus(workspaceId);

      // 更新本地状态
      state = state.map((w) {
        if (w.id == workspaceId) {
          return w.copyWith(
            status: WorkspaceStatusData(value: response.status),
          );
        }
        return w;
      }).toList();

      return WorkspaceStatusData(value: response.status);
    } catch (e) {
      debugPrint('WorkspaceState: 获取工作区状态失败: $e');
      return null;
    }
  }

  /// 添加工作区（本地操作，不调用后端）
  ///
  /// 用于接收后端事件通知时更新状态
  void addWorkspace(Workspace workspace) {
    // 检查是否已存在
    final exists = state.any((w) => w.id == workspace.id);
    if (!exists) {
      state = [...state, workspace];
    }
  }

  /// 更新工作区（本地操作）
  ///
  /// 用于接收后端事件通知时更新状态
  void updateWorkspace(String id, Workspace updated) {
    state = state.map((w) => w.id == id ? updated : w).toList();
  }

  /// 移除工作区（本地操作）
  ///
  /// 用于接收后端事件通知时更新状态
  void removeWorkspace(String id) {
    state = state.where((w) => w.id != id).toList();
  }

  /// 根据 ID 获取工作区
  Workspace? getWorkspaceById(String id) {
    try {
      return state.firstWhere((w) => w.id == id);
    } catch (e) {
      return null;
    }
  }

  /// 获取活动工作区
  Workspace? get activeWorkspace {
    final activeId = ref.read(appStateProvider).activeWorkspaceId;
    if (activeId == null) return null;
    return getWorkspaceById(activeId);
  }
}

/// 工作区轮询 Stream Provider
///
/// 使用 Riverpod StreamProvider 替代 Timer.periodic
/// 每5秒检查一次工作区状态，如果发现有工作区正在处理则刷新列表
final workspacePollingProvider = StreamProvider<void>((ref) {
  return Stream.periodic(
    const Duration(seconds: 5),
    (count) => count, // 发出事件计数，用于触发轮询
  ).asyncMap((_) async {
    final workspaces = ref.read(workspaceStateProvider);
    if (workspaces.isEmpty) return;

    // 检查是否有正在处理的工作区
    final hasProcessing = workspaces.any(
      (w) =>
          w.status.value == 'SCANNING' ||
          w.status.value == 'PROCESSING' ||
          w.status.value == 'INDEXING',
    );

    if (!hasProcessing) return;

    // 刷新工作区列表以获取最新状态
    try {
      await ref.read(workspaceStateProvider.notifier).loadWorkspaces();
    } catch (e) {
      debugPrint('Workspace polling error: $e');
    }
  });
});
