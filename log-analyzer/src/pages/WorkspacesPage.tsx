import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FileText,
  Plus,
  RefreshCw,
  Trash2,
  Eye,
  EyeOff,
  CheckCircle2,
  Settings,
  FolderOpen,
  MoreHorizontal,
} from "lucide-react";
import { useAppStore } from "../stores/appStore";
import { useWorkspaceSelection } from "../hooks/useWorkspaceSelection";
import { useWorkspaceImport } from "../hooks/useWorkspaceImport";
import { useWorkspaceManagement } from "../hooks/useWorkspaceManagement";
import { useWorkspaceWatch } from "../hooks/useWorkspaceWatch";
import { Button, Card, DialogSurface, EmptyState } from "../components/ui";
import { FileFilterSettings } from "../components/modals";
import { cn } from "../utils/classNames";
import type { Workspace } from "../types/common";

/**
 * 工作区管理页面
 * 核心功能:
 * 1. 显示工作区列表
 * 2. 导入文件夹/文件
 * 3. 刷新工作区
 * 4. 删除工作区
 * 5. 切换文件监听状态
 */
const WorkspacesPage: React.FC = () => {
  const { t } = useTranslation();
  const { workspaces, switchWorkspace } = useWorkspaceSelection();
  const { importFile, importFolder } = useWorkspaceImport();
  const { deleteWorkspace, refreshWorkspace } = useWorkspaceManagement();
  const { toggleWatch } = useWorkspaceWatch();
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const [isFilterSettingsOpen, setIsFilterSettingsOpen] = useState(false);
  const [workspacePendingDelete, setWorkspacePendingDelete] =
    useState<Workspace | null>(null);
  const deleteDialogCloseRef = useRef<(() => void) | null>(null);

  const handleDelete = async () => {
    if (!workspacePendingDelete) return;
    await deleteWorkspace(workspacePendingDelete.id);
    deleteDialogCloseRef.current?.();
  };

  const handleToggleWatch = async (ws: Workspace) => {
    await toggleWatch(ws);
  };

  const handleRefresh = async (ws: Workspace) => {
    await refreshWorkspace(ws);
  };

  const handleImportFile = async () => {
    await importFile();
  };

  const handleImportFolder = async () => {
    await importFolder();
  };

  const workspaceSummary = [
    {
      label: t("workspaces.metric_total", "Total"),
      value: workspaces.length,
      tone: "text-text-main",
    },
    {
      label: t("workspaces.metric_ready", "Ready"),
      value: workspaces.filter((workspace) => workspace.status === "READY")
        .length,
      tone: "text-cta",
    },
    {
      label: t("workspaces.metric_watching", "Watching"),
      value: workspaces.filter((workspace) => workspace.watching).length,
      tone: "text-status-warn",
    },
    {
      label: t("workspaces.metric_processing", "Processing"),
      value: workspaces.filter((workspace) => workspace.status !== "READY")
        .length,
      tone: "text-primary-text",
    },
  ];

  return (
    <div className="mx-auto h-full max-w-6xl overflow-auto px-3 py-3 sm:px-5 sm:py-4 lg:px-6">
      {/* 页面标题 */}
      <div className="mb-4 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="truncate text-[22px] font-semibold leading-tight tracking-[-0.02em] text-text-main">
            {t("workspaces.title")}
          </h1>
          <p className="mt-0.5 hidden truncate text-xs text-text-muted sm:block">
            {t("workspaces.subtitle")}
          </p>
        </div>
        <div className="flex shrink-0 gap-1.5">
          <Button
            icon={Settings}
            onClick={() => setIsFilterSettingsOpen(true)}
            variant="secondary"
            className="w-9 px-0 lg:w-auto lg:px-3"
            aria-label={t("workspaces.filter_settings")}
            title={t("workspaces.filter_settings")}
            data-testid="file-filter-settings-button"
          >
            <span className="hidden lg:inline">
              {t("workspaces.filter_settings")}
            </span>
          </Button>
          <Button
            icon={FileText}
            onClick={handleImportFile}
            variant="secondary"
            className="w-9 px-0 lg:w-auto lg:px-3"
            aria-label={t("workspaces.import_file")}
            title={t("workspaces.import_file")}
            data-testid="import-file-button"
          >
            <span className="hidden lg:inline">
              {t("workspaces.import_file")}
            </span>
          </Button>
          <Button
            icon={Plus}
            onClick={handleImportFolder}
            variant="cta"
            className="w-9 px-0 sm:w-auto sm:px-3"
            aria-label={t("workspaces.import_folder")}
            title={t("workspaces.import_folder")}
            data-testid="import-folder-button"
          >
            <span className="hidden sm:inline">
              {t("workspaces.import_folder")}
            </span>
          </Button>
        </div>
      </div>

      {/* 工作区网格或空状态 */}
      {workspaces.length === 0 ? (
        <EmptyState
          icon={FolderOpen}
          title={t("workspaces.empty_title", "还没有工作区")}
          description={t(
            "workspaces.empty_description",
            "导入一个文件夹或日志文件，开始分析"
          )}
          action={{
            label: t("workspaces.import_folder"),
            onClick: handleImportFolder,
            icon: Plus,
            variant: "cta",
          }}
        />
      ) : (
        <>
          <section
            className="mb-3 grid grid-cols-2 gap-px overflow-hidden rounded-[12px] border border-border-subtle bg-border-subtle sm:grid-cols-4"
            aria-label={t("workspaces.overview", "Workspace overview")}
          >
            {workspaceSummary.map((metric) => (
              <div key={metric.label} className="bg-bg-card px-3 py-2.5">
                <div
                  className={cn(
                    "text-lg font-semibold tabular-nums",
                    metric.tone
                  )}
                >
                  {metric.value}
                </div>
                <div className="text-[10px] font-medium uppercase tracking-[0.08em] text-text-dim">
                  {metric.label}
                </div>
              </div>
            ))}
          </section>

          <div className="space-y-2">
            {workspaces.map((ws: Workspace) => (
              <div key={ws.id}>
                <Card
                  variant="interactive"
                  padding="none"
                  data-testid={`workspace-card-${ws.id}`}
                  className={cn(
                    "group flex min-h-[64px] items-center gap-3 overflow-visible px-3 py-2.5",
                    activeWorkspaceId === ws.id &&
                      "border-primary ring-1 ring-primary/30"
                  )}
                  onClick={() => switchWorkspace(ws.id)}
                >
                  <div
                    className={cn(
                      "h-2.5 w-2.5 shrink-0 rounded-full",
                      ws.status === "READY" ? "bg-cta" : "bg-primary"
                    )}
                    aria-hidden="true"
                  />

                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-semibold text-text-main">
                        {ws.name}
                      </span>
                      {ws.watching && (
                        <span className="hidden items-center gap-1 rounded-full bg-status-warn/10 px-1.5 py-0.5 text-[9px] font-bold tracking-wide text-status-warn sm:flex">
                          <Eye size={10} aria-hidden="true" /> WATCHING
                        </span>
                      )}
                    </div>
                    <code className="block truncate font-mono text-[11px] leading-4 text-text-dim">
                      {ws.path}
                    </code>
                  </div>

                  <div className="hidden w-24 shrink-0 items-center gap-1.5 text-[10px] font-bold sm:flex">
                    {ws.status === "READY" ? (
                      <>
                        <CheckCircle2 size={13} className="text-cta" />
                        <span className="text-cta">READY</span>
                      </>
                    ) : (
                      <>
                        <RefreshCw
                          size={13}
                          className="animate-spin text-primary-text"
                        />
                        <span className="text-primary-text">PROCESSING</span>
                      </>
                    )}
                  </div>

                  <div className="flex shrink-0 gap-0.5">
                    <Button
                      variant="ghost"
                      icon={ws.watching ? EyeOff : Eye}
                      className="h-8 w-8 p-0 text-text-dim hover:text-cta-text"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleToggleWatch(ws);
                      }}
                      title={ws.watching ? "Stop watching" : "Start watching"}
                      data-testid={`workspace-watch-${ws.id}`}
                    />
                    <Button
                      variant="ghost"
                      icon={RefreshCw}
                      className="hidden h-8 w-8 p-0 text-text-dim hover:text-primary-text sm:flex"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleRefresh(ws);
                      }}
                      title="Refresh workspace"
                      data-testid={`workspace-refresh-${ws.id}`}
                    />
                    <details
                      className="relative"
                      onClick={(event) => event.stopPropagation()}
                    >
                      <summary
                        className="ui-pressable grid h-8 w-8 cursor-pointer list-none place-items-center rounded-[9px] text-text-dim hover:bg-bg-hover hover:text-text-main"
                        aria-label={`More actions for ${ws.name}`}
                      >
                        <MoreHorizontal size={16} aria-hidden="true" />
                      </summary>
                      <div className="popover-surface absolute right-0 top-full z-30 mt-1 w-40 p-1.5">
                        <Button
                          variant="ghost"
                          icon={Trash2}
                          className="h-9 w-full justify-start text-status-error"
                          onClick={() => setWorkspacePendingDelete(ws)}
                          data-testid={`workspace-delete-${ws.id}`}
                        >
                          Delete
                        </Button>
                      </div>
                    </details>
                  </div>
                </Card>
              </div>
            ))}
          </div>
        </>
      )}

      {/* 文件过滤设置模态框 */}
      <FileFilterSettings
        isOpen={isFilterSettingsOpen}
        onClose={() => setIsFilterSettingsOpen(false)}
        onSaved={() => {
          // 配置保存后可以刷新工作区列表（可选）
        }}
      />
      <DialogSurface
        open={workspacePendingDelete !== null}
        onClose={() => setWorkspacePendingDelete(null)}
        requestCloseRef={deleteDialogCloseRef}
        ariaLabelledBy="delete-workspace-title"
        className="w-[min(26rem,calc(100vw-2rem))]"
      >
        <div className="space-y-2 px-6 py-5">
          <h2
            id="delete-workspace-title"
            className="text-lg font-semibold text-text-main"
          >
            {t("workspaces.delete_workspace", "Delete workspace?")}
          </h2>
          <p className="text-sm leading-6 text-text-muted">
            {t("workspaces.delete_confirm")}
          </p>
        </div>
        <div className="flex justify-end gap-2 border-t border-border-base bg-bg-sidebar px-6 py-4">
          <Button variant="secondary" data-dialog-close>
            {t("common.cancel", "Cancel")}
          </Button>
          <Button variant="danger" onClick={handleDelete}>
            {t("common.delete", "Delete")}
          </Button>
        </div>
      </DialogSurface>
    </div>
  );
};

export default WorkspacesPage;
