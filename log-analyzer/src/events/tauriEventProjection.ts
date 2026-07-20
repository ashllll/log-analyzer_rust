import { listen } from "@tauri-apps/api/event";
import { eventBus } from "./EventBus";
import { logger } from "../utils/logger";
import type { TaskUpdateEvent, TaskRemovedEvent } from "./types";
import { ImportCompleteEventSchema } from "./types";
import type { Task, Workspace } from "../stores/types";

// ── import-complete 一次性事件去重 ──
// 使用 Set<string> 跟踪已完成导入的 task ID。与 EventBus 中
// 面向流式事件的版本号去重不同，一次性事件无需 LRU 淘汰——
// Set 的增长受限于导入操作的总次数。
const completedImports = new Set<string>();

export interface TauriEventProjectionOptions {
  updateWorkspace: (id: string, updates: Partial<Workspace>) => void;
  updateTask: (id: string, updates: Partial<Task>) => void;
  showToast: (type: "error" | "info", message: string) => void;
  getTasks: () => Task[];
  getWorkspaces: () => Workspace[];
}

/**
 * 挂载 Tauri 事件投影：把后端原始事件订阅转换为前端 store / toast / EventBus 动作。
 *
 * 事件契约（与 src-tauri EventPublisher 一一对应，勿引入无后端发送者的监听）：
 * - task-update / task-removed / workspace-event → EventBus（Schema 验证 + 幂等性）
 * - import-complete → 直接更新 task/workspace store（带幂等性检查）
 * - import-error → toast
 * - validation-report → 导入后完整性校验发现问题时 toast 警告
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
    getTasks,
    // getWorkspaces is unused after dedup moved to Set-based tracker
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

  const importCompleteUnlisten = await listen<unknown>(
    "import-complete",
    (event) => {
      logger.debug(
        { payload: event.payload },
        "[TauriEventProjection] Received import-complete from Tauri"
      );

      // Step 1: Schema 验证（支持 string 和 object 两种 payload 格式）
      let taskId: string | null = null;
      let workspaceId: string | null = null;

      if (typeof event.payload === "string") {
        // 旧格式: 纯 task_id 字符串
        taskId = event.payload;
      } else {
        const parsed = ImportCompleteEventSchema.safeParse(event.payload);
        if (parsed.success) {
          taskId = parsed.data.task_id;
          // workspace_id 可能在 payload 中（未来扩展）
          workspaceId =
            ((event.payload as Record<string, unknown>).workspace_id as
              | string
              | undefined) ?? null;
        } else {
          logger.warn(
            { errors: parsed.error.issues, payload: event.payload },
            "[TauriEventProjection] import-complete schema validation failed"
          );
          return; // 丢弃格式异常的事件
        }
      }

      if (!taskId) {
        logger.warn(
          "[TauriEventProjection] import-complete without valid task_id, skipping"
        );
        return;
      }

      // Step 2: 一次性事件去重（Set 比 store 查找更可靠，不依赖 task GC 状态）
      if (completedImports.has(taskId)) {
        logger.debug(
          { taskId },
          "[TauriEventProjection] import-complete already processed (dedup)"
        );
        return;
      }
      completedImports.add(taskId);

      // Step 3: 状态更新
      updateTask(taskId, { status: "COMPLETED", progress: 100 });

      if (workspaceId) {
        updateWorkspace(workspaceId, { status: "READY" });
      } else {
        const task = getTasks().find((t) => t.id === taskId);
        if (task?.workspaceId) {
          updateWorkspace(task.workspaceId, { status: "READY" });
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

  // validation-report：后端在导入完成后执行完整性校验，仅在发现问题时发送。
  // Payload 形如 { workspace_id, report: ValidationReport }（见 la-storage integrity）。
  interface ValidationReportPayload {
    workspace_id?: string;
    report?: {
      total_files?: number;
      valid_files?: number;
      invalid_files?: unknown[];
      missing_objects?: unknown[];
      corrupted_objects?: unknown[];
    };
  }
  const validationReportUnlisten = await listen<ValidationReportPayload>(
    "validation-report",
    (event) => {
      const { workspace_id, report } = event.payload;
      const issueCount =
        (report?.invalid_files?.length ?? 0) +
        (report?.missing_objects?.length ?? 0) +
        (report?.corrupted_objects?.length ?? 0);
      logger.warn(
        { payload: event.payload },
        "[TauriEventProjection] Import integrity verification found issues"
      );
      showToast(
        "error",
        `导入完整性校验发现 ${issueCount} 个问题（工作区 ${workspace_id ?? "未知"}，共 ${report?.total_files ?? "?"} 个文件），详情请查看日志`
      );
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
      validationReportUnlisten,
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
