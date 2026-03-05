import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/bridge_service.dart';

part 'search_history_provider.g.dart';

/// 搜索历史条目模型
///
/// 由于 FFI SearchHistoryData 类型未生成，使用本地 Dart 模型
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

  /// 从 Map 创建（用于 FFI 返回值转换）
  factory SearchHistoryItem.fromMap(Map<String, dynamic> map) {
    return SearchHistoryItem(
      query: map['query'] as String? ?? '',
      workspaceId: map['workspace_id'] as String? ?? '',
      resultCount: map['result_count'] as int? ?? 0,
      searchedAt: map['searched_at'] as String? ?? '',
    );
  }

  /// 转换为 Map
  Map<String, dynamic> toMap() {
    return {
      'query': query,
      'workspace_id': workspaceId,
      'result_count': resultCount,
      'searched_at': searchedAt,
    };
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
  /// 从 BridgeService 获取指定工作区的搜索历史
  Future<void> _loadHistory() async {
    try {
      final bridge = ref.read(bridgeServiceProvider);

      // FFI 未初始化时返回空列表
      if (!bridge.isFfiEnabled) {
        debugPrint('SearchHistoryProvider: FFI 未初始化，返回空列表');
        state = const AsyncData([]);
        return;
      }

      // 获取搜索历史（已按时间降序排序）
      // 注意：由于 FFI 类型未生成，这里需要手动转换
      final rawHistory = await bridge.getSearchHistory(
        workspaceId: workspaceId,
      );

      // 将 FFI 返回的数据转换为本地模型
      // 由于类型系统限制，使用 dynamic 处理
      // ignore: avoid_dynamic_calls
      final history = rawHistory.map((dynamic item) {
        // 尝试从 item 获取属性
        // ignore: avoid_dynamic_calls
        return SearchHistoryItem(
          // ignore: avoid_dynamic_calls
          query: (item.query as String?) ?? '',
          // ignore: avoid_dynamic_calls
          workspaceId: (item.workspaceId as String?) ?? '',
          // ignore: avoid_dynamic_calls
          resultCount: (item.resultCount as int?) ?? 0,
          // ignore: avoid_dynamic_calls
          searchedAt: (item.searchedAt as String?) ?? '',
        );
      }).toList();

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
    final bridge = ref.read(bridgeServiceProvider);
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
      await bridge.addSearchHistory(
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
    final bridge = ref.read(bridgeServiceProvider);
    final previous = state.value ?? [];

    // 乐观更新 - 立即从列表中移除
    state = AsyncData(
      previous.where((h) => h.query != query).toList(),
    );

    try {
      // 后端同步
      await bridge.deleteSearchHistory(
        query: query,
        workspaceId: workspaceId,
      );
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
    final bridge = ref.read(bridgeServiceProvider);
    final previous = state.value ?? [];

    // 乐观更新
    final querySet = queries.toSet();
    state = AsyncData(
      previous.where((h) => !querySet.contains(h.query)).toList(),
    );

    try {
      // 后端同步
      await bridge.deleteSearchHistories(
        queries: queries,
        workspaceId: workspaceId,
      );
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
    final bridge = ref.read(bridgeServiceProvider);
    final previous = state.value ?? [];

    // 乐观更新 - 立即清空
    state = const AsyncData([]);

    try {
      // 后端同步
      await bridge.clearSearchHistory(workspaceId: workspaceId);
      debugPrint('SearchHistoryProvider: 已清空搜索历史');
    } catch (e) {
      debugPrint('SearchHistoryProvider: 清空搜索历史失败: $e');
      // 回滚
      state = AsyncData(previous);
      rethrow;
    }
  }
}
