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

  return (
    <div className="mx-auto h-full max-w-6xl overflow-auto px-8 py-7">
      {/* 页面标题 */}
      <div className="mb-7 flex items-center justify-between">
        <div>
          <h1 className="text-[28px] font-semibold leading-tight text-text-main tracking-[-0.02em]">
            {t("workspaces.title")}
          </h1>
          <p className="text-sm text-text-muted mt-1">
            {t("workspaces.subtitle")}
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            icon={Settings}
            onClick={() => setIsFilterSettingsOpen(true)}
            variant="secondary"
            data-testid="file-filter-settings-button"
          >
            {t("workspaces.filter_settings")}
          </Button>
          <Button
            icon={FileText}
            onClick={handleImportFile}
            variant="secondary"
            data-testid="import-file-button"
          >
            {t("workspaces.import_file")}
          </Button>
          <Button
            icon={Plus}
            onClick={handleImportFolder}
            variant="cta"
            data-testid="import-folder-button"
          >
            {t("workspaces.import_folder")}
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
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {workspaces.map((ws: Workspace) => (
            <div key={ws.id}>
              <Card
                variant="interactive"
                data-testid={`workspace-card-${ws.id}`}
                className={cn(
                  "h-full flex flex-col",
                  activeWorkspaceId === ws.id &&
                    "border-primary ring-1 ring-primary/30"
                )}
                onClick={() => switchWorkspace(ws.id)}
              >
                {/* 卡片头部 */}
                <div className="flex items-center justify-between border-b border-border-subtle bg-bg-elevated/40 px-4 py-3 text-sm font-semibold">
                  <span className="truncate">{ws.name}</span>
                  <div className="flex gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      icon={ws.watching ? EyeOff : Eye}
                      className="h-10 w-10 p-0 text-text-dim hover:text-cta-text"
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
                      className="h-10 w-10 p-0 text-text-dim hover:text-primary-text"
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
                        className="ui-pressable grid h-10 w-10 cursor-pointer list-none place-items-center rounded-[10px] text-text-dim hover:bg-bg-hover hover:text-text-main"
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
                </div>

                {/* 卡片内容 */}
                <div className="p-4 space-y-4">
                  <code className="block truncate rounded-[8px] border border-border-subtle bg-bg-main px-3 py-2 font-mono text-xs text-text-muted">
                    {ws.path}
                  </code>
                  <div className="flex items-center gap-2 text-xs font-bold flex-wrap">
                    {ws.status === "READY" ? (
                      <>
                        <CheckCircle2 size={14} className="text-cta" />
                        <span className="text-cta">READY</span>
                      </>
                    ) : (
                      <>
                        <RefreshCw
                          size={14}
                          className="text-primary-text animate-spin"
                        />
                        <span className="text-primary-text">PROCESSING</span>
                      </>
                    )}
                    {ws.watching && (
                      <>
                        <Eye size={14} className="text-status-warn ml-2" />
                        <span className="text-status-warn">WATCHING</span>
                      </>
                    )}
                  </div>
                </div>
              </Card>
            </div>
          ))}
        </div>
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
