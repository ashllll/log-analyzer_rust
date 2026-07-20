import { Cog, FileText, CheckCircle2, RefreshCw } from "lucide-react";
import { AppearanceControl } from "../theme/AppearanceControl";
import type { Workspace } from "../stores/workspaceStore";

interface WorkspaceHeaderProps {
  currentPage: string;
  activeWorkspace: Workspace | null;
}

/**
 * 顶部工作区状态栏
 * 显示当前页面路径或工作区名称及状态
 */
export const WorkspaceHeader: React.FC<WorkspaceHeaderProps> = ({
  currentPage,
  activeWorkspace,
}) => {
  return (
    <header
      className="apple-material h-[52px] border-b border-border-subtle flex items-center justify-between px-5 shrink-0 z-40"
      style={{ background: "var(--material-toolbar)" }}
    >
      <div className="flex items-center text-sm text-text-muted select-none">
        {currentPage === "settings" ? (
          <span className="font-medium text-text-main flex items-center gap-2">
            <Cog size={14} className="text-primary-text" /> Settings
          </span>
        ) : (
          <>
            <span className="opacity-50">Workspace / </span>
            <span className="font-medium text-text-main ml-2 flex items-center gap-2">
              <FileText size={14} className="text-primary-text" />
              {activeWorkspace ? activeWorkspace.name : "Select Workspace"}
            </span>
          </>
        )}
      </div>
      {/* 工作区状态 badge */}
      <div className="flex items-center gap-3">
        {activeWorkspace && currentPage !== "settings" && (
          <div className="flex items-center gap-1.5 text-xs font-semibold">
            {activeWorkspace.status === "READY" ? (
              <>
                <CheckCircle2 size={12} className="text-cta" />
                <span className="text-cta">READY</span>
              </>
            ) : (
              <>
                <RefreshCw
                  size={12}
                  className="text-primary-text animate-spin"
                />
                <span className="text-primary-text">PROCESSING</span>
              </>
            )}
          </div>
        )}
        <AppearanceControl />
      </div>
    </header>
  );
};
