import type { WorkspaceStatusPayload } from "./types";

export interface StatusToast {
  toastType: "success" | "error";
  message: string;
}

/**
 * 把后端 WorkspaceStatus 映射为 toast 文案（纯函数，便于契约级测试）。
 *
 * 返回 null 表示该状态变更不需要打扰用户（Idle / Processing 为中间态）。
 * 注意：后端 delete_workspace 以 Cancelled 广播工作区删除，
 * load_workspace 以 Completed 广播加载完成（见 commands/workspace.rs）。
 */
export function describeWorkspaceStatusChange(
  status: WorkspaceStatusPayload
): StatusToast | null {
  switch (status.status) {
    case "Completed":
      return { toastType: "success", message: "Workspace updated" };
    case "Cancelled":
      return { toastType: "error", message: "Workspace deleted" };
    case "Failed":
      return {
        toastType: "error",
        message: status.error
          ? `Workspace error: ${status.error}`
          : "Workspace error",
      };
    case "Idle":
    case "Processing":
      return null;
  }
}
