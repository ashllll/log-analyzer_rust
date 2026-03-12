// 端到端工作流集成测试
//
// 测试完整的用户工作流：文件浏览 -> 搜索 -> 历史管理

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/features/search/providers/search_query_provider.dart';
import 'package:log_analyzer_flutter/shared/providers/search_history_provider.dart';
import 'package:log_analyzer_flutter/shared/providers/virtual_file_tree_provider.dart';

/// Helper function to wait for provider initialization
Future<void> _waitForHistoryInitialization(
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
  group('End-to-End Workflow Integration Tests', () {
    late ProviderContainer container;
    const testWorkspaceId = 'test-workspace-1';

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    group('完整工作流: 浏览 -> 搜索 -> 历史', () {
      test('应能完成搜索到历史的完整流程', () async {
        // Wait for history provider initialization
        await _waitForHistoryInitialization(container, testWorkspaceId);

        // ========== 步骤 1: 添加关键词 ==========
        final searchNotifier = container.read(searchQueryProvider.notifier);
        searchNotifier.addKeyword('error');

        // 验证关键词已添加
        expect(container.read(searchQueryProvider).terms.length, equals(1));

        // ========== 步骤 2: 构建查询 ==========
        final query = await searchNotifier.buildQuery();
        expect(query.terms.length, equals(1));

        // ========== 步骤 3: 保存搜索历史 ==========
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 2);

        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history.length, equals(1));
        expect(history.first.query, equals('error'));
        expect(history.first.resultCount, equals(2));
      });

      test('应能从历史记录恢复搜索', () async {
        // Wait for history provider initialization
        await _waitForHistoryInitialization(container, testWorkspaceId);

        final searchNotifier = container.read(searchQueryProvider.notifier);
        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 初始搜索
        searchNotifier.addKeyword('error');
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 2);
        searchNotifier.clearKeywords();

        // 验证初始搜索已完成
        expect(container.read(searchQueryProvider).terms, isEmpty);

        // ========== 从历史恢复搜索 ==========
        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        // 选择历史记录
        final lastSearch = history.first.query;
        expect(lastSearch, equals('error'));

        // 恢复关键词
        searchNotifier.addKeyword(lastSearch);

        // 验证关键词已恢复
        final restoredState = container.read(searchQueryProvider);
        expect(restoredState.terms.length, equals(1));
        expect(restoredState.terms.first.value, equals('error'));
      });
    });

    group('历史管理工作流', () {
      test('应能管理多条历史记录', () async {
        // Wait for history provider initialization
        await _waitForHistoryInitialization(container, testWorkspaceId);

        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        // 添加多条历史
        await historyNotifier.addSearchHistory(query: 'error', resultCount: 10);
        await historyNotifier.addSearchHistory(
          query: 'warning',
          resultCount: 5,
        );
        await historyNotifier.addSearchHistory(query: 'info', resultCount: 3);

        // 验证历史记录顺序
        var historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        var history = historyState.value ?? [];

        expect(history.length, equals(3));
        expect(history[0].query, equals('info'));
        expect(history[1].query, equals('warning'));
        expect(history[2].query, equals('error'));

        // 删除中间记录
        await historyNotifier.deleteSearchHistory('warning');

        // 验证删除后状态
        historyState = container.read(searchHistoryProvider(testWorkspaceId));
        history = historyState.value ?? [];

        expect(history.length, equals(2));
        expect(history[0].query, equals('info'));
        expect(history[1].query, equals('error'));

        // 清空所有
        await historyNotifier.clearSearchHistory();

        // 验证清空后状态
        historyState = container.read(searchHistoryProvider(testWorkspaceId));
        history = historyState.value ?? [];

        expect(history, isEmpty);
      });

      test('批量删除应正确工作', () async {
        // Wait for history provider initialization
        await _waitForHistoryInitialization(container, testWorkspaceId);

        final historyNotifier = container.read(
          searchHistoryProvider(testWorkspaceId).notifier,
        );

        await historyNotifier.addSearchHistory(query: 'error', resultCount: 10);
        await historyNotifier.addSearchHistory(
          query: 'warning',
          resultCount: 5,
        );
        await historyNotifier.addSearchHistory(query: 'info', resultCount: 3);
        await historyNotifier.addSearchHistory(query: 'debug', resultCount: 1);

        // 批量删除 error 和 info
        await historyNotifier.deleteSearchHistories(['error', 'info']);

        final historyState = container.read(
          searchHistoryProvider(testWorkspaceId),
        );
        final history = historyState.value ?? [];

        expect(history.length, equals(2));
        expect(
          history.map((h) => h.query).toList(),
          containsAll(['warning', 'debug']),
        );
      });
    });

    group('文件树导航工作流', () {
      test('应能导航目录结构', () async {
        // 验证 Provider 正常工作
        await Future.delayed(const Duration(milliseconds: 100));

        final treeState = container.read(
          virtualFileTreeProvider(testWorkspaceId),
        );
        expect(treeState.value, isNotNull);
      });

      test('应能处理节点结构', () {
        const archiveNode = VirtualTreeNode.archive(
          name: 'logs',
          path: '/logs',
          hash: 'dir-hash-1',
          archiveType: 'directory',
          children: [
            VirtualTreeNode.file(
              name: 'app.log',
              path: '/logs/app.log',
              hash: 'file-hash-1',
              size: 1024,
            ),
            VirtualTreeNode.file(
              name: 'error.log',
              path: '/logs/error.log',
              hash: 'file-hash-2',
              size: 2048,
            ),
          ],
        );

        expect(archiveNode.isArchive, isTrue); // 是归档/目录节点
        expect(archiveNode.hasChildren, isTrue);
        expect(archiveNode.children.length, equals(2));

        final appLog = archiveNode.children.firstWhere(
          (n) => n.name == 'app.log',
        );
        expect(appLog.isFile, isTrue);
        // 使用模式匹配访问 size
        if (appLog case VirtualTreeNodeFile(:final size)) {
          expect(size, equals(1024));
        }
      });
    });

    group('跨工作区工作流', () {
      test('应能隔离不同工作区的数据', () async {
        const ws1 = 'workspace-1';
        const ws2 = 'workspace-2';

        // Wait for both workspace providers to initialize
        await _waitForHistoryInitialization(container, ws1);
        await _waitForHistoryInitialization(container, ws2);

        // 为工作区 1 添加数据
        final historyWs1 = container.read(searchHistoryProvider(ws1).notifier);
        await historyWs1.addSearchHistory(query: 'ws1-error', resultCount: 1);

        // 为工作区 2 添加数据
        final historyWs2 = container.read(searchHistoryProvider(ws2).notifier);
        await historyWs2.addSearchHistory(query: 'ws2-warning', resultCount: 2);

        // 验证数据隔离
        final state1 = container.read(searchHistoryProvider(ws1));
        final state2 = container.read(searchHistoryProvider(ws2));

        expect(state1.value?.length, equals(1));
        expect(state1.value?.first.query, equals('ws1-error'));

        expect(state2.value?.length, equals(1));
        expect(state2.value?.first.query, equals('ws2-warning'));

        // 删除一个工作区的数据不应影响另一个
        await historyWs1.clearSearchHistory();

        final updatedState1 = container.read(searchHistoryProvider(ws1));
        final updatedState2 = container.read(searchHistoryProvider(ws2));

        expect(updatedState1.value, isEmpty);
        expect(updatedState2.value?.length, equals(1));
      });
    });
  });
}
