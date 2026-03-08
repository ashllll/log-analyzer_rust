// SearchQueryBuilder 测试
//
// 遵循编码规范：使用 flutter_test

import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:log_analyzer_flutter/shared/services/search_query_builder.dart';
import 'package:log_analyzer_flutter/core/constants/app_constants.dart';

void main() {
  group('SearchQueryBuilder', () {
    group('create', () {
      test('应该创建空的查询构建器', () {
        final builder = SearchQueryBuilder.create();

        expect(builder.termCount, 0);
        expect(builder.enabledTermCount, 0);
        expect(builder.id, isNotEmpty);
      });
    });

    group('fromString', () {
      test('应该解析简单关键词', () {
        final builder = SearchQueryBuilder.fromString('error');
        final query = builder.toQuery();

        expect(query.terms, hasLength(1));
        expect(query.terms.first.value, 'error');
        expect(query.terms.first.operator_.value, 'OR');
      });

      test('应该解析多个关键词（OR 模式）', () {
        final builder = SearchQueryBuilder.fromString(
          'error | warning | fatal',
        );
        final query = builder.toQuery();

        expect(query.terms, hasLength(3));
        expect(query.terms[0].value, 'error');
        expect(query.terms[1].value, 'warning');
        expect(query.terms[2].value, 'fatal');
      });

      test('应该处理空输入', () {
        final builder = SearchQueryBuilder.fromString('');
        final query = builder.toQuery();

        expect(query.terms, isEmpty);
      });

      test('应该去除多余空格', () {
        final builder = SearchQueryBuilder.fromString(
          '  error   |   warning  ',
        );
        final query = builder.toQuery();

        expect(query.terms, hasLength(2));
      });
    });

    group('addTerm', () {
      test('应该添加单个搜索术语', () {
        final builder = SearchQueryBuilder.create().addTerm('error');

        expect(builder.termCount, 1);
        expect(builder.enabledTermCount, 1);
      });

      test('应该添加多个搜索术语', () {
        final builder = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning')
            .addTerm('fatal');

        expect(builder.termCount, 3);
      });

      test('应该支持正则表达式', () {
        final builder = SearchQueryBuilder.create().addTerm(
          r'\berror\d+\b',
          isRegex: true,
        );
        final query = builder.toQuery();

        expect(query.terms.first.isRegex, true);
        expect(query.terms.first.value, r'\berror\d+\b');
      });

      test('应该支持大小写敏感', () {
        final builder = SearchQueryBuilder.create().addTerm(
          'Error',
          caseSensitive: true,
        );
        final query = builder.toQuery();

        expect(query.terms.first.caseSensitive, true);
      });

      test('应该支持优先级设置', () {
        final builder = SearchQueryBuilder.create().addTerm(
          'fatal',
          priority: 10,
        );
        final query = builder.toQuery();

        expect(query.terms.first.priority, 10);
      });
    });

    group('removeTerm', () {
      test('应该移除指定术语', () {
        final builder = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning');
        final termId = builder.toQuery().terms.first.id;

        final updated = builder.removeTerm(termId);

        expect(updated.termCount, 1);
        expect(updated.toQuery().terms.first.value, 'warning');
      });

      test('移除不存在的术语应该无效果', () {
        final builder = SearchQueryBuilder.create().addTerm('error');

        final updated = builder.removeTerm('non-existent-id');

        expect(updated.termCount, 1);
      });
    });

    group('toggleTerm', () {
      test('应该切换术语启用状态', () {
        final builder = SearchQueryBuilder.create().addTerm('error');
        final termId = builder.toQuery().terms.first.id;

        final disabled = builder.toggleTerm(termId);
        expect(disabled.enabledTermCount, 0);

        final enabled = disabled.toggleTerm(termId);
        expect(enabled.enabledTermCount, 1);
      });
    });

    group('clear', () {
      test('应该清空所有术语', () {
        final builder = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning');

        final cleared = builder.clear();

        expect(cleared.termCount, 0);
      });
    });

    group('setGlobalOperator', () {
      test('应该设置全局操作符', () {
        final builder = SearchQueryBuilder.create()
            .addTerm('error')
            .setGlobalOperator(QueryOperator.and);

        final query = builder.toQuery();
        expect(query.globalOperator.value, 'AND');
      });
    });

    group('toQueryString', () {
      test('应该生成 OR 模式查询字符串', () {
        final queryString = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning')
            .toQueryString();

        expect(queryString, contains('error'));
        expect(queryString, contains('|'));
        expect(queryString, contains('warning'));
      });

      test('应该生成 AND 模式查询字符串', () {
        final queryString = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning')
            .setGlobalOperator(QueryOperator.and)
            .toQueryString();

        expect(queryString, contains('error'));
        expect(queryString, contains('warning'));
        expect(queryString.contains('|'), false); // AND 模式使用空格分隔
      });

      test('应该排除禁用的关键词', () {
        var builder = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning');
        final disabledTermId = builder.toQuery().terms.last.id;
        builder = builder.toggleTerm(disabledTermId);

        final queryString = builder.toQueryString();

        expect(queryString, contains('error'));
        expect(queryString, isNot(contains('warning')));
      });
    });

    group('toOptimizedQuery', () {
      test('应该生成优化后的查询', () {
        final builder = SearchQueryBuilder.create()
            .addTerm('error', priority: 1)
            .addTerm('fatal', priority: 10);

        final optimized = builder.toOptimizedQuery();

        expect(optimized.queryString, isNotEmpty);
        expect(optimized.prioritizedTerms, hasLength(2));
        // 高优先级应该在前面
        expect(optimized.prioritizedTerms.first, 'fatal');
      });
    });

    group('validate', () {
      test('应该验证有效的查询', () {
        final builder = SearchQueryBuilder.create().addTerm('error');
        final result = builder.validate();

        expect(result.valid, true);
        expect(result.errors, isNull);
      });

      test('应该检测空查询', () {
        final builder = SearchQueryBuilder.create();
        final result = builder.validate();

        expect(result.valid, false);
        expect(result.errors, isNotEmpty);
      });

      test('应该检测没有启用的术语', () {
        var builder = SearchQueryBuilder.create().addTerm('error');
        final termId = builder.toQuery().terms.first.id;
        builder = builder.toggleTerm(termId);

        final result = builder.validate();

        expect(result.warnings, isNotEmpty);
      });

      test('应该检测无效的正则表达式', () {
        final builder = SearchQueryBuilder.create().addTerm(
          '[invalid(',
          isRegex: true,
        );
        final result = builder.validate();

        expect(result.valid, false);
        expect(result.errors, isNotEmpty);
      });
    });

    group('JSON 序列化', () {
      test('export 应该生成有效的 JSON', () {
        final builder = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning');

        final json = builder.export();

        expect(json, isNotEmpty);
        // 应该是有效的 JSON
        expect(() => jsonDecode(json), returnsNormally);
      });

      test('import 应该从 JSON 恢复查询', () {
        final original = SearchQueryBuilder.create()
            .addTerm('error')
            .addTerm('warning')
            .setGlobalOperator(QueryOperator.and);

        final json = original.export();
        final restored = SearchQueryBuilder.import(json);
        final query = restored.toQuery();

        expect(query.terms, hasLength(2));
        expect(query.terms[0].value, 'error');
        expect(query.terms[1].value, 'warning');
        expect(query.globalOperator.value, 'AND');
      });

      test('import 无效 JSON 应该返回空构建器', () {
        final restored = SearchQueryBuilder.import('invalid json');

        expect(restored.termCount, 0);
      });
    });
  });
}
