/// 工作区仓库实现
/// 
/// 实现 Domain 层定义的接口
/// 使用 FFI 数据源进行实际操作

import 'dart:async';

import 'package:fpdart/fpdart.dart';

import '../../core/errors/app_error.dart';
import '../../domain/entities/workspace.dart';
import '../../domain/repositories/workspace_repository.dart';
import '../datasources/ffi_datasource.dart';
import '../datasources/event_datasource.dart';
import '../mappers/workspace_mapper.dart';

/// 工作区仓库实现
class WorkspaceRepositoryImpl implements WorkspaceRepository {
  final FfiDataSource _ffiDataSource;
  final EventDataSource _eventDataSource;

  // 缓存
  List<Workspace>? _cachedWorkspaces;
  DateTime? _lastCacheTime;
  static const _cacheDuration = Duration(seconds: 30);

  WorkspaceRepositoryImpl({
    FfiDataSource? ffiDataSource,
    EventDataSource? eventDataSource,
  })  : _ffiDataSource = ffiDataSource ?? FfiDataSource.instance,
        _eventDataSource = eventDataSource ?? EventDataSource.instance;

  @override
  AppTask<List<Workspace>> getWorkspaces() {
    return TaskEither(() async {
      try {
        // 检查缓存
        if (_isCacheValid) {
          return right(_cachedWorkspaces!);
        }

        // 调用 FFI
        final result = await _ffiDataSource.getWorkspaces().run();
        
        return result.fold(
          (error) => left(error),
          (ffiWorkspaces) {
            final workspaces = WorkspaceMapper.fromFfiList(ffiWorkspaces);
            _updateCache(workspaces);
            return right(workspaces);
          },
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '获取工作区失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<Workspace?> getWorkspaceById(String id) {
    return getWorkspaces().map((workspaces) {
      try {
        return workspaces.firstWhere((w) => w.id == id);
      } catch (_) {
        return null;
      }
    });
  }

  @override
  AppTask<Workspace?> getWorkspaceByPath(String path) {
    return getWorkspaces().map((workspaces) {
      try {
        return workspaces.firstWhere((w) => w.path == path);
      } catch (_) {
        return null;
      }
    });
  }

  @override
  AppTask<Workspace> createWorkspace(CreateWorkspaceParams params) {
    return TaskEither(() async {
      // 验证参数
      final validationError = params.validate();
      if (validationError != null) {
        return left(ValidationError(
          message: validationError,
        ));
      }

      try {
        final result = await _ffiDataSource.createWorkspace(
          name: params.name,
          path: params.path,
        ).run();

        return result.fold(
          (error) => left(error),
          (workspaceId) async {
            // 清除缓存
            _invalidateCache();
            
            // 返回新创建的工作区
            final workspace = Workspace(
              id: workspaceId,
              name: params.name,
              path: params.path,
              status: WorkspaceStatus.indexing,
              createdAt: DateTime.now(),
            );
            return right(workspace);
          },
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '创建工作区失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<void> deleteWorkspace(String id) {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.deleteWorkspace(id).run();
        
        return result.fold(
          (error) => left(error),
          (_) {
            _invalidateCache();
            return right(null);
          },
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '删除工作区失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<String> refreshWorkspace(RefreshWorkspaceParams params) {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.refreshWorkspace(
          params.workspaceId,
          params.path,
        ).run();

        return result.fold(
          (error) => left(error),
          (taskId) {
            _invalidateCache();
            return right(taskId);
          },
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '刷新工作区失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<Workspace> updateWorkspace(Workspace workspace) {
    // FFI 可能不直接支持更新，这里先刷新缓存
    return TaskEither(() async {
      _invalidateCache();
      return right(workspace);
    });
  }

  @override
  AppTask<WorkspaceStats> getWorkspaceStats(String id) {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.getWorkspaceStatus(id).run();

        return result.fold(
          (error) => left(error),
          (status) => right(WorkspaceStatsMapper.fromFfi(status)),
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '获取工作区统计失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  AppTask<void> startWatching(
    String workspaceId, {
    required List<String> paths,
    required bool recursive,
  }) {
    // FFI 可能不直接支持，使用原生实现
    return TaskEither(() async {
      // TODO: 实现文件监听
      return right(null);
    });
  }

  @override
  AppTask<void> stopWatching(String workspaceId) {
    return TaskEither(() async {
      // TODO: 实现停止文件监听
      return right(null);
    });
  }

  @override
  AppTask<bool> isWatching(String workspaceId) {
    return TaskEither(() async {
      // 从工作区状态获取
      final workspace = await getWorkspaceById(workspaceId).run();
      return workspace.fold(
        (error) => left(error),
        (w) => right(w?.isWatching ?? false),
      );
    });
  }

  @override
  AppTask<String> importFolder(String workspaceId, String path) {
    return TaskEither(() async {
      try {
        final result = await _ffiDataSource.importFolder(path, workspaceId).run();

        return result.fold(
          (error) => left(error),
          (taskId) => right(taskId),
        );
      } catch (e, stack) {
        return left(UnknownError(
          message: '导入文件夹失败',
          technicalDetails: e.toString(),
          cause: e,
        ));
      }
    });
  }

  @override
  Stream<List<Workspace>> watchWorkspaces() {
    // 使用事件流实现实时更新
    return _eventDataSource.rawEvents
        .where((event) => event.type == EventType.workspaceUpdated)
        .asyncMap((_) async {
          _invalidateCache();
          final result = await getWorkspaces().run();
          return result.getOrElse((_) => []);
        })
        .distinct();
  }

  // ==================== 缓存管理 ====================

  bool get _isCacheValid {
    if (_cachedWorkspaces == null || _lastCacheTime == null) {
      return false;
    }
    return DateTime.now().difference(_lastCacheTime!) < _cacheDuration;
  }

  void _updateCache(List<Workspace> workspaces) {
    _cachedWorkspaces = workspaces;
    _lastCacheTime = DateTime.now();
  }

  void _invalidateCache() {
    _cachedWorkspaces = null;
    _lastCacheTime = null;
  }
}
