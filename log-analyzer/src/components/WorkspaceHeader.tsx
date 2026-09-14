import {
  Cog,
  FileText,
  CheckCircle2,
  RefreshCw,
  Search,
  LayoutGrid,
  ListTodo,
  Layers,
} from "lucide-react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
  const pageMeta = {
    workspaces: { label: t("nav.workspaces"), icon: LayoutGrid },
    search: { label: t("nav.search"), icon: Search },
    keywords: { label: t("nav.keywords"), icon: ListTodo },
    tasks: { label: t("nav.tasks"), icon: Layers },
    settings: { label: t("nav.settings"), icon: Cog },
  }[currentPage] ?? { label: t("nav.workspaces"), icon: FileText };
  const PageIcon = pageMeta.icon;

  return (
    <header className="glass z-40 flex h-12 shrink-0 items-center justify-between border-b border-border-subtle px-3 sm:px-4">
      <div className="flex min-w-0 items-center gap-2 text-sm select-none">
        <PageIcon size={15} className="shrink-0 text-primary-text" />
        <span className="font-semibold text-text-main">{pageMeta.label}</span>
        {activeWorkspace && currentPage !== "settings" && (
          <>
            <span className="text-text-dim">/</span>
            <span className="truncate text-xs text-text-muted">
              {activeWorkspace.name}
            </span>
          </>
        )}
      </div>
      {/* 工作区状态 badge */}
      <div className="ml-3 flex shrink-0 items-center gap-2">
        {activeWorkspace && currentPage !== "settings" && (
          <div className="flex items-center gap-1.5 rounded-full border border-border-subtle bg-bg-card px-2 py-1 text-[10px] font-bold tracking-wide">
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
