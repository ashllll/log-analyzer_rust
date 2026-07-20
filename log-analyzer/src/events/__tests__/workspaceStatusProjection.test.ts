import { describeWorkspaceStatusChange } from "../workspaceStatusProjection";

describe("describeWorkspaceStatusChange", () => {
  it("maps Completed (load_workspace broadcast) to a success toast", () => {
    expect(
      describeWorkspaceStatusChange({ status: "Completed", duration: 0 })
    ).toEqual({ toastType: "success", message: "Workspace updated" });
  });

  it("maps Cancelled (delete_workspace broadcast) to an error toast", () => {
    expect(
      describeWorkspaceStatusChange({
        status: "Cancelled",
        cancelled_at: 1700000002,
      })
    ).toEqual({ toastType: "error", message: "Workspace deleted" });
  });

  it("maps Failed to an error toast carrying the backend error message", () => {
    expect(
      describeWorkspaceStatusChange({
        status: "Failed",
        error: "disk full",
        failed_at: 1700000001,
      })
    ).toEqual({ toastType: "error", message: "Workspace error: disk full" });
  });

  it("maps Failed without an error message to a generic error toast", () => {
    expect(describeWorkspaceStatusChange({ status: "Failed" })).toEqual({
      toastType: "error",
      message: "Workspace error",
    });
  });

  it("maps intermediate states to no toast", () => {
    expect(describeWorkspaceStatusChange({ status: "Idle" })).toBeNull();
    expect(
      describeWorkspaceStatusChange({
        status: "Processing",
        started_at: 1700000000,
      })
    ).toBeNull();
  });
});
