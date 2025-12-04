import React from 'react';
import { FileText, Plus, RefreshCw, Trash2, Eye, EyeOff, CheckCircle2 } from 'lucide-react';
import { useApp } from '../contexts/AppContext';
import { useWorkspaceOperations } from '../hooks/useWorkspaceOperations';
import { Button, Card } from '../components/ui';
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
  const { workspaces, importFile, importFolder, refreshWorkspace, deleteWorkspace, toggleWatch, switchWorkspace } = useWorkspaceOperations();
  const { state: appState } = useApp();
  
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
         {workspaces.map((ws: Workspace) => (
           <Card key={ws.id} className={cn("h-full flex flex-col hover:border-primary/50 transition-colors group cursor-pointer", appState.activeWorkspaceId === ws.id ? "border-primary ring-1 ring-primary" : "border-border-base")} onClick={() => switchWorkspace(ws.id)}>
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
              </div>
           </Card>
         ))}
      </div>
    </div>
  );
};

export default WorkspacesPage;
