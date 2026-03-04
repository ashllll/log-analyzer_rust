import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'import_progress_provider.g.dart';

/// 导入状态枚举
enum ImportStatus {
  /// 空闲状态
  idle,
  /// 导入中
  importing,
  /// 已暂停
  paused,
  /// 已完成
  completed,
  /// 已取消
  cancelled,
  /// 失败
  failed,
}

/// 导入进度状态
class ImportProgressState {
  /// 总文件数
  final int totalFiles;

  /// 已处理文件数
  final int processedFiles;

  /// 当前正在处理的文件名
  final String currentFile;

  /// 进度百分比 (0.0-1.0 或 0-100)
  final double progressPercent;

  /// 错误列表
  final List<String> errors;

  /// 当前状态
  final ImportStatus status;

  /// 任务ID (用于取消)
  final String? taskId;

  /// 开始时间
  final DateTime? startTime;

  /// 预估剩余时间 (秒)
  final int? estimatedRemainingSeconds;

  /// 处理速度 (文件/秒)
  final double? filesPerSecond;

  const ImportProgressState({
    this.totalFiles = 0,
    this.processedFiles = 0,
    this.currentFile = '',
    this.progressPercent = 0,
    this.errors = const [],
    this.status = ImportStatus.idle,
    this.taskId,
    this.startTime,
    this.estimatedRemainingSeconds,
    this.filesPerSecond,
  });

  /// 复制方法
  ImportProgressState copyWith({
    int? totalFiles,
    int? processedFiles,
    String? currentFile,
    double? progressPercent,
    List<String>? errors,
    ImportStatus? status,
    String? taskId,
    DateTime? startTime,
    int? estimatedRemainingSeconds,
    double? filesPerSecond,
  }) {
    return ImportProgressState(
      totalFiles: totalFiles ?? this.totalFiles,
      processedFiles: processedFiles ?? this.processedFiles,
      currentFile: currentFile ?? this.currentFile,
      progressPercent: progressPercent ?? this.progressPercent,
      errors: errors ?? this.errors,
      status: status ?? this.status,
      taskId: taskId ?? this.taskId,
      startTime: startTime ?? this.startTime,
      estimatedRemainingSeconds: estimatedRemainingSeconds ?? this.estimatedRemainingSeconds,
      filesPerSecond: filesPerSecond ?? this.filesPerSecond,
    );
  }
}

/// 导入进度 Provider
///
/// 类型别名，用于测试兼容
typedef ImportProgressNotifier = ImportProgress;

/// 导入进度 Provider
@riverpod
class ImportProgress extends _$ImportProgress {
  @override
  ImportProgressState build() {
    return const ImportProgressState();
  }

  /// 更新导入进度
  void updateProgress({
    required int totalFiles,
    required int processedFiles,
    String? currentFile,
    List<String>? errors,
  }) {
    // 计算进度百分比 (0.0 - 1.0)
    final progressPercent = totalFiles > 0
        ? (processedFiles / totalFiles)
        : 0.0;

    // 计算处理速度
    double? filesPerSecond;
    int? estimatedRemainingSeconds;

    if (state.startTime != null && processedFiles > 0) {
      final elapsed = DateTime.now().difference(state.startTime!).inSeconds;
      if (elapsed > 0) {
        filesPerSecond = processedFiles / elapsed;
        if (filesPerSecond! > 0) {
          estimatedRemainingSeconds = ((totalFiles - processedFiles) / filesPerSecond!).round();
        }
      }
    }

    state = state.copyWith(
      totalFiles: totalFiles,
      processedFiles: processedFiles,
      currentFile: currentFile,
      progressPercent: progressPercent,
      errors: errors,
      filesPerSecond: filesPerSecond,
      estimatedRemainingSeconds: estimatedRemainingSeconds,
    );
  }

  /// 开始导入
  void startImport({
    required String taskId,
    required int totalFiles,
  }) {
    state = ImportProgressState(
      totalFiles: totalFiles,
      processedFiles: 0,
      status: ImportStatus.importing,
      taskId: taskId,
      startTime: DateTime.now(),
    );
  }

  /// 暂停导入
  void pauseImport() {
    if (state.status == ImportStatus.importing) {
      state = state.copyWith(status: ImportStatus.paused);
    }
  }

  /// 继续导入
  void resumeImport() {
    if (state.status == ImportStatus.paused) {
      state = state.copyWith(status: ImportStatus.importing);
    }
  }

  /// 取消导入
  void cancelImport() {
    state = state.copyWith(status: ImportStatus.cancelled);
  }

  /// 完成导入
  void completeImport() {
    state = state.copyWith(
      status: ImportStatus.completed,
      progressPercent: 1.0,
      processedFiles: state.totalFiles,
    );
  }

  /// 导入失败
  void failImport(String error) {
    state = state.copyWith(
      status: ImportStatus.failed,
      errors: [...state.errors, error],
    );
  }

  /// 添加错误
  void addError(String error) {
    state = state.copyWith(
      errors: [...state.errors, error],
    );
  }

  /// 重置状态
  void reset() {
    state = const ImportProgressState();
  }
}
