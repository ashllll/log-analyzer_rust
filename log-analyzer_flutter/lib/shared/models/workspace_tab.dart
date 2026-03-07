import 'package:freezed_annotation/freezed_annotation.dart';

part 'workspace_tab.freezed.dart';
part 'workspace_tab.g.dart';

/// 工作区标签页模型
///
/// 存储每个标签页的元数据和状态
@freezed
abstract class WorkspaceTab with _$WorkspaceTab {
  const factory WorkspaceTab({
    required String id,           // 唯一标签页 ID
    required String workspaceId,   // 工作区 ID
    required String title,         // 显示标题 (工作区名称)
    required DateTime openedAt,   // 打开时间
    @Default(false) bool isPinned, // 是否固定
  }) = _WorkspaceTab;

  factory WorkspaceTab.fromJson(Map<String, dynamic> json) =>
      _$WorkspaceTabFromJson(json);
}

/// 标签页状态 - 用于保存每个标签页的独立状态
@freezed
abstract class TabState with _$TabState {
  const factory TabState({
    @Default({}) Map<String, dynamic> searchQuery,     // 搜索条件
    @Default([]) List<String> expandedFolders,         // 展开的文件夹
    @Default('') String selectedFile,                  // 当前选中的文件
    @Default({}) Map<String, dynamic> filterOptions,  // 过滤选项
  }) = _TabState;

  factory TabState.fromJson(Map<String, dynamic> json) =>
      _$TabStateFromJson(json);
}
