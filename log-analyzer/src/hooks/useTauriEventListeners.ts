import { useEffect, useRef } from "react";
import { mountTauriEventProjection } from "../events/tauriEventProjection";
import { useWorkspaceStore } from "../stores/workspaceStore";
import { useTaskStore } from "../stores/taskStore";
import { useToast } from "./useToast";
import { logger } from "../utils/logger";

/**
 * Tauri 原生事件监听 Hook
 *
 * 职责分工：
 * - 需要 Schema 验证 / 幂等性检查的事件 → 桥接到 TaskEventBus
 *   - task-update → eventBus.processEvent('task-update')
 *   - task-removed → eventBus.processEvent('task-removed')
 *   - workspace-event → eventBus.processEvent('workspace-event')
 * - 简单通知事件 → 直接处理（避免不必要的验证开销）
 *   - import-complete → 直接更新 task/workspace 状态
 *   - import-error → toast 错误提示
 *   - validation-report → 导入完整性校验问题 toast
 *
 * 使用 tauriCleanupRef 确保异步注册完成后同步清理。
 */
export const useTauriEventListeners = () => {
  const updateWorkspace = useWorkspaceStore((state) => state.updateWorkspace);
  const updateTask = useTaskStore((state) => state.updateTask);
  const { showToast } = useToast();

  // 使用 ref 存储 Tauri 清理函数，确保在组件卸载时同步调用
  const tauriCleanupRef = useRef<(() => void) | null>(null);
  useEffect(() => {
    let isMounted = true;

    // 异步设置 Tauri 监听器
    mountTauriEventProjection({
      updateWorkspace,
      updateTask,
      showToast,
      getTasks: () => useTaskStore.getState().tasks,
      getWorkspaces: () => useWorkspaceStore.getState().workspaces,
    })
      .then((cleanup) => {
        if (isMounted) {
          tauriCleanupRef.current = cleanup;
        } else {
          // 组件已卸载（如 React StrictMode 双重挂载、快速路由切换），立即清理
          cleanup();
        }
      })
      .catch((error: unknown) => {
        logger.error(
          { error },
          "[TauriEventListeners] Tauri 事件监听器初始化失败，部分实时更新不可用"
        );
      });

    return () => {
      isMounted = false;

      // 清理Tauri监听（同步调用，避免 Promise 时序问题）
      if (tauriCleanupRef.current) {
        tauriCleanupRef.current();
        tauriCleanupRef.current = null;
      }
    };
  }, [updateWorkspace, updateTask, showToast]);
};
