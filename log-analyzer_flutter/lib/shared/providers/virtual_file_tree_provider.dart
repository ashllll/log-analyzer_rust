import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/bridge_service.dart';
import '../services/generated/ffi/types.dart' as ffi_types;

part 'virtual_file_tree_provider.freezed.dart';
part 'virtual_file_tree_provider.g.dart';

// ==================== 虚拟文件树数据模型 ====================

/// 虚拟文件树节点类型
enum VirtualTreeNodeType {
  file,
  archive,
}

/// 虚拟文件树节点数据
///
/// 用于 Flutter 端虚拟文件树展示
/// 对应 Rust 后端的 VirtualTreeNodeData 枚举
@freezed
sealed class VirtualTreeNode with _$VirtualTreeNode {
  /// 文件节点
  const factory VirtualTreeNode.file({
    required String name,
    required String path,
    required String hash,
    required int size,
    @Default(null) String? mimeType,
  }) = VirtualTreeNodeFile;

  /// 归档节点（压缩包）
  const factory VirtualTreeNode.archive({
    required String name,
    required String path,
    required String hash,
    required String archiveType,
    @Default([]) List<VirtualTreeNode> children,
  }) = VirtualTreeNodeArchive;

  factory VirtualTreeNode.fromJson(Map<String, dynamic> json) =>
      _$VirtualTreeNodeFromJson(json);
}

/// 将 FFI VirtualTreeNodeData 转换为 Dart VirtualTreeNode
///
/// 递归处理嵌套的子节点
VirtualTreeNode _convertFromFfiNode(ffi_types.VirtualTreeNodeData node) {
  return switch (node) {
    ffi_types.VirtualTreeNodeData_File(:final name, :final path, :final hash, :final size, :final mimeType) =>
      VirtualTreeNode.file(
        name: name,
        path: path,
        hash: hash,
        size: size.toInt(),  // PlatformInt64 to int
        mimeType: mimeType,
      ),
    ffi_types.VirtualTreeNodeData_Archive(:final name, :final path, :final hash, :final archiveType, :final children) =>
      VirtualTreeNode.archive(
        name: name,
        path: path,
        hash: hash,
        archiveType: archiveType,
        children: children.map(_convertFromFfiNode).toList(),  // 递归转换
      ),
  };
}

/// 批量转换 FFI 节点列表
List<VirtualTreeNode> _convertFromFfiNodes(List<ffi_types.VirtualTreeNodeData> nodes) {
  return nodes.map(_convertFromFfiNode).toList();
}

/// 文件内容响应数据
///
/// 用于通过哈希读取文件内容
@Freezed(makeCollectionsUnmodifiable: false)
sealed class FileContentResponse with _$FileContentResponse {
  const factory FileContentResponse({
    required String content,
    required String hash,
    required int size,
  }) = FileContentResponseData;

  factory FileContentResponse.fromJson(Map<String, dynamic> json) =>
      _$FileContentResponseFromJson(json);
}

// ==================== Provider 定义 ====================

/// BridgeService Provider
///
/// 提供 BridgeService 单例实例
@riverpod
BridgeService bridgeService(Ref ref) {
  return BridgeService.instance;
}

/// 虚拟文件树 Provider
///
/// 使用 Riverpod 3.0 AsyncNotifier 管理虚拟文件树状态
/// 支持懒加载子节点和工作区切换
///
/// 特性:
/// - 根据 workspaceId 加载虚拟文件树根节点
/// - 支持懒加载子节点（展开目录时从后端加载）
/// - 切换工作区时文件树自动刷新
/// - FFI 调用失败时返回空列表
@riverpod
class VirtualFileTree extends _$VirtualFileTree {
  @override
  Future<List<VirtualTreeNode>> build(String workspaceId) async {
    // 懒加载：首次 watch 时调用
    final bridge = ref.watch(bridgeServiceProvider);

    // 检查 FFI 是否可用
    if (!bridge.isFfiEnabled) {
      debugPrint('VirtualFileTreeProvider: FFI 桥接不可用，返回空树');
      return [];
    }

    try {
      // 加载根节点
      return await _loadTreeFromBackend(bridge, workspaceId);
    } catch (e) {
      debugPrint('VirtualFileTreeProvider: 加载文件树失败: $e');
      // 错误时返回空列表
      return [];
    }
  }

  /// 从后端加载文件树
  Future<List<VirtualTreeNode>> _loadTreeFromBackend(
    BridgeService bridge,
    String workspaceId,
  ) async {
    try {
      // 调用后端获取虚拟文件树
      // 注意：由于 FFI 类型生成问题，这里使用动态类型处理
      // 当 FFI 类型可用时，可以直接使用 bridge.getVirtualFileTree()
      final result = await _getVirtualFileTreeViaBridge(bridge, workspaceId);
      return result;
    } catch (e) {
      debugPrint('_loadTreeFromBackend error: $e');
      return [];
    }
  }

  /// 通过 BridgeService 获取虚拟文件树
  ///
  /// 调用 FFI 获取文件树并转换为 Dart 模型
  Future<List<VirtualTreeNode>> _getVirtualFileTreeViaBridge(
    BridgeService bridge,
    String workspaceId,
  ) async {
    try {
      debugPrint('VirtualFileTreeProvider: 调用 getVirtualFileTree($workspaceId)');

      // 调用 FFI 获取文件树
      final ffiNodes = await bridge.getVirtualFileTree(workspaceId);

      // 转换为 Dart 模型
      return _convertFromFfiNodes(ffiNodes);
    } catch (e) {
      debugPrint('_getVirtualFileTreeViaBridge error: $e');
      return [];
    }
  }

  /// 懒加载子节点
  ///
  /// 展开目录时从后端加载子节点
  ///
  /// # 参数
  ///
  /// * `parentPath` - 父节点路径
  ///
  /// # 返回
  ///
  /// 加载的子节点列表
  Future<List<VirtualTreeNode>> loadChildren(String parentPath) async {
    final currentValue = state.value;
    if (currentValue == null) {
      return [];
    }
    final currentTree = currentValue;
    if (currentTree.isEmpty) {
      return [];
    }

    final bridge = ref.read(bridgeServiceProvider);
    if (!bridge.isFfiEnabled) {
      debugPrint('loadChildren: FFI 桥接不可用');
      return [];
    }

    try {
      debugPrint('VirtualFileTreeProvider: 加载子节点 $parentPath');

      // 调用后端加载子节点
      final children = await _getTreeChildrenViaBridge(
        bridge,
        workspaceId,
        parentPath,
      );

      if (children.isEmpty) {
        return [];
      }

      // 更新树中对应节点的子节点
      final updatedTree = _updateNodeChildren(currentTree, parentPath, children);
      state = AsyncData(updatedTree);

      return children;
    } catch (e) {
      debugPrint('loadChildren error: $e');
      return [];
    }
  }

  /// 通过 BridgeService 获取子节点
  ///
  /// 调用 FFI 获取子节点并转换为 Dart 模型
  Future<List<VirtualTreeNode>> _getTreeChildrenViaBridge(
    BridgeService bridge,
    String workspaceId,
    String parentPath,
  ) async {
    try {
      debugPrint(
        'VirtualFileTreeProvider: 调用 getTreeChildren($workspaceId, $parentPath)',
      );

      // 调用 FFI 获取子节点
      final ffiChildren = await bridge.getTreeChildren(
        workspaceId: workspaceId,
        parentPath: parentPath,
      );

      // 转换为 Dart 模型
      return _convertFromFfiNodes(ffiChildren);
    } catch (e) {
      debugPrint('_getTreeChildrenViaBridge error: $e');
      return [];
    }
  }

  /// 更新树中指定节点的子节点
  ///
  /// 递归查找并更新节点，保持不可变性
  List<VirtualTreeNode> _updateNodeChildren(
    List<VirtualTreeNode> nodes,
    String targetPath,
    List<VirtualTreeNode> newChildren,
  ) {
    return nodes.map((node) {
      return switch (node) {
        VirtualTreeNodeFile() => node,
        VirtualTreeNodeArchive(:final name, :final path, :final hash, :final archiveType, :final children) =>
          // 如果路径匹配，更新子节点
          targetPath == path
              ? VirtualTreeNode.archive(
                  name: name,
                  path: path,
                  hash: hash,
                  archiveType: archiveType,
                  children: newChildren,
                )
              : // 否则递归处理子节点
              VirtualTreeNode.archive(
                  name: name,
                  path: path,
                  hash: hash,
                  archiveType: archiveType,
                  children: _updateNodeChildren(children, targetPath, newChildren),
                ),
      };
    }).toList();
  }

  /// 刷新文件树
  ///
  /// 重新从后端加载文件树
  Future<void> refresh() async {
    state = const AsyncLoading();

    final bridge = ref.read(bridgeServiceProvider);
    if (!bridge.isFfiEnabled) {
      state = const AsyncData([]);
      return;
    }

    try {
      final tree = await _loadTreeFromBackend(bridge, workspaceId);
      state = AsyncData(tree);
    } catch (e) {
      debugPrint('refresh error: $e');
      state = AsyncError(e, StackTrace.current);
    }
  }

  /// 读取文件内容
  ///
  /// 通过哈希从 CAS 存储读取文件内容
  ///
  /// # 参数
  ///
  /// * `hash` - 文件 SHA-256 哈希
  ///
  /// # 返回
  ///
  /// 文件内容响应
  Future<FileContentResponse?> readFileByHash(String hash) async {
    final bridge = ref.read(bridgeServiceProvider);
    if (!bridge.isFfiEnabled) {
      debugPrint('readFileByHash: FFI 桥接不可用');
      return null;
    }

    try {
      debugPrint('VirtualFileTreeProvider: 读取文件 $hash');

      // 调用后端读取文件
      final ffiResult = await bridge.readFileByHash(
        workspaceId: workspaceId,
        hash: hash,
      );

      // 转换为 Dart 模型
      if (ffiResult == null) {
        return null;
      }

      return FileContentResponse(
        content: ffiResult.content,
        hash: ffiResult.hash,
        size: ffiResult.size.toInt(),  // PlatformInt64 to int
      );
    } catch (e) {
      debugPrint('readFileByHash error: $e');
      return null;
    }
  }
}

// ==================== 辅助扩展 ====================

/// VirtualTreeNode 辅助扩展
extension VirtualTreeNodeExtension on VirtualTreeNode {
  /// 获取节点名称
  String get nodeName => switch (this) {
    VirtualTreeNodeFile(:final name) => name,
    VirtualTreeNodeArchive(:final name) => name,
  };

  /// 获取节点路径
  String get nodePath => switch (this) {
    VirtualTreeNodeFile(:final path) => path,
    VirtualTreeNodeArchive(:final path) => path,
  };

  /// 获取节点哈希
  String get nodeHash => switch (this) {
    VirtualTreeNodeFile(:final hash) => hash,
    VirtualTreeNodeArchive(:final hash) => hash,
  };

  /// 获取节点类型
  VirtualTreeNodeType get nodeType => switch (this) {
    VirtualTreeNodeFile() => VirtualTreeNodeType.file,
    VirtualTreeNodeArchive() => VirtualTreeNodeType.archive,
  };

  /// 是否为归档节点
  bool get isArchive => this is VirtualTreeNodeArchive;

  /// 是否为文件节点
  bool get isFile => this is VirtualTreeNodeFile;

  /// 获取子节点（仅归档节点有子节点）
  List<VirtualTreeNode> get children => switch (this) {
    VirtualTreeNodeFile() => [],
    VirtualTreeNodeArchive(:final children) => children,
  };

  /// 是否有子节点（归档节点可能有子节点）
  bool get hasChildren => switch (this) {
    VirtualTreeNodeFile() => false,
    VirtualTreeNodeArchive(:final children) => children.isNotEmpty,
  };

  /// 是否需要懒加载（归档节点且无子节点）
  bool get needsLazyLoad => switch (this) {
    VirtualTreeNodeFile() => false,
    VirtualTreeNodeArchive(:final children) => children.isEmpty,
  };
}
