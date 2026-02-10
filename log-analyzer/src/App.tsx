import { useState, useRef, useEffect, lazy, Suspense } from "react";
import {
  Search, LayoutGrid, ListTodo, Cog, Layers,
  Zap, Loader2, FileText, Activity
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
// 导入错误边界组件
import { PageErrorFallback, CompactErrorFallback, initGlobalErrorHandlers } from './components/ErrorBoundary';

// 恢复懒加载以优化性能，但确保 ErrorBoundary 在外层
const SearchPage = lazy(() => import('./pages/SearchPage'));
const KeywordsPage = lazy(() => import('./pages/KeywordsPage'));
const WorkspacesPage = lazy(() => import('./pages/WorkspacesPage'));
const TasksPage = lazy(() => import('./pages/TasksPage'));
const SettingsPage = lazy(() => import('./pages/SettingsPage'));
const PerformancePage = lazy(() => import('./pages/PerformancePage'));

// 懒加载页面加载骨架
const PageSkeleton: React.FC = () => (
  <div className="flex items-center justify-center h-full">
    <div className="text-center">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
      <p className="text-sm text-text-muted">Loading...</p>
    </div>
  </div>
);

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

  // 初始化状态同步并监听工作区事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupStateSync = async () => {
      try {
        // 初始化状态同步
        await invoke('init_state_sync');

        // 标记应用已初始化
        addToast('success', 'Application initialized successfully');

        // 监听工作区事件
        unlisten = await listen('workspace-event', (event: any) => {
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
            <NavItem icon={LayoutGrid} label="Workspaces" active={page === 'workspaces'} onClick={() => setPage('workspaces')} data-testid="nav-workspaces" />
            <NavItem icon={Search} label="Search Logs" active={page === 'search'} onClick={() => setPage('search')} data-testid="nav-search" />
            <NavItem icon={ListTodo} label="Keywords" active={page === 'keywords'} onClick={() => setPage('keywords')} data-testid="nav-keywords" />
            <NavItem icon={Layers} label="Tasks" active={page === 'tasks'} onClick={() => setPage('tasks')} data-testid="nav-tasks" />
            <NavItem icon={Activity} label="Performance" active={page === 'performance'} onClick={() => setPage('performance')} data-testid="nav-performance" />
        </div>
        {importStatus && <div className="p-3 m-3 bg-bg-card border border-primary/20 rounded text-xs text-primary animate-pulse"><div className="font-bold mb-1 flex items-center gap-2"><Loader2 size={12} className="animate-spin"/> Processing</div><div className="truncate opacity-80">{importStatus}</div></div>}
        <div className="p-3 border-t border-border-base">
          <NavItem icon={Cog} label="Settings" active={page === 'settings'} onClick={() => setPage('settings')} data-testid="nav-settings" />
        </div>
      </div>
      <div className="flex-1 flex flex-col min-w-0 bg-bg-main">
        <div className="h-14 border-b border-border-base bg-bg-main flex items-center justify-between px-6 shrink-0 z-40">
          <div className="flex items-center text-sm text-text-muted select-none">
            {page === 'settings' ? (
              <span className="font-medium text-text-main flex items-center gap-2">
                <Cog size={14} className="text-primary"/> Settings
              </span>
            ) : page === 'performance' ? (
              <span className="font-medium text-text-main flex items-center gap-2">
                <Activity size={14} className="text-primary"/> Performance
              </span>
            ) : (
              <>
                <span className="opacity-50">Workspace / </span>
                <span className="font-medium text-text-main ml-2 flex items-center gap-2">
                  <FileText size={14} className="text-primary"/> {activeWorkspace ? activeWorkspace.name : "Select Workspace"}
                </span>
              </>
            )}
          </div>
        </div>
        <div className="flex-1 overflow-hidden relative">
          {/* ErrorBoundary 移到 Suspense 外部 - React 19 最佳实践 */}
          <ErrorBoundary 
            FallbackComponent={CompactErrorFallback} 
            onReset={() => {
              // 清除错误状态并保持在当前页面
              console.log('Error boundary reset, staying on page:', page);
            }}
            resetKeys={[page]}
          >
            <Suspense fallback={<PageSkeleton />}>
              {page === 'search' && <SearchPage keywordGroups={keywordGroups} addToast={addToast} searchInputRef={searchInputRef} activeWorkspace={activeWorkspace} />}
              {page === 'keywords' && <KeywordsPage />}
              {page === 'workspaces' && <WorkspacesPage />}
              {page === 'tasks' && <TasksPage />}
              {page === 'performance' && <PerformancePage />}
              {page === 'settings' && <SettingsPage />}
            </Suspense>
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
  // 初始化全局错误处理器
  useEffect(() => {
    const cleanup = initGlobalErrorHandlers();
    console.log('[App] Global error handlers initialized');

    return () => {
      if (cleanup) {
        cleanup();
      }
    };
  }, []);

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