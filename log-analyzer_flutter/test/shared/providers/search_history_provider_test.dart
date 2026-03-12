// SearchHistoryProvider 测试
//
// 测试搜索历史状态管理功能

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/shared/providers/search_history_provider.dart';

/// Helper function to wait for provider initialization
Future<void> _waitForInitialization(
  ProviderContainer container,
  String workspaceId,
) async {
  final provider = searchHistoryProvider(workspaceId);

  // Listen to trigger initialization
  container.listen<AsyncValue<List<SearchHistoryItem>>>(
    provider,
    (_, __) {},
  );

  // Wait for async initialization to complete
  await Future.delayed(const Duration(milliseconds: 50));
}

void main() {
  group('SearchHistoryProvider Tests', () {
    late ProviderContainer container;
    const testWorkspaceId = 'test-workspace-1';

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    group('初始状态', () {
      test('应返回空列表', () async {
        // 监听 provider 以触发初始化
        final provider = searchHistoryProvider(testWorkspaceId);

        // 使用 listen 触发 provider 初始化
        container.listen<AsyncValue<List<SearchHistoryItem>>>(
          provider,
          (_, __) {},
        );

        // 等待异步初始化完成（FFI 未初始化时返回空列表）
        // 使用 pumpEventQueue 确保所有微任务完成
        await Future.delayed(const Duration(milliseconds: 50));

        final state = container.read(provider);

        // FFI 未初始化时，应返回空列表（AsyncData）
        expect(state.value, isNotNull);
        expect(state.value, isEmpty);
      });
    });

    group('添加搜索历史', () {
      test('应成功添加搜索历史记录', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        await notifier.addSearchHistory(query: 'error', resultCount: 10);

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('error'));
        expect(history.first.resultCount, equals(10));
      });

      test('应将新记录插入到列表开头', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加第一条记录
        await notifier.addSearchHistory(query: 'error', resultCount: 10);

        // 添加第二条记录
        await notifier.addSearchHistory(query: 'warning', resultCount: 5);

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history.first.query, equals('warning'));
        expect(history.last.query, equals('error'));
      });

      test('应设置正确的 workspaceId', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        await notifier.addSearchHistory(query: 'error', resultCount: 10);

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history.first.workspaceId, equals(testWorkspaceId));
      });
    });

    group('删除搜索历史', () {
      test('应成功删除单条历史记录', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 先添加记录
        await notifier.addSearchHistory(query: 'error', resultCount: 10);
        await notifier.addSearchHistory(query: 'warning', resultCount: 5);

        // 删除一条
        await notifier.deleteSearchHistory('error');

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('warning'));
      });

      test('删除不存在的记录应无效果', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加记录
        await notifier.addSearchHistory(query: 'error', resultCount: 10);

        // 删除不存在的记录
        await notifier.deleteSearchHistory('non-existent');

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history.length, equals(1));
      });
    });

    group('批量删除搜索历史', () {
      test('应成功批量删除多条记录', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加多条记录
        await notifier.addSearchHistory(query: 'error', resultCount: 10);
        await notifier.addSearchHistory(query: 'warning', resultCount: 5);
        await notifier.addSearchHistory(query: 'info', resultCount: 3);

        // 批量删除
        await notifier.deleteSearchHistories(['error', 'info']);

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('warning'));
      });
    });

    group('清空搜索历史', () {
      test('应成功清空所有历史记录', () async {
        // Wait for provider initialization first
        await _waitForInitialization(container, testWorkspaceId);

        final notifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加多条记录
        await notifier.addSearchHistory(query: 'error', resultCount: 10);
        await notifier.addSearchHistory(query: 'warning', resultCount: 5);

        // 清空
        await notifier.clearSearchHistory();

        final state = container.read(searchHistoryProvider(testWorkspaceId));
        final history = state.value ?? [];

        expect(history, isEmpty);
      });
    });
  });

  group('SearchHistoryItem Tests', () {
    test('应正确实现相等性', () {
      const item1 = SearchHistoryItem(
        query: 'error',
        workspaceId: 'ws-1',
        resultCount: 10,
        searchedAt: '2024-01-01T00:00:00Z',
      );

      const item2 = SearchHistoryItem(
        query: 'error',
        workspaceId: 'ws-1',
        resultCount: 5,
        searchedAt: '2024-01-02T00:00:00Z',
      );

      const item3 = SearchHistoryItem(
        query: 'warning',
        workspaceId: 'ws-1',
        resultCount: 10,
        searchedAt: '2024-01-01T00:00:00Z',
      );

      // query 和 workspaceId 相同则相等
      expect(item1, equals(item2));
      // query 不同则不相等
      expect(item1, isNot(equals(item3)));
    });

    test('应正确实现 hashCode', () {
      const item1 = SearchHistoryItem(
        query: 'error',
        workspaceId: 'ws-1',
        resultCount: 10,
        searchedAt: '2024-01-01T00:00:00Z',
      );

      const item2 = SearchHistoryItem(
        query: 'error',
        workspaceId: 'ws-1',
        resultCount: 5,
        searchedAt: '2024-01-02T00:00:00Z',
      );

      expect(item1.hashCode, equals(item2.hashCode));
    });

    test('toString 应返回正确的字符串表示', () {
      const item = SearchHistoryItem(
        query: 'error',
        workspaceId: 'ws-1',
        resultCount: 10,
        searchedAt: '2024-01-01T00:00:00Z',
      );

      final str = item.toString();

      expect(str, contains('error'));
      expect(str, contains('ws-1'));
      expect(str, contains('10'));
    });
  });
}
