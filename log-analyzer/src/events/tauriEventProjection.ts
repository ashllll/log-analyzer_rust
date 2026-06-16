import { listen } from "@tauri-apps/api/event";
import { eventBus } from "./EventBus";
import { logger } from "../utils/logger";
import type { TaskUpdateEvent, TaskRemovedEvent } from "./types";
import type { Task, Workspace } from "../stores/types";

export interface TauriEventProjectionOptions {
  updateWorkspace: (id: string, updates: Partial<Workspace>) => void;
  updateTask: (id: string, updates: Partial<Task>) => void;
  showToast: (type: "error" | "info", message: string) => void;
  debouncedUpdateWorkspace: (
    workspaceId: string,
    readyCount: number,
    totalCount: number
  ) => void;
  getTasks: () => Task[];
  getWorkspaces: () => Workspace[];
}

/**
 * 挂载 Tauri 事件投影：把后端原始事件订阅转换为前端 store / toast / EventBus 动作。
 *
 * 规则与原始 hook 保持一致：
 * - task-update / task-removed / workspace-event → EventBus（Schema 验证 + 幂等性）
 * - import-complete → 直接更新 task/workspace store（带幂等性检查）
 * - import-error / import-warning → toast
 * - files-ready-batch → 防抖更新 workspace 进度
 *
 * @returns 卸载函数：逐个调用 Tauri unlisten，忽略异常
 */
export async function mountTauriEventProjection(
  options: TauriEventProjectionOptions
): Promise<() => void> {
  const {
    updateWorkspace,
    updateTask,
    showToast,
    debouncedUpdateWorkspace,
    getTasks,
    getWorkspaces,
  } = options;

  const taskUpdateUnlisten = await listen<TaskUpdateEvent>(
    "task-update",
    (event) => {
      logger.debug(
        { payload: event.payload },
        "[TauriEventProjection] Received task-update from Tauri"
      );

      const cleanedPayload = {
        ...event.payload,
        workspace_id: event.payload.workspace_id ?? undefined,
      };

      eventBus.processEvent("task-update", cleanedPayload).catch((error) => {
        logger.error(
          { error },
          "[TauriEventProjection] Failed to process task-update event"
        );
      });
    }
  );

  const taskRemovedUnlisten = await listen<TaskRemovedEvent>(
    "task-removed",
    (event) => {
      logger.debug(
        { payload: event.payload },
        "[TauriEventProjection] Received task-removed from Tauri"
      );

      eventBus.processEvent("task-removed", event.payload).catch((error) => {
        logger.error(
          { error },
          "[TauriEventProjection] Failed to process task-removed event"
        );
      });
    }
  );

  type ImportCompletePayload =
    | string
    | { task_id?: string; workspace_id?: string };
  const importCompleteUnlisten = await listen<ImportCompletePayload>(
    "import-complete",
    (event) => {
      logger.debug(
        { payload: event.payload },
        "[TauriEventProjection] Received import-complete from Tauri"
      );

      const payload = event.payload;
      let taskId: string | null = null;
      let workspaceId: string | null = null;

      if (typeof payload === "string") {
        taskId = payload;
      } else if (payload !== null && typeof payload === "object") {
        taskId = payload.task_id ?? null;
        workspaceId = payload.workspace_id ?? null;
      }

      if (taskId) {
        const currentTask = getTasks().find((t) => t.id === taskId);
        if (!currentTask || currentTask.status !== "COMPLETED") {
          updateTask(taskId, { status: "COMPLETED", progress: 100 });
        }
      }

      if (workspaceId) {
        const currentWs = getWorkspaces().find((w) => w.id === workspaceId);
        if (!currentWs || currentWs.status !== "READY") {
          logger.debug(
            "[TauriEventProjection] import-complete with workspace_id, updating status to READY:",
            workspaceId
          );
          updateWorkspace(workspaceId, { status: "READY" });
        }
      } else if (taskId) {
        const task = getTasks().find((t) => t.id === taskId);
        if (task?.workspaceId) {
          const currentWs = getWorkspaces().find(
            (w) => w.id === task.workspaceId
          );
          if (!currentWs || currentWs.status !== "READY") {
            logger.debug(
              "[TauriEventProjection] import-complete fallback, updating workspace status to READY:",
              task.workspaceId
            );
            updateWorkspace(task.workspaceId, { status: "READY" });
          }
        }
      }
    }
  );

  const importErrorUnlisten = await listen<string>("import-error", (event) => {
    logger.error(
      { payload: event.payload },
      "[TauriEventProjection] Received import-error from Tauri"
    );
    showToast("error", `导入失败: ${event.payload}`);
  });

  const importWarningUnlisten = await listen<string>(
    "import-warning",
    (event) => {
      logger.warn(
        { payload: event.payload },
        "[TauriEventProjection] Received import-warning from Tauri"
      );
      showToast("info", `导入提示: ${event.payload}`);
    }
  );

  interface FilesReadyBatchPayload {
    workspace_id: string;
    ready_count: number;
    total_count: number;
  }
  const filesReadyBatchUnlisten = await listen<FilesReadyBatchPayload>(
    "files-ready-batch",
    (event) => {
      const { workspace_id, ready_count, total_count } = event.payload;
      logger.debug(
        { workspace_id, ready_count, total_count },
        "[TauriEventProjection] Received files-ready-batch from Tauri"
      );

      debouncedUpdateWorkspace(workspace_id, ready_count, total_count);
    }
  );

  const workspaceEventUnlisten = await listen<unknown>(
    "workspace-event",
    (event) => {
      logger.debug(
        { payload: event.payload },
        "[TauriEventProjection] Received workspace-event from Tauri"
      );

      eventBus.processEvent("workspace-event", event.payload).catch((error) => {
        logger.error(
          { error },
          "[TauriEventProjection] Failed to process workspace-event"
        );
      });
    }
  );

  return () => {
    [
      taskUpdateUnlisten,
      taskRemovedUnlisten,
      importCompleteUnlisten,
      importErrorUnlisten,
      importWarningUnlisten,
      filesReadyBatchUnlisten,
      workspaceEventUnlisten,
    ].forEach((unlisten) => {
      try {
        unlisten();
      } catch {
        /* Tauri unlisten 不应抛出，静默处理 */
      }
    });
  };
}
