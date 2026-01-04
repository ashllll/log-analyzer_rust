import { useState, useRef, useEffect } from "react";
import {
  Search, LayoutGrid, ListTodo, Settings, Layers,
  Zap, Loader2, FileText
} from "lucide-react";
import { ErrorBoundary } from 'react-error-boundary';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Toaster } from 'react-hot-toast';

// 初始化i18n
import './i18n';

// 导入 React Query
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/queryClient';

// 导入全局Store和Hooks
import { AppStoreProvider } from './stores/AppStoreProvider';
import { useAppStore } from './stores/appStore';
import { useWorkspaceOperations } from './hooks/useWorkspaceOperations';
import { useKeywordManager } from './hooks/useKeywordManager';

// 导入UI组件
import { NavItem } from './components/ui';
import { PageErrorFallback, CompactErrorFallback } from './components/ErrorFallback';

// 导入页面组件
import { SearchPage, KeywordsPage, WorkspacesPage, TasksPage, SettingsPage } from './pages';

// 导入 IPC 预热机制
import { initializeIPCConnection } from './utils/ipcWarmup';

// --- Main App Component (Internal) ---
function AppContent() {
  const page = useAppStore((state) => state.page);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const setPage = useAppStore((state) => state.setPage);
  const addToast = useAppStore((state) => state.addToast);
  const isInitialized = useAppStore((state) => state.isInitialized);
  const initializationError = useAppStore((state) => state.initializationError);

  const { keywordGroups } = useKeywordManager();
  const { workspaces, refreshWorkspaces } = useWorkspaceOperations();

  const searchInputRef = useRef<HTMLInputElement>(null);
  const [importStatus] = useState("");  // 保留以兼容旧代码，但实际不再使用

  const activeWorkspace = workspaces.find(w => w.id === activeWorkspaceId) || null;

  // IPC 连接预热（应用启动时）
  useEffect(() => {
    initializeIPCConnection();
  }, []);

  // 初始化状态同步并监听工作区事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupStateSync = async () => {
      try {
        // 初始化状态同步
        await invoke('init_state_sync');
        console.log('[StateSync] Initialized successfully');

        // 标记应用已初始化
        addToast('success', 'Application initialized successfully');

        // 监听工作区事件
        unlisten = await listen('workspace-event', (event: any) => {
          console.log('[StateSync] Received workspace event:', event.payload);
          
          const { status } = event.payload;

          // 根据事件类型更新UI
          if (event.payload.type === 'StatusChanged') {
            // 显示Toast通知
            const toastType = status?.status === 'Cancelled' ? 'error' : 'success';
            const toastMessage = status?.status === 'Cancelled' 
              ? 'Workspace deleted' 
              : status?.status === 'Completed' 
                ? 'Workspace updated' 
                : 'Workspace status changed';
             
            addToast(toastType, toastMessage);

            // 刷新工作区列表
            refreshWorkspaces();
          }
        });

        console.log('[StateSync] Event listener registered');
      } catch (error) {
        console.error('[StateSync] Failed to initialize:', error);
        addToast('error', 'Failed to initialize state sync');
      }
    };

    setupStateSync();

    // 清理函数
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [addToast, refreshWorkspaces]);

  // 显示初始化加载状态
  if (!isInitialized) {
    return (
      <div className="flex h-screen items-center justify-center bg-bg-main text-text-main">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
          {initializationError ? (
            <div>
              <p className="text-red-500 mb-2">Initialization failed</p>
              <p className="text-sm text-text-muted">{initializationError}</p>
            </div>
          ) : (
            <p>Loading application...</p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-bg-main text-text-main font-sans selection:bg-primary/30">
      <div className="w-[240px] bg-bg-sidebar border-r border-border-base flex flex-col shrink-0 z-50">
        <div className="h-14 flex items-center px-5 border-b border-border-base mb-2 select-none"><div className="h-8 w-8 bg-primary rounded-lg flex items-center justify-center text-white mr-3 shadow-lg shadow-primary/20"><Zap size={18} fill="currentColor" /></div><span className="font-bold text-lg tracking-tight">LogAnalyzer</span></div>
        <div className="flex-1 px-3 py-4 space-y-1">
            <NavItem icon={LayoutGrid} label="Workspaces" active={page === 'workspaces'} onClick={() => setPage('workspaces')} />
            <NavItem icon={Search} label="Search Logs" active={page === 'search'} onClick={() => setPage('search')} />
            <NavItem icon={ListTodo} label="Keywords" active={page === 'keywords'} onClick={() => setPage('keywords')} />
            <NavItem icon={Layers} label="Tasks" active={page === 'tasks'} onClick={() => setPage('tasks')} />
        </div>
        {importStatus && <div className="p-3 m-3 bg-bg-card border border-primary/20 rounded text-xs text-primary animate-pulse"><div className="font-bold mb-1 flex items-center gap-2"><Loader2 size={12} className="animate-spin"/> Processing</div><div className="truncate opacity-80">{importStatus}</div></div>}
        <div className="p-3 border-t border-border-base">
          <NavItem icon={Settings} label="Settings" active={page === 'settings'} onClick={() => setPage('settings')} />
        </div>
      </div>
      <div className="flex-1 flex flex-col min-w-0 bg-bg-main">
        <div className="h-14 border-b border-border-base bg-bg-main flex items-center justify-between px-6 shrink-0 z-40"><div className="flex items-center text-sm text-text-muted select-none"><span className="opacity-50">Workspace / </span><span className="font-medium text-text-main ml-2 flex items-center gap-2"><FileText size={14} className="text-primary"/> {activeWorkspace ? activeWorkspace.name : "Select Workspace"}</span></div></div>
        <div className="flex-1 overflow-hidden relative">
           <ErrorBoundary FallbackComponent={CompactErrorFallback} onReset={() => setPage('workspaces')}>
             {page === 'search' && <SearchPage keywordGroups={keywordGroups} addToast={addToast} searchInputRef={searchInputRef} activeWorkspace={activeWorkspace} />}
           </ErrorBoundary>
           <ErrorBoundary FallbackComponent={CompactErrorFallback} onReset={() => setPage('keywords')}>
             {page === 'keywords' && <KeywordsPage />}
           </ErrorBoundary>
           <ErrorBoundary FallbackComponent={CompactErrorFallback} onReset={() => setPage('workspaces')}>
             {page === 'workspaces' && <WorkspacesPage />}
           </ErrorBoundary>
           <ErrorBoundary FallbackComponent={CompactErrorFallback} onReset={() => setPage('tasks')}>
             {page === 'tasks' && <TasksPage />}
           </ErrorBoundary>
           <ErrorBoundary FallbackComponent={CompactErrorFallback} onReset={() => setPage('settings')}>
             {page === 'settings' && <SettingsPage />}
           </ErrorBoundary>
        </div>
      </div>
      <Toaster
        position="bottom-right"
        toastOptions={{
          duration: 3000,
          style: {
            background: 'rgb(30, 41, 59)',
            color: 'rgb(226, 232, 240)',
            border: '1px solid rgba(148, 163, 184, 0.2)',
            borderRadius: '0.5rem',
            padding: '0.75rem 1rem',
            fontSize: '0.875rem',
            maxWidth: '400px',
          },
          success: {
            duration: 2500,
            iconTheme: {
              primary: '#10b981',
              secondary: '#1e293b',
            },
            style: {
              border: '1px solid rgba(16, 185, 129, 0.3)',
            },
          },
          error: {
            duration: 4000,
            iconTheme: {
              primary: '#ef4444',
              secondary: '#1e293b',
            },
            style: {
              border: '1px solid rgba(239, 68, 68, 0.3)',
            },
          },
        }}
      />
    </div>
  );
}

// --- Main App (Wrapped with Provider) ---
export default function App() {
  return (
    <ErrorBoundary FallbackComponent={PageErrorFallback}>
      <QueryClientProvider client={queryClient}>
        <AppStoreProvider>
          <AppContent />
        </AppStoreProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}