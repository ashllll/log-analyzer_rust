import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/saved_filter.dart';
import '../services/bridge_service.dart';

part 'saved_filters_provider.g.dart';

/// BridgeService Provider
///
/// 提供 BridgeService 单例访问
@riverpod
BridgeService bridgeService(Ref ref) {
  return BridgeService.instance;
}

/// 过滤器状态 Provider
///
/// 使用 Riverpod 3.0 AsyncNotifier 管理过滤器状态
/// 支持 workspaceId 参数化，切换工作区时自动刷新
@riverpod
class SavedFilters extends _$SavedFilters {
  @override
  AsyncValue<List<SavedFilter>> build(String workspaceId) {
    // 初始加载过滤器（延迟执行避免在 build 中直接调用异步方法）
    Future.microtask(() => _loadFilters());

    // 返回初始加载状态
    return const AsyncLoading();
  }

  /// 加载过滤器列表
  ///
  /// 从 FFI 获取指定工作区的过滤器列表
  Future<void> _loadFilters() async {
    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回空列表
      if (!bridge.isFfiEnabled) {
        debugPrint('SavedFiltersProvider: FFI 未初始化，返回空列表');
        state = const AsyncData([]);
        return;
      }

      // 获取过滤器列表
      final filters = await bridge.getSavedFilters(workspaceId);

      state = AsyncData(filters);
      debugPrint('SavedFiltersProvider: 已加载 ${filters.length} 个过滤器');
    } catch (e) {
      debugPrint('SavedFiltersProvider: 加载过滤器失败: $e');
      // FFI 调用失败时返回空列表
      state = const AsyncData([]);
    }
  }

  /// 刷新过滤器列表
  ///
  /// 重新从后端加载过滤器
  Future<void> refresh() async {
    await _loadFilters();
  }

  /// 保存过滤器
  ///
  /// 使用乐观更新模式，先更新 UI，后同步后端
  Future<bool> saveFilter(SavedFilter filter) async {
    // 获取当前值（处理可能的 loading/error 状态）
    final previous = state.value ?? [];

    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回 false
      if (!bridge.isFfiEnabled) {
        debugPrint('SavedFiltersProvider: FFI 未初始化，无法保存过滤器');
        return false;
      }

      // 检查是否为更新（已有 ID）或新建
      final isUpdate = filter.id.isNotEmpty;

      // 乐观更新 - 立即更新 UI
      if (isUpdate) {
        // 更新现有过滤器
        state = AsyncData(
          previous.map((f) => f.id == filter.id ? filter : f).toList(),
        );
      } else {
        // 添加新过滤器（生成临时 ID）
        final newFilter = filter.copyWith(
          id: 'temp-${DateTime.now().millisecondsSinceEpoch}',
          createdAt: DateTime.now().toIso8601String(),
        );
        state = AsyncData([...previous, newFilter]);
      }

      // 后端同步
      final success = await bridge.saveFilter(filter);

      if (success) {
        debugPrint('SavedFiltersProvider: 过滤器 "${filter.name}" 已保存');
        // 刷新列表以获取正确的 ID
        await _loadFilters();
        return true;
      } else {
        // 保存失败，回滚
        state = AsyncData(previous);
        debugPrint('SavedFiltersProvider: 保存过滤器失败');
        return false;
      }
    } catch (e) {
      debugPrint('SavedFiltersProvider: 保存过滤器失败: $e');
      // 回滚 - 重新从后端加载
      await _loadFilters();
      return false;
    }
  }

  /// 删除过滤器
  ///
  /// 使用乐观更新模式，失败时回滚
  Future<bool> deleteFilter(String filterId) async {
    final previous = state.value ?? [];

    // 乐观更新 - 立即从列表中移除
    state = AsyncData(previous.where((f) => f.id != filterId).toList());

    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回 false
      if (!bridge.isFfiEnabled) {
        debugPrint('SavedFiltersProvider: FFI 未初始化，无法删除过滤器');
        state = AsyncData(previous);
        return false;
      }

      // 后端同步
      final success = await bridge.deleteFilter(filterId, workspaceId);

      if (success) {
        debugPrint('SavedFiltersProvider: 过滤器已删除');
        return true;
      } else {
        // 删除失败，回滚
        state = AsyncData(previous);
        debugPrint('SavedFiltersProvider: 删除过滤器失败');
        return false;
      }
    } catch (e) {
      debugPrint('SavedFiltersProvider: 删除过滤器失败: $e');
      // 回滚 - 恢复之前状态
      state = AsyncData(previous);
      return false;
    }
  }

  /// 更新过滤器使用统计
  ///
  /// 当用户使用过滤器时调用，更新使用次数和最后使用时间
  Future<bool> updateFilterUsage(String filterId) async {
    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回 false
      if (!bridge.isFfiEnabled) {
        return false;
      }

      // 后端同步
      final success = await bridge.updateFilterUsage(filterId, workspaceId);

      if (success) {
        debugPrint('SavedFiltersProvider: 过滤器使用统计已更新');
        // 可选：刷新列表以获取最新的使用次数
        // await _loadFilters();
      }

      return success;
    } catch (e) {
      debugPrint('SavedFiltersProvider: 更新过滤器使用统计失败: $e');
      return false;
    }
  }

  /// 获取默认过滤器
  ///
  /// 返回工作区的默认过滤器
  SavedFilter? getDefaultFilter() {
    final filters = state.value ?? [];
    try {
      return filters.firstWhere((f) => f.isDefault);
    } catch (e) {
      return null;
    }
  }

  /// 按名称查找过滤器
  ///
  /// 返回指定名称的过滤器
  SavedFilter? findByName(String name) {
    final filters = state.value ?? [];
    try {
      return filters.firstWhere((f) => f.name == name);
    } catch (e) {
      return null;
    }
  }
}
