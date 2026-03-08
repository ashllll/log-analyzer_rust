import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:uuid/uuid.dart';

import '../../../shared/services/bridge_service.dart';
import '../../../shared/services/generated/ffi/types.dart' as ffi_types;

part 'search_query_provider.g.dart';

/// 本地搜索条件模型
///
/// 用于 Riverpod 状态管理的本地 Dart 模型
/// 对应 FFI 的 SearchTermData 结构体
class SearchTerm {
  /// 条件唯一标识
  final String id;

  /// 搜索值/关键词
  final String value;

  /// 该条件的操作符
  final ffi_types.QueryOperatorData operator_;

  /// 是否为正则表达式
  final bool isRegex;

  /// 优先级（数字越小优先级越高）
  final int priority;

  /// 是否启用该条件
  final bool enabled;

  /// 是否大小写敏感
  final bool caseSensitive;

  const SearchTerm({
    required this.id,
    required this.value,
    required this.operator_,
    this.isRegex = false,
    this.priority = 0,
    this.enabled = true,
    this.caseSensitive = false,
  });

  /// 从 FFI SearchTermData 创建
  factory SearchTerm.fromFfi(ffi_types.SearchTermData data) {
    return SearchTerm(
      id: data.id,
      value: data.value,
      operator_: data.operator_,
      isRegex: data.isRegex,
      priority: data.priority,
      enabled: data.enabled,
      caseSensitive: data.caseSensitive,
    );
  }

  /// 转换为 FFI SearchTermData
  ffi_types.SearchTermData toFfi() {
    return ffi_types.SearchTermData(
      id: id,
      value: value,
      operator_: operator_,
      isRegex: isRegex,
      priority: priority,
      enabled: enabled,
      caseSensitive: caseSensitive,
    );
  }

  /// 复制并修改部分字段
  SearchTerm copyWith({
    String? id,
    String? value,
    ffi_types.QueryOperatorData? operator_,
    bool? isRegex,
    int? priority,
    bool? enabled,
    bool? caseSensitive,
  }) {
    return SearchTerm(
      id: id ?? this.id,
      value: value ?? this.value,
      operator_: operator_ ?? this.operator_,
      isRegex: isRegex ?? this.isRegex,
      priority: priority ?? this.priority,
      enabled: enabled ?? this.enabled,
      caseSensitive: caseSensitive ?? this.caseSensitive,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SearchTerm &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          value == other.value &&
          operator_ == other.operator_ &&
          isRegex == other.isRegex &&
          priority == other.priority &&
          enabled == other.enabled &&
          caseSensitive == other.caseSensitive;

  @override
  int get hashCode =>
      id.hashCode ^
      value.hashCode ^
      operator_.hashCode ^
      isRegex.hashCode ^
      priority.hashCode ^
      enabled.hashCode ^
      caseSensitive.hashCode;

  @override
  String toString() =>
      'SearchTerm(id: $id, value: $value, operator: $operator_, enabled: $enabled)';
}

/// BridgeService Provider
@riverpod
BridgeService bridgeServiceForQuery(Ref ref) {
  return BridgeService.instance;
}

/// 搜索关键词数量 Provider
///
/// 使用 select() 只在关键词数量变化时重建
/// 避免整个状态变化时的不必要重建
@riverpod
int searchTermCount(Ref ref) {
  return ref.watch(searchQueryProvider).terms.length;
}

/// 是否有搜索关键词
///
/// 使用 select() 只在关键词存在性变化时重建
@riverpod
bool hasSearchKeywords(Ref ref) {
  return ref.watch(searchQueryProvider).terms.any((t) => t.enabled);
}

/// 搜索关键词列表 Provider (只读)
///
/// 使用 select() 获取关键词列表，避免重建
@riverpod
List<SearchTerm> searchTerms(Ref ref) {
  return ref.watch(searchQueryProvider).terms;
}

/// 搜索查询状态 Provider
///
/// 使用 Riverpod 3.0 Notifier 管理多关键词组合搜索状态
/// 支持 AND/OR/NOT 逻辑操作符
@riverpod
class SearchQuery extends _$SearchQuery {
  /// 全局操作符
  ffi_types.QueryOperatorData _globalOperator = ffi_types.QueryOperatorData.and;

  @override
  SearchQueryState build() {
    return SearchQueryState(terms: const [], globalOperator: _globalOperator);
  }

  /// 获取当前关键词列表
  List<SearchTerm> get terms => state.terms;

  /// 获取当前全局操作符
  ffi_types.QueryOperatorData get globalOperator => state.globalOperator;

  /// 添加关键词
  ///
  /// 使用 uuid 生成唯一 ID，自动设置优先级
  void addKeyword(String value) {
    final trimmedValue = value.trim();
    if (trimmedValue.isEmpty) return;

    // 检查是否已存在相同关键词
    if (terms.any((t) => t.value == trimmedValue)) {
      debugPrint('SearchQueryProvider: 关键词 "$trimmedValue" 已存在，跳过添加');
      return;
    }

    final newTerm = SearchTerm(
      id: const Uuid().v4(),
      value: trimmedValue,
      operator_: _globalOperator,
      isRegex: false,
      priority: terms.length,
      enabled: true,
      caseSensitive: false,
    );

    state = state.copyWith(terms: [...terms, newTerm]);

    debugPrint(
      'SearchQueryProvider: 已添加关键词 "${newTerm.value}"，共 ${terms.length} 个',
    );
  }

  /// 删除关键词
  ///
  /// 根据 ID 删除指定关键词
  void removeKeyword(String id) {
    final previousCount = terms.length;
    state = state.copyWith(terms: terms.where((t) => t.id != id).toList());

    debugPrint(
      'SearchQueryProvider: 已删除关键词 ID "$id"，剩余 ${terms.length} 个（之前 $previousCount 个）',
    );
  }

  /// 更新关键词
  ///
  /// 根据 ID 更新指定关键词的值
  void updateKeyword(String id, String newValue) {
    final trimmedValue = newValue.trim();
    if (trimmedValue.isEmpty) return;

    state = state.copyWith(
      terms: terms.map((t) {
        if (t.id == id) {
          return t.copyWith(value: trimmedValue);
        }
        return t;
      }).toList(),
    );

    debugPrint('SearchQueryProvider: 已更新关键词 ID "$id" 为 "$trimmedValue"');
  }

  /// 切换关键词启用状态
  ///
  /// 切换指定关键词的启用/禁用状态
  void toggleKeyword(String id) {
    state = state.copyWith(
      terms: terms.map((t) {
        if (t.id == id) {
          return t.copyWith(enabled: !t.enabled);
        }
        return t;
      }).toList(),
    );
  }

  /// 设置全局操作符
  ///
  /// 设置所有关键词的组合逻辑（AND/OR/NOT）
  void setGlobalOperator(ffi_types.QueryOperatorData operator_) {
    _globalOperator = operator_;
    state = state.copyWith(globalOperator: operator_);

    debugPrint('SearchQueryProvider: 已设置全局操作符为 $operator_');
  }

  /// 清空所有关键词
  void clearKeywords() {
    state = state.copyWith(terms: const []);
    debugPrint('SearchQueryProvider: 已清空所有关键词');
  }

  /// 构建 FFI 结构化搜索查询对象
  ///
  /// 将当前状态转换为 FFI 可用的 StructuredSearchQueryData
  Future<ffi_types.StructuredSearchQueryData> buildQuery() async {
    // 只包含启用的关键词
    final enabledTerms = terms.where((t) => t.enabled).toList();

    // 转换为 FFI 类型
    final ffiTerms = enabledTerms.map((t) => t.toFfi()).toList();

    return ffi_types.StructuredSearchQueryData(
      terms: ffiTerms,
      globalOperator: _globalOperator,
    );
  }

  /// 获取关键词值列表
  ///
  /// 返回所有启用的关键词值字符串列表
  List<String> getKeywordValues() {
    return terms.where((t) => t.enabled).map((t) => t.value).toList();
  }

  /// 是否有可搜索的关键词
  bool get hasKeywords => terms.where((t) => t.enabled).isNotEmpty;

  /// 获取启用的关键词数量
  int get enabledCount => terms.where((t) => t.enabled).length;

  /// 构建预览文本
  ///
  /// 返回格式化的搜索条件预览字符串
  String buildPreviewText() {
    final enabledTerms = terms.where((t) => t.enabled).toList();
    if (enabledTerms.isEmpty) {
      return '无搜索条件';
    }

    final opStr = _getOperatorString(_globalOperator);
    return enabledTerms.map((t) => t.value).join(opStr);
  }

  /// 获取操作符显示字符串
  String _getOperatorString(ffi_types.QueryOperatorData op) {
    switch (op) {
      case ffi_types.QueryOperatorData.and:
        return ' AND ';
      case ffi_types.QueryOperatorData.or:
        return ' OR ';
      case ffi_types.QueryOperatorData.not:
        return ' NOT ';
    }
  }
}

/// 搜索查询状态
///
/// 包含关键词列表和全局操作符的不可变状态对象
class SearchQueryState {
  /// 搜索条件列表
  final List<SearchTerm> terms;

  /// 全局操作符
  final ffi_types.QueryOperatorData globalOperator;

  const SearchQueryState({required this.terms, required this.globalOperator});

  /// 复制并修改部分字段
  SearchQueryState copyWith({
    List<SearchTerm>? terms,
    ffi_types.QueryOperatorData? globalOperator,
  }) {
    return SearchQueryState(
      terms: terms ?? this.terms,
      globalOperator: globalOperator ?? this.globalOperator,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SearchQueryState &&
          runtimeType == other.runtimeType &&
          _listEquals(terms, other.terms) &&
          globalOperator == other.globalOperator;

  bool _listEquals(List<SearchTerm> a, List<SearchTerm> b) {
    if (a.length != b.length) return false;
    for (int i = 0; i < a.length; i++) {
      if (a[i] != b[i]) return false;
    }
    return true;
  }

  @override
  int get hashCode => terms.hashCode ^ globalOperator.hashCode;
}
