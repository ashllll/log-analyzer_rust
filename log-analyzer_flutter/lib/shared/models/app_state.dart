import 'package:freezed_annotation/freezed_annotation.dart';

import '../../core/constants/app_constants.dart';

part 'app_state.freezed.dart';
part 'app_state.g.dart';

/// Toast 消息模型
///
/// 对应 React 版本的 Toast 接口
@freezed
abstract class Toast with _$Toast {
  const factory Toast({
    required String id,
    required ToastType type,
    required String message,
    int? duration,
  }) = _Toast;

  factory Toast.fromJson(Map<String, dynamic> json) => _$ToastFromJson(json);
}

/// 应用状态模型
///
/// 对应 React 版本的 AppState 接口
/// 使用 Riverpod 管理全局状态
@freezed
abstract class AppModel with _$AppModel {
  const factory AppModel({
    @Default(AppPage.search) AppPage currentPage,
    @Default([]) List<Toast> toasts,
    String? activeWorkspaceId,
    @Default(false) bool isInitialized,
    String? initializationError,
  }) = _AppModel;

  factory AppModel.fromJson(Map<String, dynamic> json) =>
      _$AppModelFromJson(json);
}
