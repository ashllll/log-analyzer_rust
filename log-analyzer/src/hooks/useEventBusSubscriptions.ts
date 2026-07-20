import { useEffect } from "react";
import { useWorkspaceStore } from "../stores/workspaceStore";
import { useTaskStore } from "../stores/taskStore";
import { eventBus } from "../events/EventBus";
import { describeWorkspaceStatusChange } from "../events/workspaceStatusProjection";
import { useWorkspaceList } from "./useWorkspaceList";
import { useToast } from "./useToast";
import type {
  TaskUpdateEvent,
  TaskRemovedEvent,
  WorkspaceEvent,
} from "../events/types";
import { logger } from "../utils/logger";

/**
 * EventBus 订阅 Hook
 *
 * 负责注册应用层 EventBus 事件处理器：
 * - task-update: 任务状态更新（upsert 合并操作、工作区状态联动、toast 通知）
 * - task-removed: 任务自动清理
 * - workspace-event: 工作区状态变更（统一处理，替代 App.tsx 中的直接监听）
 *
 */
export const useEventBusSubscriptions = () => {
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const upsertTask = useTaskStore((state) => state.upsertTask);
  const deleteTask = useTaskStore((state) => state.deleteTask);
  const { showToast } = useToast();

  // 复用 useWorkspaceList 的 refreshWorkspaces，避免重复定义和全量加载
  const { refreshWorkspaces } = useWorkspaceList();

  useEffect(() => {
    // 注册任务更新事件处理器
    const unsubscribeTaskUpdate = eventBus.on<TaskUpdateEvent>(
      "task-update",
      (event) => {
        const task = {
          id: event.task_id,
          type: event.task_type,
          target: event.target,
          progress: event.progress,
          message: event.message,
          status: event.status,
          workspaceId: event.workspace_id,
        };

        logger.debug(
          { task },
          "[EventBusSubscriptions] Processing task update"
        );

        // 使用 upsert 合并添加/更新为单次操作
        upsertTask(task);

        // 更新工作区状态并发送 toast 通知
        if (task.workspaceId) {
          if (task.status === "COMPLETED") {
            updateWorkspace(task.workspaceId, { status: "READY" });
            showToast("success", "导入完成");
          } else if (task.status === "RUNNING") {
            updateWorkspace(task.workspaceId, { status: "PROCESSING" });
          } else if (task.status === "FAILED") {
            updateWorkspace(task.workspaceId, { status: "OFFLINE" });
          }
        }
      }
    );

    // 注册任务移除事件处理器
    const unsubscribeTaskRemoved = eventBus.on<TaskRemovedEvent>(
      "task-removed",
      (event) => {
        logger.info(
          { taskId: event.task_id },
          "[EventBusSubscriptions] Auto-removing task"
        );
        deleteTask(event.task_id);
      }
    );

    // 注册工作区事件处理器（统一处理工作区状态变更）
    const unsubscribeWorkspaceEvent = eventBus.on<WorkspaceEvent>(
      "workspace-event",
      (event) => {
        logger.debug(
          { event },
          "[EventBusSubscriptions] Processing workspace event"
        );

        switch (event.type) {
          case "StatusChanged": {
            const toast = describeWorkspaceStatusChange(event.status);
            if (toast) {
              showToast(toast.toastType, toast.message);
            }

            // 刷新工作区列表以同步最新状态
            refreshWorkspaces();
            break;
          }
          case "FilesUpdated": {
            // Watch 模式：静默刷新，不发 toast（高频日志下 toast 淹没用户）
            refreshWorkspaces();
            break;
          }
        }
      }
    );

    return () => {
      unsubscribeTaskUpdate();
      unsubscribeTaskRemoved();
      unsubscribeWorkspaceEvent();
    };
  }, [upsertTask, deleteTask, updateWorkspace, refreshWorkspaces, showToast]);
};
