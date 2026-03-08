import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/keyword.dart';
import '../services/api_service.dart';
import '../../core/constants/app_constants.dart';
import 'app_provider.dart';

part 'keyword_provider.g.dart';

/// 关键词组状态 Provider
///
/// 对应 React 版本的 keywordStore.ts
/// 管理关键词高亮组
@riverpod
class KeywordState extends _$KeywordState {
  @override
  List<KeywordGroup> build() {
    // 初始加载关键词组（延迟执行避免在 build 中直接调用异步方法）
    Future.microtask(() => loadKeywordGroups());
    return [];
  }

  /// 加载关键词组
  ///
  /// 从 Rust 后端获取所有关键词组
  Future<void> loadKeywordGroups() async {
    try {
      final apiService = ref.read(apiServiceProvider);
      final bridge = apiService.bridge;

      // 检查 FFI 是否可用
      if (!bridge.isFfiEnabled) {
        debugPrint('KeywordState: FFI 桥接不可用，跳过加载');
        return;
      }

      // 调用后端获取关键词组列表
      final groupsData = await bridge.getKeywords();

      // 转换为 KeywordGroup 模型
      final groups = groupsData.map<KeywordGroup>((data) {
        if (data is Map<String, dynamic>) {
          return _parseKeywordGroup(data);
        }
        // 尝试从 FFI 类型转换
        return _parseKeywordGroupFromFfi(data);
      }).toList();

      state = groups;
      debugPrint('KeywordState: 已加载 ${groups.length} 个关键词组');
    } catch (e) {
      debugPrint('KeywordState: 加载关键词组失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '加载关键词组失败: $e');
    }
  }

  /// 创建关键词组
  ///
  /// [name] 关键词组名称
  /// [color] 颜色键 (blue, green, red, orange, purple)
  /// [patterns] 关键词模式列表
  Future<String?> createKeywordGroup({
    required String name,
    required String color,
    required List<String> patterns,
    bool enabled = true,
  }) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      final bridge = apiService.bridge;

      // 构建关键词组数据
      final groupData = {
        'name': name,
        'color': color,
        'patterns': patterns,
        'enabled': enabled,
      };

      // 调用后端 API 创建关键词组
      final success = await bridge.addKeywordGroup(groupData);

      if (success) {
        // 重新加载关键词组列表
        await loadKeywordGroups();

        ref
            .read(appStateProvider.notifier)
            .addToast(ToastType.success, '关键词组 "$name" 创建成功');
      }

      return success ? name : null;
    } catch (e) {
      debugPrint('KeywordState: 创建关键词组失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '创建关键词组失败: $e');
      return null;
    }
  }

  /// 更新关键词组
  ///
  /// [groupId] 关键词组 ID
  /// [name] 新名称
  /// [color] 新颜色
  /// [patterns] 新模式列表
  /// [enabled] 是否启用
  Future<bool> updateKeywordGroupById({
    required String groupId,
    String? name,
    String? color,
    List<String>? patterns,
    bool? enabled,
  }) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      final bridge = apiService.bridge;

      // 获取当前关键词组
      final currentGroup = state.firstWhere(
        (g) => g.id == groupId,
        orElse: () => throw Exception('关键词组不存在'),
      );

      // 构建更新数据
      final groupData = {
        'name': name ?? currentGroup.name,
        'color': color ?? currentGroup.color.value,
        'patterns':
            patterns ?? currentGroup.patterns.map((p) => p.regex).toList(),
        'enabled': enabled ?? currentGroup.enabled,
      };

      // 调用后端 API 更新关键词组
      final success = await bridge.updateKeywordGroup(groupId, groupData);

      if (success) {
        // 更新本地状态
        state = state.map((g) {
          if (g.id == groupId) {
            return g.copyWith(
              name: groupData['name'] as String,
              color: ColorKeyData(value: groupData['color'] as String),
              patterns: (groupData['patterns'] as List)
                  .map((p) => KeywordPattern(regex: p as String, comment: ''))
                  .toList(),
              enabled: groupData['enabled'] as bool,
            );
          }
          return g;
        }).toList();

        ref
            .read(appStateProvider.notifier)
            .addToast(ToastType.success, '关键词组已更新');
      }

      return success;
    } catch (e) {
      debugPrint('KeywordState: 更新关键词组失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '更新关键词组失败: $e');
      return false;
    }
  }

  /// 删除关键词组
  ///
  /// [groupId] 关键词组 ID
  Future<bool> deleteKeywordGroupById(String groupId) async {
    try {
      final apiService = ref.read(apiServiceProvider);
      final bridge = apiService.bridge;

      // 先从本地状态移除（乐观更新）
      final previousState = state;
      state = state.where((g) => g.id != groupId).toList();

      // 调用后端 API 删除关键词组
      final success = await bridge.deleteKeywordGroup(groupId);

      if (success) {
        ref
            .read(appStateProvider.notifier)
            .addToast(ToastType.success, '关键词组已删除');
      } else {
        // 恢复状态
        state = previousState;
      }

      return success;
    } catch (e) {
      debugPrint('KeywordState: 删除关键词组失败: $e');
      ref
          .read(appStateProvider.notifier)
          .addToast(ToastType.error, '删除关键词组失败: $e');
      return false;
    }
  }

  /// 切换关键词组启用状态
  ///
  /// [groupId] 关键词组 ID
  Future<void> toggleKeywordGroupEnabled(String groupId) async {
    final group = state.firstWhere(
      (g) => g.id == groupId,
      orElse: () => throw Exception('关键词组不存在'),
    );

    await updateKeywordGroupById(groupId: groupId, enabled: !group.enabled);
  }

  /// 添加关键词组（本地操作，不调用后端）
  ///
  /// 用于接收后端事件通知时更新状态
  void addKeywordGroup(KeywordGroup group) {
    final exists = state.any((g) => g.id == group.id);
    if (!exists) {
      state = [...state, group];
    }
  }

  /// 更新关键词组（本地操作）
  ///
  /// 用于接收后端事件通知时更新状态
  void updateKeywordGroup(KeywordGroup updated) {
    state = state.map((g) => g.id == updated.id ? updated : g).toList();
  }

  /// 删除关键词组（本地操作）
  ///
  /// 用于接收后端事件通知时更新状态
  void removeKeywordGroup(String id) {
    state = state.where((g) => g.id != id).toList();
  }

  /// 切换关键词组启用状态（本地操作）
  void toggleKeywordGroup(String id) {
    state = state.map((g) {
      if (g.id == id) {
        return g.copyWith(enabled: !g.enabled);
      }
      return g;
    }).toList();
  }

  /// 重新排序关键词组
  ///
  /// [oldIndex] 原始位置
  /// [newIndex] 新位置
  void reorderKeywordGroups(int oldIndex, int newIndex) {
    // ReorderableListView 在移动到末尾时 newIndex 会减 1
    if (newIndex > oldIndex) {
      newIndex -= 1;
    }

    final newList = List<KeywordGroup>.from(state);
    final item = newList.removeAt(oldIndex);
    newList.insert(newIndex, item);
    state = newList;
  }

  /// 复制关键词组
  ///
  /// 创建一个新的关键词组副本，名称添加 " (副本)" 后缀
  KeywordGroup duplicateKeywordGroup(String id) {
    final original = state.firstWhere((g) => g.id == id);
    final duplicated = original.copyWith(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      name: '${original.name} (副本)',
    );
    state = [...state, duplicated];
    return duplicated;
  }

  /// 导入关键词组配置
  ///
  /// 从 JSON 字符串导入关键词组列表
  /// 返回导入的数量
  int importFromJson(String jsonString) {
    try {
      final List<dynamic> jsonList = jsonDecode(jsonString);
      final groups = jsonList
          .map((json) => KeywordGroup.fromJson(json as Map<String, dynamic>))
          .toList();

      // 为导入的组分配新 ID，避免冲突
      final newGroups = groups
          .map(
            (g) => g.copyWith(
              id: '${g.id}_${DateTime.now().millisecondsSinceEpoch}',
            ),
          )
          .toList();

      state = [...state, ...newGroups];
      return newGroups.length;
    } catch (e) {
      throw FormatException('解析关键词配置失败: $e');
    }
  }

  /// 导出关键词组配置
  ///
  /// 将当前关键词组列表导出为 JSON 字符串
  String exportToJson() {
    return const JsonEncoder.withIndent(
      '  ',
    ).convert(state.map((g) => g.toJson()).toList());
  }

  /// 获取启用的关键词组
  List<KeywordGroup> get enabledGroups =>
      state.where((g) => g.enabled).toList();

  /// 获取所有启用的关键词模式（用于搜索高亮）
  List<KeywordPattern> get enabledPatterns {
    return enabledGroups.expand((g) => g.patterns).toList();
  }

  /// 根据颜色获取关键词组
  List<KeywordGroup> getGroupsByColor(String color) {
    return state.where((g) => g.color.value == color).toList();
  }

  /// 根据 ID 获取关键词组
  KeywordGroup? getKeywordGroupById(String id) {
    try {
      return state.firstWhere((g) => g.id == id);
    } catch (e) {
      return null;
    }
  }

  // ==================== 私有辅助方法 ====================

  /// 从 Map 解析 KeywordGroup
  KeywordGroup _parseKeywordGroup(Map<String, dynamic> data) {
    final patterns =
        (data['patterns'] as List?)?.map((p) {
          if (p is String) {
            return KeywordPattern(regex: p, comment: '');
          } else if (p is Map<String, dynamic>) {
            return KeywordPattern(
              regex: p['regex'] as String? ?? '',
              comment: p['comment'] as String? ?? '',
            );
          }
          return const KeywordPattern(regex: '', comment: '');
        }).toList() ??
        <KeywordPattern>[];

    return KeywordGroup(
      id: data['id'] as String? ?? '',
      name: data['name'] as String? ?? '',
      color: ColorKeyData(value: data['color'] as String? ?? 'blue'),
      patterns: patterns,
      enabled: data['enabled'] as bool? ?? true,
    );
  }

  /// 从 FFI 数据类型解析 KeywordGroup
  KeywordGroup _parseKeywordGroupFromFfi(dynamic data) {
    // 处理 flutter_rust_bridge 生成的 KeywordGroupData 类型
    try {
      // 转换为 Map 以避免动态调用
      final mapData = data as Map<String, dynamic>;
      final id = mapData['id'] as String? ?? '';
      final name = mapData['name'] as String? ?? '';
      final color = mapData['color'] as String? ?? 'blue';
      final patterns =
          (mapData['patterns'] as List?)?.map((p) {
            return KeywordPattern(regex: p as String, comment: '');
          }).toList() ??
          <KeywordPattern>[];
      final enabled = mapData['enabled'] as bool? ?? true;

      return KeywordGroup(
        id: id,
        name: name,
        color: ColorKeyData(value: color),
        patterns: patterns,
        enabled: enabled,
      );
    } catch (e) {
      debugPrint('KeywordState: 解析 FFI 数据失败: $e');
    }

    // 返回默认空关键词组
    return const KeywordGroup(
      id: '',
      name: '',
      color: ColorKeyData(value: 'blue'),
      patterns: [],
      enabled: false,
    );
  }
}

/// 关键词加载状态 Provider
///
/// 管理关键词组的加载状态
@riverpod
class KeywordLoading extends _$KeywordLoading {
  @override
  bool build() {
    return false;
  }

  /// 设置加载状态
  void setLoading(bool loading) {
    state = loading;
  }
}
