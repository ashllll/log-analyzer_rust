import React, { useState, useEffect } from 'react';
import { FileText, Plus, RefreshCw, Trash2, Eye, EyeOff, CheckCircle2, AlertCircle } from 'lucide-react';
import { useAppStore } from '../stores/appStore';
import { useWorkspaceOperations } from '../hooks/useWorkspaceOperations';
import { Button, Card } from '../components/ui';
import { cn } from '../utils/classNames';
import type { Workspace } from '../types/common';
import { MigrationDialog } from '../components/MigrationDialog';
import { useMigration } from '../hooks/useMigration';
import { logger } from '../utils/logger';

/**
 * 工作区管理页面
 * 核心功能:
 * 1. 显示工作区列表
 * 2. 导入文件夹/文件
 * 3. 刷新工作区
 * 4. 删除工作区
 * 5. 切换文件监听状态
 * 6. 检测并提示迁移旧格式工作区
 */
const WorkspacesPage: React.FC = () => {
  const { workspaces, importFile, importFolder, refreshWorkspace, deleteWorkspace, toggleWatch, switchWorkspace } = useWorkspaceOperations();
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const { checkNeedsMigration } = useMigration();
  
  const [migrationDialogOpen, setMigrationDialogOpen] = useState(false);
  const [selectedWorkspaceForMigration, setSelectedWorkspaceForMigration] = useState<Workspace | null>(null);
  const [workspaceMigrationStatus, setWorkspaceMigrationStatus] = useState<Record<string, boolean>>({});

  // Check migration status for all workspaces on mount
  useEffect(() => {
    const checkAllWorkspaces = async () => {
      const statusMap: Record<string, boolean> = {};
      
      for (const ws of workspaces) {
        try {
          const needsMigration = await checkNeedsMigration(ws.id);
          statusMap[ws.id] = needsMigration;
        } catch (err) {
          logger.error('[WorkspacesPage] Failed to check migration status:', err);
          statusMap[ws.id] = false;
        }
      }
      
      setWorkspaceMigrationStatus(statusMap);
    };

    if (workspaces.length > 0) {
      checkAllWorkspaces();
    }
  }, [workspaces, checkNeedsMigration]);
  
  const handleDelete = async (id: string) => {
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

  const handleMigrate = (ws: Workspace) => {
    setSelectedWorkspaceForMigration(ws);
    setMigrationDialogOpen(true);
  };

  const handleMigrationComplete = async () => {
    setMigrationDialogOpen(false);
    
    // Refresh the workspace to update its status
    if (selectedWorkspaceForMigration) {
      await handleRefresh(selectedWorkspaceForMigration);
      
      // Update migration status
      setWorkspaceMigrationStatus(prev => ({
        ...prev,
        [selectedWorkspaceForMigration.id]: false,
      }));
    }
    
    setSelectedWorkspaceForMigration(null);
  };

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-text-main">Workspaces</h1>
        <div className="flex gap-2">
          <Button icon={FileText} onClick={handleImportFile}>Import File</Button>
          <Button icon={Plus} onClick={handleImportFolder}>Import Folder</Button>
        </div>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
         {workspaces.map((ws: Workspace) => {
           const needsMigration = workspaceMigrationStatus[ws.id] || false;
           
           return (
           <Card key={ws.id} className={cn("h-full flex flex-col hover:border-primary/50 transition-colors group cursor-pointer", activeWorkspaceId === ws.id ? "border-primary ring-1 ring-primary" : "border-border-base")} onClick={() => switchWorkspace(ws.id)}>
              <div className="px-4 py-3 border-b border-border-base bg-bg-sidebar/50 font-bold text-sm flex justify-between items-center">
                  {ws.name}
                  <div className="flex gap-1">
                    <Button variant="ghost" icon={ws.watching ? EyeOff : Eye} className="h-6 w-6 p-0 text-text-dim hover:text-emerald-400" onClick={(e) => { e.stopPropagation(); handleToggleWatch(ws); }} title={ws.watching ? "Stop watching" : "Start watching"} />
                    <Button variant="ghost" icon={RefreshCw} className="h-6 w-6 p-0 text-text-dim hover:text-blue-400" onClick={(e) => { e.stopPropagation(); handleRefresh(ws); }} title="Refresh workspace"/>
                    <Button variant="ghost" icon={Trash2} className="h-6 w-6 p-0 text-text-dim hover:text-red-400" onClick={(e) => { e.stopPropagation(); handleDelete(ws.id); }}/>
                  </div>
              </div>
              <div className="p-4 space-y-4">
                 <code className="text-xs bg-bg-main px-2 py-1.5 rounded border border-border-base block truncate font-mono text-text-muted">{ws.path}</code>
                 <div className="flex items-center gap-2 text-xs font-bold">
                   {ws.status === 'READY' ? <><CheckCircle2 size={14} className="text-emerald-500"/> <span className="text-emerald-500">READY</span></> : <><RefreshCw size={14} className="text-blue-500 animate-spin"/> <span className="text-blue-500">PROCESSING</span></>}
                   {ws.watching && <><Eye size={14} className="text-amber-500 ml-2"/> <span className="text-amber-500">WATCHING</span></>}
                 </div>
                 
                 {/* Migration Banner */}
                 {needsMigration && (
                   <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded p-3 space-y-2">
                     <div className="flex items-start gap-2">
                       <AlertCircle size={16} className="text-yellow-600 dark:text-yellow-400 mt-0.5 flex-shrink-0" />
                       <div className="flex-1 min-w-0">
                         <p className="text-xs font-semibold text-yellow-900 dark:text-yellow-100">
                           Migration Available
                         </p>
                         <p className="text-xs text-yellow-800 dark:text-yellow-200 mt-1">
                           This workspace uses the old format. Migrate to improve performance and enable new features.
                         </p>
                       </div>
                     </div>
                     <Button
                       variant="secondary"
                       className="w-full text-xs h-7"
                       onClick={(e) => {
                         e.stopPropagation();
                         handleMigrate(ws);
                       }}
                     >
                       Migrate Now
                     </Button>
                   </div>
                 )}
              </div>
           </Card>
         )})}
      </div>
      
      {/* Migration Dialog */}
      {selectedWorkspaceForMigration && (
        <MigrationDialog
          workspaceId={selectedWorkspaceForMigration.id}
          workspaceName={selectedWorkspaceForMigration.name}
          isOpen={migrationDialogOpen}
          onClose={() => {
            setMigrationDialogOpen(false);
            setSelectedWorkspaceForMigration(null);
          }}
          onMigrationComplete={handleMigrationComplete}
        />
      )}
    </div>
  );
};

export default WorkspacesPage;
