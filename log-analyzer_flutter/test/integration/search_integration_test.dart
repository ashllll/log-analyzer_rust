// 搜索流程集成测试
//
// 测试完整的搜索工作流

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/features/search/providers/search_query_provider.dart';
import 'package:log_analyzer_flutter/shared/providers/search_history_provider.dart';

void main() {
  group('Search Integration Tests', () {
    late ProviderContainer container;
    const testWorkspaceId = 'test-workspace-1';

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    group('搜索流程', () {
      test('添加关键词后应触发搜索', () async {
        final searchNotifier = container.read(searchQueryProvider.notifier);
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加搜索关键词
        searchNotifier.addKeyword('error');

        // 验证关键词已添加
        expect(container.read(searchQueryProvider).terms.length, equals(1));

        // 构建查询
        final query = await searchNotifier.buildQuery();
        expect(query.terms.length, equals(1));
        expect(query.terms.first.value, equals('error'));

        // 模拟搜索完成，添加历史记录
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 2);

        // 验证历史记录已添加
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];
        expect(history.length, equals(1));
        expect(history.first.query, equals('error'));
        expect(history.first.resultCount, equals(2));
      });

      test('多关键词搜索应正确组合', () async {
        final searchNotifier = container.read(searchQueryProvider.notifier);

        // 添加多个关键词
        searchNotifier.addKeyword('error');
        searchNotifier.addKeyword('warning');

        // 构建查询
        final query = await searchNotifier.buildQuery();

        expect(query.terms.length, equals(2));
      });

      test('禁用关键词后不应影响搜索', () async {
        final searchNotifier = container.read(searchQueryProvider.notifier);

        // 添加关键词
        searchNotifier.addKeyword('error');
        searchNotifier.addKeyword('warning');

        // 禁用第一个关键词
        final errorId = container.read(searchQueryProvider).terms.first.id;
        searchNotifier.toggleKeyword(errorId);

        // 构建查询
        final query = await searchNotifier.buildQuery();

        // 应该只有 1 个启用的关键词
        expect(query.terms.length, equals(1));
        expect(query.terms.first.value, equals('warning'));
      });

      test('清空关键词应重置搜索', () async {
        final searchNotifier = container.read(searchQueryProvider.notifier);

        // 添加关键词
        searchNotifier.addKeyword('error');
        searchNotifier.addKeyword('warning');

        // 清空
        searchNotifier.clearKeywords();

        // 验证已清空
        expect(container.read(searchQueryProvider).terms, isEmpty);
        expect(searchNotifier.hasKeywords, isFalse);
      });
    });

    group('搜索历史流程', () {
      test('搜索完成后应自动保存历史', () async {
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 模拟搜索完成
        await historyNotifier.addSearchHistory(
          query: 'error AND fatal',
          resultCount: 5,
        );

        // 验证历史记录
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('error AND fatal'));
        expect(history.first.resultCount, equals(5));
      });

      test('从历史记录恢复搜索应正确加载', () async {
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );
        final searchNotifier = container.read(searchQueryProvider.notifier);

        // 添加历史记录
        await historyNotifier.addSearchHistory(
          query: 'error warning',
          resultCount: 3,
        );

        // 模拟从历史记录选择
        final history =
            container
                .read(searchHistoryProvider(testWorkspaceId))
                .value ??
            [];
        final selectedQuery = history.first.query;

        // 解析并添加到搜索
        final keywords = selectedQuery.split(' ');
        for (final keyword in keywords) {
          if (keyword.isNotEmpty) {
            searchNotifier.addKeyword(keyword);
          }
        }

        // 验证关键词已加载
        expect(container.read(searchQueryProvider).terms.length, equals(2));
      });

      test('删除历史记录后应更新搜索', () async {
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加历史记录
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 1);
        await historyNotifier.addSearchHistory(
          query: 'warning',
          resultCount: 2,
        );

        // 删除一条
        await historyNotifier.deleteSearchHistory('error');

        // 验证
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('warning'));
      });

      test('清空历史记录后应重置', () async {
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加历史记录
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 1);
        await historyNotifier.addSearchHistory(
          query: 'warning',
          resultCount: 2,
        );

        // 清空
        await historyNotifier.clearSearchHistory();

        // 验证
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history, isEmpty);
      });
    });

    group('端到端搜索流程', () {
      test('完整搜索流程: 添加关键词 -> 搜索 -> 保存历史', () async {
        final searchNotifier = container.read(searchQueryProvider.notifier);
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 1. 添加搜索关键词
        searchNotifier.addKeyword('error');
        searchNotifier.addKeyword('fatal');

        expect(container.read(searchQueryProvider).terms.length, equals(2));

        // 2. 构建查询
        final query = await searchNotifier.buildQuery();
        expect(query.terms.length, equals(2));

        // 3. 模拟搜索完成
        const resultCount = 2;

        // 4. 保存历史
        await historyNotifier.addSearchHistory(
          query: searchNotifier.buildPreviewText(),
          resultCount: resultCount,
        );

        // 5. 验证结果
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('error AND fatal'));
        expect(history.first.resultCount, equals(2));
      });

      test('多轮搜索应正确累积历史', () async {
        final searchNotifier = container.read(searchQueryProvider.notifier);
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 第一轮搜索
        searchNotifier.addKeyword('error');
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 10);
        searchNotifier.clearKeywords();

        // 第二轮搜索
        searchNotifier.addKeyword('warning');
        await historyNotifier.addSearchHistory(
          query: 'warning',
          resultCount: 5,
        );
        searchNotifier.clearKeywords();

        // 第三轮搜索
        searchNotifier.addKeyword('info');
        await historyNotifier.addSearchHistory(query: 'info', resultCount: 3);

        // 验证历史记录累积
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history.length, equals(3));
        // 最近的在最前面
        expect(history[0].query, equals('info'));
        expect(history[1].query, equals('warning'));
        expect(history[2].query, equals('error'));
      });

      test('切换工作区应隔离历史记录', () async {
        const workspace1 = 'workspace-1';
        const workspace2 = 'workspace-2';

        final historyNotifier1 = container.read(
          searchHistoryProvider(workspace1).notifier,
        );
        final historyNotifier2 = container.read(
          searchHistoryProvider(workspace2).notifier,
        );

        // 工作区 1 添加历史
        await historyNotifier1.addSearchHistory(
          query: 'error-ws1',
          resultCount: 1,
        );

        // 工作区 2 添加历史
        await historyNotifier2.addSearchHistory(
          query: 'error-ws2',
          resultCount: 2,
        );

        // 验证各工作区历史独立
        final state1 = container.read(searchHistoryProvider(workspace1));
        final state2 = container.read(searchHistoryProvider(workspace2));

        expect(state1.value?.length, equals(1));
        expect(state1.value?.first.query, equals('error-ws1'));

        expect(state2.value?.length, equals(1));
        expect(state2.value?.first.query, equals('error-ws2'));
      });
    });
  });
}
