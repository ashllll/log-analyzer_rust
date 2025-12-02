import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { 
  Search, LayoutGrid, ListTodo, Settings, Layers, 
  RefreshCw, Zap, Loader2, FileText
} from "lucide-react";

// 导入全局Context和Hooks
import { AppProvider, useApp } from './contexts/AppContext';
import { useWorkspaceOperations } from './hooks/useWorkspaceOperations';
import { useKeywordManager } from './hooks/useKeywordManager';

// 导入类型定义
import type { PerformanceStats } from './types/common';

// 导入UI组件
import { Button, Card, NavItem, ToastContainer } from './components/ui';

// 导入页面组件
import { SearchPage, KeywordsPage, WorkspacesPage, TasksPage } from './pages';

// Page Components: Moved to pages/

// --- Performance Page ---
const PerformancePage = ({ addToast }: { addToast: any }) => {
  const [stats, setStats] = useState<PerformanceStats | null>(null);
  const [loading, setLoading] = useState(false);

  const loadStats = async () => {
    setLoading(true);
    try {
      const metrics = await invoke<any>('get_performance_metrics');
      setStats({
        memoryUsed: metrics.memory_used_mb,
        pathMapSize: metrics.path_map_size,
        cacheSize: metrics.cache_size,
        lastSearchDuration: metrics.last_search_duration_ms,
        cacheHitRate: metrics.cache_hit_rate,
        indexedFilesCount: metrics.indexed_files_count,
        indexFileSizeMb: metrics.index_file_size_mb
      });
    } catch (e) {
      addToast('error', `Failed to load stats: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 5000); // 每5秒刷新
    return () => clearInterval(interval);
  }, []);

  if (!stats) {
    return <div className="p-10 text-center text-text-dim">Loading performance stats...</div>;
  }

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-text-main">Performance Monitor</h1>
        <Button icon={RefreshCw} onClick={loadStats} disabled={loading}>
          {loading ? 'Refreshing...' : 'Refresh'}
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {/* 内存使用 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Memory Usage</div>
          <div className="text-3xl font-bold text-primary">
            {stats.memoryUsed > 0 ? `${stats.memoryUsed.toFixed(1)} MB` : 'N/A'}
          </div>
          <div className="text-xs text-text-muted mt-1">进程内存占用</div>
        </Card>

        {/* 索引文件数 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Indexed Files</div>
          <div className="text-3xl font-bold text-emerald-400">
            {stats.indexedFilesCount.toLocaleString()}
          </div>
          <div className="text-xs text-text-muted mt-1">已索引文件数量</div>
        </Card>

        {/* 缓存大小 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Cache Size</div>
          <div className="text-3xl font-bold text-blue-400">
            {stats.cacheSize}
          </div>
          <div className="text-xs text-text-muted mt-1">搜索缓存条目数</div>
        </Card>

        {/* 搜索耗时 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Last Search</div>
          <div className="text-3xl font-bold text-amber-400">
            {stats.lastSearchDuration > 0 ? `${stats.lastSearchDuration} ms` : 'N/A'}
          </div>
          <div className="text-xs text-text-muted mt-1">最近搜索耗时</div>
        </Card>

        {/* 缓存命中率 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Cache Hit Rate</div>
          <div className="text-3xl font-bold text-purple-400">
            {stats.cacheHitRate.toFixed(1)}%
          </div>
          <div className="text-xs text-text-muted mt-1">缓存命中率</div>
        </Card>

        {/* 索引文件大小 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Index Size</div>
          <div className="text-3xl font-bold text-red-400">
            {stats.indexFileSizeMb.toFixed(2)} MB
          </div>
          <div className="text-xs text-text-muted mt-1">索引文件磁盘占用</div>
        </Card>
      </div>
    </div>
  );
};

// --- Main App Component (Internal) ---
function AppContent() {
  const { state: appState, setPage, addToast, removeToast } = useApp();
  const { keywordGroups } = useKeywordManager();
  const { workspaces } = useWorkspaceOperations();
  
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [importStatus] = useState("");  // 保留以兼容旧代码，但实际不再使用
  
  const activeWorkspace = workspaces.find(w => w.id === appState.activeWorkspaceId) || null;

  return (
    <div className="flex h-screen bg-bg-main text-text-main font-sans selection:bg-primary/30">
      <div className="w-[240px] bg-bg-sidebar border-r border-border-base flex flex-col shrink-0 z-50">
        <div className="h-14 flex items-center px-5 border-b border-border-base mb-2 select-none"><div className="h-8 w-8 bg-primary rounded-lg flex items-center justify-center text-white mr-3 shadow-lg shadow-primary/20"><Zap size={18} fill="currentColor" /></div><span className="font-bold text-lg tracking-tight">LogAnalyzer</span></div>
        <div className="flex-1 px-3 py-4 space-y-1">
            <NavItem icon={LayoutGrid} label="Workspaces" active={appState.page === 'workspaces'} onClick={() => setPage('workspaces')} />
            <NavItem icon={Search} label="Search Logs" active={appState.page === 'search'} onClick={() => setPage('search')} />
            <NavItem icon={ListTodo} label="Keywords" active={appState.page === 'keywords'} onClick={() => setPage('keywords')} />
            <NavItem icon={Layers} label="Tasks" active={appState.page === 'tasks'} onClick={() => setPage('tasks')} />
        </div>
        {importStatus && <div className="p-3 m-3 bg-bg-card border border-primary/20 rounded text-xs text-primary animate-pulse"><div className="font-bold mb-1 flex items-center gap-2"><Loader2 size={12} className="animate-spin"/> Processing</div><div className="truncate opacity-80">{importStatus}</div></div>}
        <div className="p-3 border-t border-border-base">
          <NavItem icon={Settings} label="Settings" active={appState.page === 'settings'} onClick={() => setPage('settings')} />
        </div>
      </div>
      <div className="flex-1 flex flex-col min-w-0 bg-bg-main">
        <div className="h-14 border-b border-border-base bg-bg-main flex items-center justify-between px-6 shrink-0 z-40"><div className="flex items-center text-sm text-text-muted select-none"><span className="opacity-50">Workspace / </span><span className="font-medium text-text-main ml-2 flex items-center gap-2"><FileText size={14} className="text-primary"/> {activeWorkspace ? activeWorkspace.name : "Select Workspace"}</span></div></div>
        <div className="flex-1 overflow-hidden relative">
           {appState.page === 'search' && <SearchPage keywordGroups={keywordGroups} addToast={addToast} searchInputRef={searchInputRef} activeWorkspace={activeWorkspace} />}
           {appState.page === 'keywords' && <KeywordsPage />}
           {appState.page === 'workspaces' && <WorkspacesPage />}
           {appState.page === 'tasks' && <TasksPage />}
           {appState.page === 'settings' && <PerformancePage addToast={addToast} />}
        </div>
      </div>
      <ToastContainer toasts={appState.toasts} removeToast={removeToast} />
    </div>
  );
}

// --- Main App (Wrapped with Provider) ---
export default function App() {
  return (
    <AppProvider>
      <AppContent />
    </AppProvider>
  );
}