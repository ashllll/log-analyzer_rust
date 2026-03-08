import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/bridge_service.dart';
import '../services/generated/ffi/bridge.dart' as ffi;
import '../services/generated/ffi/types.dart' as ffi_types;

part 'search_history_provider.g.dart';

// ==================== 搜索结果缓存 ====================

/// 搜索结果缓存
///
/// 缓存搜索结果以提升重复搜索的性能
class SearchResultCache {
  /// 最大缓存条目数
  static const int maxCacheSize = 50;

  /// 缓存容器 (query -> List of results)
  final Map<String, List<Map<String, dynamic>>> _cache = {};

  /// 缓存时间戳
  final Map<String, DateTime> _cacheTime = {};

  /// TTL (毫秒)
  final int ttlMs;

  SearchResultCache({this.ttlMs = 5 * 60 * 1000}); // 默认 5 分钟

  /// 获取缓存的搜索结果
  List<Map<String, dynamic>>? get(String query) {
    final cached = _cache[query];
    if (cached == null) return null;

    // 检查是否过期
    final cacheTime = _cacheTime[query];
    if (cacheTime != null) {
      final age = DateTime.now().difference(cacheTime).inMilliseconds;
      if (age > ttlMs) {
        // 过期，删除缓存
        _cache.remove(query);
        _cacheTime.remove(query);
        return null;
      }
    }

    return cached;
  }

  /// 设置缓存
  void set(String query, List<Map<String, dynamic>> results) {
    // 如果缓存已满，删除最旧的条目
    if (_cache.length >= maxCacheSize) {
      _removeOldest();
    }

    _cache[query] = results;
    _cacheTime[query] = DateTime.now();
  }

  /// 清空缓存
  void clear() {
    _cache.clear();
    _cacheTime.clear();
  }

  /// 删除最旧的缓存条目
  void _removeOldest() {
    if (_cache.isEmpty) return;

    String? oldestKey;
    DateTime? oldestTime;

    for (final entry in _cacheTime.entries) {
      if (oldestTime == null || entry.value.isBefore(oldestTime)) {
        oldestTime = entry.value;
        oldestKey = entry.key;
      }
    }

    if (oldestKey != null) {
      _cache.remove(oldestKey);
      _cacheTime.remove(oldestKey);
    }
  }

  /// 获取缓存大小
  int get size => _cache.length;

  /// 检查是否包含某个查询
  bool contains(String query) => _cache.containsKey(query);
}

/// 全局搜索结果缓存实例
final searchResultCache = SearchResultCache();

/// 搜索历史条目模型
///
/// 本地 Dart 模型，用于 Riverpod 状态管理
/// 对应 Rust 后端的 SearchHistoryData 结构体
class SearchHistoryItem {
  /// 查询内容
  final String query;

  /// 工作区 ID
  final String workspaceId;

  /// 结果数量
  final int resultCount;

  /// 搜索时间（ISO 8601 格式）
  final String searchedAt;

  const SearchHistoryItem({
    required this.query,
    required this.workspaceId,
    required this.resultCount,
    required this.searchedAt,
  });

  /// 从 FFI SearchHistoryData 创建
  factory SearchHistoryItem.fromFfi(ffi_types.SearchHistoryData data) {
    return SearchHistoryItem(
      query: data.query,
      workspaceId: data.workspaceId,
      resultCount: data.resultCount,
      searchedAt: data.searchedAt,
    );
  }

  /// 转换为 FFI SearchHistoryData
  ffi_types.SearchHistoryData toFfi() {
    return ffi_types.SearchHistoryData(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
      searchedAt: searchedAt,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SearchHistoryItem &&
          runtimeType == other.runtimeType &&
          query == other.query &&
          workspaceId == other.workspaceId;

  @override
  int get hashCode => query.hashCode ^ workspaceId.hashCode;

  @override
  String toString() =>
      'SearchHistoryItem(query: $query, workspaceId: $workspaceId, resultCount: $resultCount)';
}

/// BridgeService Provider
///
/// 提供 BridgeService 单例访问
@riverpod
BridgeService bridgeService(Ref ref) {
  return BridgeService.instance;
}

/// 搜索历史状态 Provider
///
/// 使用 Riverpod 3.0 AsyncNotifier 管理搜索历史状态
/// 支持 workspaceId 参数化，切换工作区时自动刷新
@riverpod
class SearchHistory extends _$SearchHistory {
  @override
  AsyncValue<List<SearchHistoryItem>> build(String workspaceId) {
    // 初始加载搜索历史（延迟执行避免在 build 中直接调用异步方法）
    Future.microtask(() => _loadHistory());

    // 返回初始加载状态
    return const AsyncLoading();
  }

  /// 加载搜索历史
  ///
  /// 从 FFI 获取指定工作区的搜索历史
  Future<void> _loadHistory() async {
    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回空列表
      if (!bridge.isFfiEnabled) {
        debugPrint('SearchHistoryProvider: FFI 未初始化，返回空列表');
        state = const AsyncData([]);
        return;
      }

      // 直接使用 FFI 生成的函数
      final ffiHistory = ffi.getSearchHistory(workspaceId: workspaceId);

      // 转换为本地模型
      final history = ffiHistory
          .map((data) => SearchHistoryItem.fromFfi(data))
          .toList();

      state = AsyncData(history);
      debugPrint('SearchHistoryProvider: 已加载 ${history.length} 条搜索历史');
    } catch (e) {
      debugPrint('SearchHistoryProvider: 加载搜索历史失败: $e');
      // FFI 调用失败时返回空列表（根据 CONTEXT.md 决策）
      state = const AsyncData([]);
    }
  }

  /// 刷新搜索历史
  ///
  /// 重新从后端加载搜索历史
  Future<void> refresh() async {
    await _loadHistory();
  }

  /// 添加搜索历史记录
  ///
  /// 使用乐观更新模式，先更新 UI，后同步后端
  Future<void> addSearchHistory({
    required String query,
    required int resultCount,
  }) async {
    // 获取当前值（处理可能的 loading/error 状态）
    final previous = state.value ?? [];

    // 创建新的历史记录
    final newItem = SearchHistoryItem(
      query: query,
      workspaceId: workspaceId,
      resultCount: resultCount,
      searchedAt: DateTime.now().toIso8601String(),
    );

    // 乐观更新 - 立即更新 UI
    // 将新记录插入到列表开头（最近搜索在最前）
    final updatedList = [newItem, ...previous];
    state = AsyncData(updatedList);

    try {
      // 后端同步
      ffi.addSearchHistory(
        query: query,
        workspaceId: workspaceId,
        resultCount: resultCount,
      );
      debugPrint('SearchHistoryProvider: 已添加搜索历史 "$query"');
    } catch (e) {
      debugPrint('SearchHistoryProvider: 添加搜索历史失败: $e');
      // 回滚 - 重新从后端加载
      await _loadHistory();
      rethrow;
    }
  }

  /// 删除单条搜索历史记录
  ///
  /// 使用乐观更新模式，失败时回滚
  Future<void> deleteSearchHistory(String query) async {
    final previous = state.value ?? [];

    // 乐观更新 - 立即从列表中移除
    state = AsyncData(previous.where((h) => h.query != query).toList());

    try {
      // 后端同步
      ffi.deleteSearchHistory(query: query, workspaceId: workspaceId);
      debugPrint('SearchHistoryProvider: 已删除搜索历史 "$query"');
    } catch (e) {
      debugPrint('SearchHistoryProvider: 删除搜索历史失败: $e');
      // 回滚 - 恢复之前状态
      state = AsyncData(previous);
      rethrow;
    }
  }

  /// 批量删除搜索历史记录
  ///
  /// 使用乐观更新模式，失败时回滚
  Future<void> deleteSearchHistories(List<String> queries) async {
    final previous = state.value ?? [];

    // 乐观更新
    final querySet = queries.toSet();
    state = AsyncData(
      previous.where((h) => !querySet.contains(h.query)).toList(),
    );

    try {
      // 后端同步
      ffi.deleteSearchHistories(queries: queries, workspaceId: workspaceId);
      debugPrint('SearchHistoryProvider: 已批量删除 ${queries.length} 条搜索历史');
    } catch (e) {
      debugPrint('SearchHistoryProvider: 批量删除搜索历史失败: $e');
      // 回滚
      state = AsyncData(previous);
      rethrow;
    }
  }

  /// 清空所有搜索历史
  ///
  /// 使用乐观更新模式，清空当前工作区的所有历史记录
  Future<void> clearSearchHistory() async {
    final previous = state.value ?? [];

    // 乐观更新 - 立即清空
    state = const AsyncData([]);

    try {
      // 后端同步
      ffi.clearSearchHistory(workspaceId: workspaceId);
      debugPrint('SearchHistoryProvider: 已清空搜索历史');
    } catch (e) {
      debugPrint('SearchHistoryProvider: 清空搜索历史失败: $e');
      // 回滚
      state = AsyncData(previous);
      rethrow;
    }
  }
}
