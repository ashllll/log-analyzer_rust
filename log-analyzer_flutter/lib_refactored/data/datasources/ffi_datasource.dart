/// FFI 数据源
/// 
/// 封装所有 FFI 调用，提供统一的异步接口
/// 所有同步 FFI 调用都在 Isolate 中执行

import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated_io.dart';
import 'package:fpdart/fpdart.dart';

import '../../core/errors/app_error.dart';
import '../../core/utils/isolate_utils.dart';
import '../../shared/services/generated/frb_generated.dart';
import '../../shared/services/generated/ffi/bridge.dart' as ffi;
import '../../shared/services/generated/ffi/types.dart' as ffi_types;

/// FFI 数据源
/// 
/// 单例模式，管理 FFI 桥接的生命周期
class FfiDataSource {
  static FfiDataSource? _instance;
  static bool _isInitialized = false;
  static String? _initError;

  FfiDataSource._();

  static FfiDataSource get instance {
    _instance ??= FfiDataSource._();
    return _instance!;
  }

  /// 是否已初始化
  bool get isInitialized => _isInitialized;

  /// 初始化错误信息
  String? get initError => _initError;

  /// 初始化 FFI
  /// 
  /// 必须在应用启动时调用一次
  Future<void> initialize() async {
    if (_isInitialized) return;

    try {
      final externalLibrary = await _resolveExternalLibrary();
      await LogAnalyzerBridge.init(externalLibrary: externalLibrary);
      _isInitialized = true;
      debugPrint('FFI 初始化成功');
    } catch (e, stack) {
      _initError = e.toString();
      debugPrint('FFI 初始化失败: $e');
      throw FfiError.initialization(details: e.toString(), cause: e);
    }
  }

  /// 解析外部库路径
  Future<ExternalLibrary> _resolveExternalLibrary() async {
    if (!Platform.isMacOS && !Platform.isLinux && !Platform.isWindows) {
      throw UnsupportedError('不支持的平台');
    }

    final paths = _getLibrarySearchPaths();

    for (final path in paths) {
      final file = File(path);
      if (await file.exists()) {
        debugPrint('加载 Rust 库: $path');
        return ExternalLibrary.open(path);
      }
    }

    throw FileSystemException(
      '找不到 Rust 库，搜索路径:\n${paths.join('\n')}',
    );
  }

  /// 获取库搜索路径
  List<String> _getLibrarySearchPaths() {
    final scriptDir = Platform.script.toFilePath();
    final projectRoot = _findProjectRoot(scriptDir);
    final paths = <String>[];

    if (Platform.isMacOS) {
      const libName = 'liblog_analyzer.dylib';
      paths.addAll([
        '../log-analyzer/src-tauri/target/release/$libName',
        '../log-analyzer/src-tauri/target/debug/$libName',
        if (projectRoot != null) ...[
          '$projectRoot/../log-analyzer/src-tauri/target/release/$libName',
          '$projectRoot/../log-analyzer/src-tauri/target/debug/$libName',
        ],
      ]);
    } else if (Platform.isLinux) {
      const libName = 'liblog_analyzer.so';
      paths.addAll([
        '../log-analyzer/src-tauri/target/release/$libName',
        '../log-analyzer/src-tauri/target/debug/$libName',
      ]);
    } else if (Platform.isWindows) {
      const libName = 'log_analyzer.dll';
      paths.addAll([
        '../log-analyzer/src-tauri/target/release/$libName',
        '../log-analyzer/src-tauri/target/debug/$libName',
      ]);
    }

    return paths;
  }

  String? _findProjectRoot(String startPath) {
    var dir = File(startPath).parent;
    for (var i = 0; i < 10; i++) {
      final pubspec = File('${dir.path}/pubspec.yaml');
      if (pubspec.existsSync()) {
        return dir.path;
      }
      final parent = dir.parent;
      if (parent.path == dir.path) break;
      dir = parent;
    }
    return null;
  }

  // ==================== 健康检查 ====================

  /// 健康检查
  AppTask<String> healthCheck() {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(call: ffi.healthCheck);
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'healthCheck',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  // ==================== 工作区操作 ====================

  /// 获取工作区列表（异步）
  AppTask<List<ffi_types.WorkspaceData>> getWorkspaces() {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.queryList(
          query: ffi.getWorkspaces,
          timeout: const Duration(seconds: 10),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'getWorkspaces',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 创建工作区
  AppTask<String> createWorkspace({
    required String name,
    required String path,
  }) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.createWorkspace(name: name, path: path),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'createWorkspace',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 删除工作区
  AppTask<bool> deleteWorkspace(String workspaceId) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.deleteWorkspace(workspaceId: workspaceId),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'deleteWorkspace',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 刷新工作区
  AppTask<String> refreshWorkspace(String workspaceId, String path) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.refreshWorkspace(workspaceId: workspaceId, path: path),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'refreshWorkspace',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 获取工作区状态
  AppTask<ffi_types.WorkspaceStatusData?> getWorkspaceStatus(String workspaceId) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.querySingle(
          query: () => ffi.getWorkspaceStatus(workspaceId: workspaceId),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'getWorkspaceStatus',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  // ==================== 搜索操作 ====================

  /// 执行搜索
  AppTask<String> searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    String? filters,
  }) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.searchLogs(
            query: query,
            workspaceId: workspaceId,
            maxResults: maxResults,
            filters: filters,
          ),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'searchLogs',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 取消搜索
  AppTask<bool> cancelSearch(String searchId) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.cancelSearch(searchId: searchId),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'cancelSearch',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 执行正则搜索
  AppTask<List<ffi_types.FfiSearchResultEntry>> searchRegex({
    required String pattern,
    String? workspaceId,
    int maxResults = 10000,
    bool caseSensitive = false,
  }) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.queryList(
          query: () => ffi.searchRegex(
            pattern: pattern,
            workspaceId: workspaceId,
            maxResults: maxResults,
            caseSensitive: caseSensitive,
          ),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'searchRegex',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 验证正则表达式
  AppTask<ffi_types.RegexValidationResult> validateRegex(String pattern) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.validateRegex(pattern: pattern),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'validateRegex',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  // ==================== 任务操作 ====================

  /// 获取任务指标
  AppTask<ffi_types.TaskMetricsData?> getTaskMetrics() {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.querySingle(
          query: ffi.getTaskMetrics,
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'getTaskMetrics',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  /// 取消任务
  AppTask<bool> cancelTask(String taskId) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.cancelTask(taskId: taskId),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'cancelTask',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  // ==================== 导入操作 ====================

  /// 导入文件夹
  AppTask<String> importFolder(String path, String workspaceId) {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.execute(
          call: () => ffi.importFolder(path: path, workspaceId: workspaceId),
        );
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'importFolder',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }

  // ==================== 关键词操作 ====================

  /// 获取关键词列表
  AppTask<List<ffi_types.FfiKeywordGroupData>> getKeywords() {
    return TaskEither(() async {
      if (!_isInitialized) {
        return left(FfiError.initialization());
      }
      try {
        final result = await AsyncFfiCall.queryList(query: ffi.getKeywords);
        return right(result);
      } catch (e, stack) {
        return left(FfiError.call(
          method: 'getKeywords',
          details: e.toString(),
          cause: e,
        ));
      }
    });
  }
}
