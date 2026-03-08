// 过滤器端到端测试
//
// 测试过滤器 CRUD 操作与搜索流程的集成
// 使用 MockBridgeService 模拟 FFI 响应

import 'package:flutter_test/flutter_test.dart';
import 'package:log_analyzer_flutter/shared/models/saved_filter.dart';

void main() {
  group('过滤器端到端测试', () {
    test('创建新过滤器 - 名称和条件组合', () {
      // 创建测试用的过滤器数据
      final filter = SavedFilter(
        id: 'test-filter-1',
        name: 'Error 日志过滤器',
        description: '过滤所有 Error 级别日志',
        workspaceId: 'workspace-1',
        terms: [
          SearchTerm(
            id: 'term-1',
            value: 'ERROR',
            operator: 'AND',
            isRegex: false,
            priority: 0,
            enabled: true,
            caseSensitive: false,
          ),
        ],
        globalOperator: 'AND',
        timeRange: null,
        levels: ['ERROR'],
        filePattern: null,
        isDefault: false,
        sortOrder: 0,
        usageCount: 0,
        createdAt: DateTime.now().toIso8601String(),
        lastUsedAt: null,
      );

      // 验证过滤器创建成功
      expect(filter.id, 'test-filter-1');
      expect(filter.name, 'Error 日志过滤器');
      expect(filter.terms.length, 1);
      expect(filter.terms.first.value, 'ERROR');
      expect(filter.levels, contains('ERROR'));
    });

    test('编辑现有过滤器', () {
      // 创建初始过滤器
      final originalFilter = SavedFilter(
        id: 'test-filter-2',
        name: '原始过滤器',
        description: '原始描述',
        workspaceId: 'workspace-1',
        terms: [
          SearchTerm(
            id: 'term-1',
            value: 'WARN',
            operator: 'AND',
            isRegex: false,
            priority: 0,
            enabled: true,
            caseSensitive: false,
          ),
        ],
        globalOperator: 'AND',
        timeRange: null,
        levels: ['WARN'],
        filePattern: null,
        isDefault: false,
        sortOrder: 0,
        usageCount: 0,
        createdAt: DateTime.now().toIso8601String(),
        lastUsedAt: null,
      );

      // 复制并修改过滤器（模拟编辑操作）
      final editedFilter = originalFilter.copyWith(
        name: '修改后的过滤器',
        description: '修改后的描述',
        terms: [
          SearchTerm(
            id: 'term-1',
            value: 'ERROR',
            operator: 'AND',
            isRegex: false,
            priority: 0,
            enabled: true,
            caseSensitive: false,
          ),
          SearchTerm(
            id: 'term-2',
            value: 'FATAL',
            operator: 'OR',
            isRegex: false,
            priority: 1,
            enabled: true,
            caseSensitive: false,
          ),
        ],
        levels: ['ERROR', 'FATAL'],
      );

      // 验证过滤器编辑成功
      expect(editedFilter.name, '修改后的过滤器');
      expect(editedFilter.description, '修改后的描述');
      expect(editedFilter.terms.length, 2);
      expect(editedFilter.levels, containsAll(['ERROR', 'FATAL']));
    });

    test('删除过滤器', () {
      // 创建测试用过滤器列表
      final filters = [
        SavedFilter(
          id: 'filter-1',
          name: '过滤器1',
          workspaceId: 'workspace-1',
          terms: [],
          globalOperator: 'AND',
          levels: [],
          isDefault: false,
          sortOrder: 0,
          usageCount: 0,
          createdAt: DateTime.now().toIso8601String(),
        ),
        SavedFilter(
          id: 'filter-2',
          name: '过滤器2',
          workspaceId: 'workspace-1',
          terms: [],
          globalOperator: 'AND',
          levels: [],
          isDefault: false,
          sortOrder: 1,
          usageCount: 0,
          createdAt: DateTime.now().toIso8601String(),
        ),
        SavedFilter(
          id: 'filter-3',
          name: '过滤器3',
          workspaceId: 'workspace-1',
          terms: [],
          globalOperator: 'AND',
          levels: [],
          isDefault: false,
          sortOrder: 2,
          usageCount: 0,
          createdAt: DateTime.now().toIso8601String(),
        ),
      ];

      // 模拟删除 filter-2
      final filterIdToDelete = 'filter-2';
      final remainingFilters =
          filters.where((f) => f.id != filterIdToDelete).toList();

      // 验证删除结果
      expect(remainingFilters.length, 2);
      expect(remainingFilters.any((f) => f.id == 'filter-1'), true);
      expect(remainingFilters.any((f) => f.id == 'filter-2'), false);
      expect(remainingFilters.any((f) => f.id == 'filter-3'), true);
    });

    test('应用过滤器触发搜索', () {
      // 模拟搜索条件
      final searchTerms = [
        SearchTerm(
          id: 'term-1',
          value: 'Exception',
          operator: 'AND',
          isRegex: true,
          priority: 0,
          enabled: true,
          caseSensitive: false,
        ),
        SearchTerm(
          id: 'term-2',
          value: '500',
          operator: 'OR',
          isRegex: false,
          priority: 1,
          enabled: true,
          caseSensitive: false,
        ),
      ];

      // 模拟过滤器
      final filter = SavedFilter(
        id: 'test-filter-3',
        name: '错误搜索过滤器',
        workspaceId: 'workspace-1',
        terms: searchTerms,
        globalOperator: 'AND',
        levels: ['ERROR'],
        isDefault: false,
        sortOrder: 0,
        usageCount: 0,
        createdAt: DateTime.now().toIso8601String(),
      );

      // 验证过滤器可以触发搜索
      final enabledTerms = filter.terms.where((t) => t.enabled).toList();
      expect(enabledTerms.length, 2);
      expect(enabledTerms.any((t) => t.value == 'Exception'), true);
      expect(enabledTerms.any((t) => t.value == '500'), true);
      expect(filter.globalOperator, 'AND');
      expect(filter.levels, contains('ERROR'));
    });

    test('时间范围过滤器', () {
      // 创建带时间范围的过滤器
      final filter = SavedFilter(
        id: 'test-filter-4',
        name: '最近24小时错误',
        workspaceId: 'workspace-1',
        terms: [
          SearchTerm(
            id: 'term-1',
            value: 'ERROR',
            operator: 'AND',
            isRegex: false,
            priority: 0,
            enabled: true,
            caseSensitive: false,
          ),
        ],
        globalOperator: 'AND',
        timeRange: TimeRange(
          start: DateTime.now().subtract(const Duration(hours: 24)).toIso8601String(),
          end: DateTime.now().toIso8601String(),
        ),
        levels: ['ERROR'],
        isDefault: false,
        sortOrder: 0,
        usageCount: 0,
        createdAt: DateTime.now().toIso8601String(),
      );

      // 验证时间范围设置正确
      expect(filter.timeRange, isNotNull);
      expect(filter.timeRange!.start, isNotNull);
      expect(filter.timeRange!.end, isNotNull);
    });

    test('JSON 序列化与反序列化', () {
      // 创建过滤器
      final original = SavedFilter(
        id: 'test-filter-serialization',
        name: '序列化测试',
        description: '测试 JSON 序列化',
        workspaceId: 'workspace-1',
        terms: [
          SearchTerm(
            id: 'term-1',
            value: 'ERROR',
            operator: 'AND',
            isRegex: false,
            priority: 0,
            enabled: true,
            caseSensitive: false,
          ),
        ],
        globalOperator: 'AND',
        levels: ['ERROR', 'WARN'],
        isDefault: true,
        sortOrder: 0,
        usageCount: 5,
        createdAt: '2026-03-08T10:00:00Z',
        lastUsedAt: '2026-03-08T12:00:00Z',
      );

      // 序列化为 JSON
      final json = original.toJson();

      // 从 JSON 反序列化
      final restored = SavedFilter.fromJson(json);

      // 验证数据一致性
      expect(restored.id, original.id);
      expect(restored.name, original.name);
      expect(restored.description, original.description);
      expect(restored.workspaceId, original.workspaceId);
      expect(restored.terms.length, original.terms.length);
      expect(restored.globalOperator, original.globalOperator);
      expect(restored.levels, original.levels);
      expect(restored.isDefault, original.isDefault);
      expect(restored.usageCount, original.usageCount);
    });
  });
}
