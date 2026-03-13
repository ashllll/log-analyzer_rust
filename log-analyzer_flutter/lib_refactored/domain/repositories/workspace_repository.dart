/// 工作区仓库接口
/// 
/// Clean Architecture 中的仓库接口定义
/// 数据层将实现这些接口

import 'package:fpdart/fpdart.dart';
import '../../core/errors/app_error.dart';
import '../entities/workspace.dart';

/// 工作区仓库接口
/// 
/// 所有方法返回 TaskEither<AppError, T> 以支持函数式错误处理
/// 异步操作在实现层使用 Isolate 避免阻塞 UI
abstract class WorkspaceRepository {
  /// 获取所有工作区
  /// 
  /// 返回按最后更新时间排序的工作区列表
  AppTask<List<Workspace>> getWorkspaces();

  /// 根据 ID 获取工作区
  AppTask<Workspace?> getWorkspaceById(String id);

  /// 根据路径获取工作区
  AppTask<Workspace?> getWorkspaceByPath(String path);

  /// 创建工作区
  /// 
  /// 失败可能原因：
  /// - 路径无效
  /// - 路径已被其他工作区使用
  /// - FFI 调用失败
  AppTask<Workspace> createWorkspace(CreateWorkspaceParams params);

  /// 删除工作区
  /// 
  /// 仅删除数据库记录，不删除实际文件
  AppTask<void> deleteWorkspace(String id);

  /// 刷新工作区
  /// 
  /// 重新扫描文件夹并更新索引
  /// 返回任务 ID，用于跟踪刷新进度
  AppTask<String> refreshWorkspace(RefreshWorkspaceParams params);

  /// 更新工作区
  AppTask<Workspace> updateWorkspace(Workspace workspace);

  /// 获取工作区统计信息
  AppTask<WorkspaceStats> getWorkspaceStats(String id);

  /// 启动文件监听
  AppTask<void> startWatching(String workspaceId, {required List<String> paths, required bool recursive});

  /// 停止文件监听
  AppTask<void> stopWatching(String workspaceId);

  /// 检查是否正在监听
  AppTask<bool> isWatching(String workspaceId);

  /// 导入文件夹到工作区
  /// 
  /// 返回任务 ID，用于跟踪导入进度
  AppTask<String> importFolder(String workspaceId, String path);

  /// 监听工作区变化
  /// 
  /// 返回工作区列表的实时更新流
  Stream<List<Workspace>> watchWorkspaces();
}
