// SearchQueryProvider 测试
//
// 测试搜索查询状态管理功能

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:log_analyzer_flutter/features/search/providers/search_query_provider.dart';
import 'package:log_analyzer_flutter/shared/services/generated/ffi/types.dart'
    as ffi_types;

void main() {
  group('SearchQueryProvider Tests', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    group('初始状态', () {
      test('应该初始化为空关键词列表', () {
        final state = container.read(searchQueryProvider);
        expect(state.terms, isEmpty);
      });

      test('应该没有可搜索的关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);
        expect(notifier.hasKeywords, isFalse);
        expect(notifier.enabledCount, equals(0));
      });
    });

    group('添加关键词', () {
      test('应该成功添加关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');

        final state = container.read(searchQueryProvider);
        expect(state.terms.length, equals(1));
        expect(state.terms.first.value, equals('error'));
      });

      test('应该自动生成唯一 ID', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');

        final state = container.read(searchQueryProvider);
        expect(state.terms[0].id, isNot(equals(state.terms[1].id)));
      });

      test('应该跳过空关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('  ');
        notifier.addKeyword('');

        final state = container.read(searchQueryProvider);
        expect(state.terms, isEmpty);
      });

      test('应该跳过重复关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('error');

        final state = container.read(searchQueryProvider);
        expect(state.terms.length, equals(1));
      });

      test('应该设置正确的默认属性', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');

        final state = container.read(searchQueryProvider);
        final term = state.terms.first;
        expect(term.isRegex, isFalse);
        expect(term.enabled, isTrue);
        expect(term.caseSensitive, isFalse);
        expect(term.priority, equals(0));
      });
    });

    group('删除关键词', () {
      test('应该成功删除指定关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');

        final errorId = container.read(searchQueryProvider).terms.first.id;
        notifier.removeKeyword(errorId);

        final state = container.read(searchQueryProvider);
        expect(state.terms.length, equals(1));
        expect(state.terms.first.value, equals('warning'));
      });

      test('删除不存在的关键词应该无效果', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.removeKeyword('non-existent-id');

        final state = container.read(searchQueryProvider);
        expect(state.terms.length, equals(1));
      });
    });

    group('更新关键词', () {
      test('应该成功更新关键词值', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        final termId = container.read(searchQueryProvider).terms.first.id;

        notifier.updateKeyword(termId, 'warning');

        final state = container.read(searchQueryProvider);
        expect(state.terms.first.value, equals('warning'));
      });

      test('应该跳过空更新值', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        final termId = container.read(searchQueryProvider).terms.first.id;

        notifier.updateKeyword(termId, '  ');

        final state = container.read(searchQueryProvider);
        expect(state.terms.first.value, equals('error'));
      });
    });

    group('切换关键词启用状态', () {
      test('应该切换关键词启用状态', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        final termId = container.read(searchQueryProvider).terms.first.id;

        notifier.toggleKeyword(termId);

        final state = container.read(searchQueryProvider);
        expect(state.terms.first.enabled, isFalse);
        expect(notifier.enabledCount, equals(0));
      });

      test('应该正确切换多次', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        final termId = container.read(searchQueryProvider).terms.first.id;

        notifier.toggleKeyword(termId);
        notifier.toggleKeyword(termId);

        final state = container.read(searchQueryProvider);
        expect(state.terms.first.enabled, isTrue);
      });
    });

    group('清空关键词', () {
      test('应该清空所有关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        notifier.clearKeywords();

        final state = container.read(searchQueryProvider);
        expect(state.terms, isEmpty);
        expect(notifier.hasKeywords, isFalse);
      });
    });

    group('构建预览文本', () {
      test('无关键词时返回提示文本', () {
        final notifier = container.read(searchQueryProvider.notifier);

        final previewText = notifier.buildPreviewText();

        expect(previewText, equals('无搜索条件'));
      });

      test('单个关键词返回关键词文本', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        final previewText = notifier.buildPreviewText();

        expect(previewText, equals('error'));
      });

      test('多个关键词返回 AND 连接', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        final previewText = notifier.buildPreviewText();

        expect(previewText, equals('error AND warning'));
      });

      test('应排除禁用的关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        final warningId = container.read(searchQueryProvider).terms.last.id;
        notifier.toggleKeyword(warningId);

        final previewText = notifier.buildPreviewText();

        expect(previewText, equals('error'));
      });
    });

    group('获取关键词值列表', () {
      test('应返回所有启用的关键词值', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');

        final values = notifier.getKeywordValues();

        expect(values, contains('error'));
        expect(values, contains('warning'));
      });

      test('应排除禁用的关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        final warningId = container.read(searchQueryProvider).terms.last.id;
        notifier.toggleKeyword(warningId);

        final values = notifier.getKeywordValues();

        expect(values, equals(['error']));
      });
    });

    group('构建 FFI 查询', () {
      test('应返回正确结构的查询对象', () async {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');

        final query = await notifier.buildQuery();

        expect(query.terms.length, equals(2));
      });

      test('应排除禁用的关键词', () async {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        final warningId = container.read(searchQueryProvider).terms.last.id;
        notifier.toggleKeyword(warningId);

        final query = await notifier.buildQuery();

        expect(query.terms.length, equals(1));
        expect(query.terms.first.value, equals('error'));
      });
    });

    group('多关键词组合', () {
      test('应支持 3 个以上关键词', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        notifier.addKeyword('info');
        notifier.addKeyword('debug');

        final state = container.read(searchQueryProvider);
        expect(state.terms.length, equals(4));
      });

      test('应保持关键词优先级顺序', () {
        final notifier = container.read(searchQueryProvider.notifier);

        notifier.addKeyword('error');
        notifier.addKeyword('warning');
        notifier.addKeyword('info');

        final state = container.read(searchQueryProvider);
        expect(state.terms[0].priority, equals(0));
        expect(state.terms[1].priority, equals(1));
        expect(state.terms[2].priority, equals(2));
      });
    });
  });

  group('SearchTerm Tests', () {
    test('应正确复制并修改字段', () {
      final term = SearchTerm(
        id: 'test-id',
        value: 'error',
        operator_: ffi_types.QueryOperatorData.and,
        isRegex: false,
        priority: 0,
        enabled: true,
        caseSensitive: false,
      );

      final updated = term.copyWith(value: 'warning', isRegex: true);

      expect(updated.value, equals('warning'));
      expect(updated.isRegex, isTrue);
      expect(updated.id, equals('test-id'));
    });

    test('应正确实现相等性', () {
      final term1 = SearchTerm(
        id: 'test-id',
        value: 'error',
        operator_: ffi_types.QueryOperatorData.and,
      );

      final term2 = SearchTerm(
        id: 'test-id',
        value: 'error',
        operator_: ffi_types.QueryOperatorData.and,
      );

      expect(term1, equals(term2));
    });
  });

  group('SearchQueryState Tests', () {
    test('应正确复制并修改字段', () {
      final state = SearchQueryState(
        terms: const [],
        globalOperator: ffi_types.QueryOperatorData.and,
      );

      final updated = state.copyWith(
        globalOperator: ffi_types.QueryOperatorData.or,
      );

      expect(updated.globalOperator, equals(ffi_types.QueryOperatorData.or));
    });
  });
}
