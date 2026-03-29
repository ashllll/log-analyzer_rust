import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FileText, Plus, RefreshCw, Trash2, Eye, EyeOff, CheckCircle2, Settings } from 'lucide-react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceSelection } from '../hooks/useWorkspaceSelection';
import { useWorkspaceImport } from '../hooks/useWorkspaceImport';
import { useWorkspaceManagement } from '../hooks/useWorkspaceManagement';
import { useWorkspaceWatch } from '../hooks/useWorkspaceWatch';
import { Button, Card } from '../components/ui';
import { FileFilterSettings } from '../components/modals';
import { cn } from '../utils/classNames';
import type { Workspace } from '../types/common';

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

  const handleDelete = async (id: string) => {
    if (!window.confirm(t('workspaces.delete_confirm'))) return;
    await deleteWorkspace(id);
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
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      {/* 页面标题 */}
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-2xl font-bold text-text-main tracking-tight">{t('workspaces.title')}</h1>
          <p className="text-sm text-text-muted mt-1">{t('workspaces.subtitle')}</p>
        </div>
        <div className="flex gap-2">
          <Button icon={Settings} onClick={() => setIsFilterSettingsOpen(true)} variant="secondary" data-testid="file-filter-settings-button">
            {t('workspaces.filter_settings')}
          </Button>
          <Button icon={FileText} onClick={handleImportFile} variant="secondary" data-testid="import-file-button">{t('workspaces.import_file')}</Button>
          <Button icon={Plus} onClick={handleImportFolder} variant="cta" data-testid="import-folder-button">{t('workspaces.import_folder')}</Button>
        </div>
      </div>

      {/* 工作区网格 */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
         {workspaces.map((ws: Workspace) => (
           <Card
             key={ws.id}
             variant="interactive"
             data-testid={`workspace-card-${ws.id}`}
             className={cn(
               "h-full flex flex-col",
               activeWorkspaceId === ws.id && "ring-2 ring-primary shadow-glow-primary"
             )}
             onClick={() => switchWorkspace(ws.id)}
           >
              {/* 卡片头部 */}
              <div className="px-4 py-3 border-b border-border-subtle bg-bg-elevated/50 font-bold text-sm flex justify-between items-center">
                  <span className="truncate">{ws.name}</span>
                  <div className="flex gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      icon={ws.watching ? EyeOff : Eye}
                      className="h-7 w-7 p-0 text-text-dim hover:text-cta"
                      onClick={(e) => { e.stopPropagation(); handleToggleWatch(ws); }}
                      title={ws.watching ? "Stop watching" : "Start watching"}
                      data-testid={`workspace-watch-${ws.id}`}
                    />
                    <Button
                      variant="ghost"
                      icon={RefreshCw}
                      className="h-7 w-7 p-0 text-text-dim hover:text-primary"
                      onClick={(e) => { e.stopPropagation(); handleRefresh(ws); }}
                      title="Refresh workspace"
                      data-testid={`workspace-refresh-${ws.id}`}
                    />
                    <Button
                      variant="ghost"
                      icon={Trash2}
                      className="h-7 w-7 p-0 text-text-dim hover:text-status-error"
                      onClick={(e) => { e.stopPropagation(); handleDelete(ws.id); }}
                      data-testid={`workspace-delete-${ws.id}`}
                    />
                  </div>
              </div>

              {/* 卡片内容 */}
              <div className="p-4 space-y-4">
                 <code className="text-xs bg-bg-main px-3 py-2 rounded border border-border-subtle block truncate font-mono text-text-muted">
                   {ws.path}
                 </code>
                 <div className="flex items-center gap-2 text-xs font-bold flex-wrap">
                   {ws.status === 'READY' ? (
                     <>
                       <CheckCircle2 size={14} className="text-cta"/>
                       <span className="text-cta">READY</span>
                     </>
                   ) : (
                     <>
                       <RefreshCw size={14} className="text-primary animate-spin"/>
                       <span className="text-primary">PROCESSING</span>
                     </>
                   )}
                   {ws.watching && (
                     <>
                       <Eye size={14} className="text-status-warn ml-2"/>
                       <span className="text-status-warn">WATCHING</span>
                     </>
                   )}
                 </div>
              </div>
           </Card>
         ))}
      </div>

      {/* 文件过滤设置模态框 */}
      <FileFilterSettings
        isOpen={isFilterSettingsOpen}
        onClose={() => setIsFilterSettingsOpen(false)}
        onSaved={() => {
          // 配置保存后可以刷新工作区列表（可选）
        }}
      />
    </div>
  );
};

export default WorkspacesPage;
