import { useEffect, lazy } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation } from 'react-router-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { Toaster } from 'react-hot-toast';
import { MemoryRouter } from 'react-router-dom';

// 初始化 i18n
import './i18n';

// React Query
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/queryClient';

// Store 和 Hooks
import { AppStoreProvider } from './components/AppStoreProvider';
import { useAppStore } from './stores/appStore';
import { useWorkspaceSelection } from './hooks/useWorkspaceSelection';
import { useBackendSync } from './hooks/useBackendSync';

// UI 组件
import { Sidebar } from './components/Sidebar';
import { WorkspaceHeader } from './components/WorkspaceHeader';
import { PageTransition } from './components/PageTransition';
import { PageErrorFallback, initGlobalErrorHandlers } from './components/ErrorBoundary';

// Toast 配置
import { toastConfig } from './config/toastConfig';

// 懒加载页面
const SearchPage = lazy(() => import('./pages/SearchPage'));
const KeywordsPage = lazy(() => import('./pages/KeywordsPage'));
const WorkspacesPage = lazy(() => import('./pages/WorkspacesPage'));
const TasksPage = lazy(() => import('./pages/TasksPage'));
const SettingsPage = lazy(() => import('./pages/SettingsPage'));

const pages = {
  SearchPage,
  KeywordsPage,
  WorkspacesPage,
  TasksPage,
  SettingsPage,
};

// --- Main App Component (Internal) ---
function AppContent() {
  const { t } = useTranslation();
  const location = useLocation();
  const currentPage = location.pathname.slice(1) || 'workspaces';

  const isInitialized = useAppStore((state) => state.isInitialized);
  const initializationError = useAppStore((state) => state.initializationError);
  const { workspaces } = useWorkspaceSelection();
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) || null;

  // 后端状态同步
  useBackendSync();

  // 显示初始化加载状态
  if (!isInitialized) {
    return (
      <div className="flex h-screen items-center justify-center bg-bg-main text-text-main">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4" />
          {initializationError ? (
            <div>
              <p className="text-status-error mb-2">Initialization failed</p>
              <p className="text-sm text-text-muted">{initializationError}</p>
            </div>
          ) : (
            <p className="text-text-muted text-sm">Loading application...</p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-bg-main text-text-main font-sans selection:bg-primary/30">
      {/* Skip Link - 键盘导航辅助 */}
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-[1000] focus:px-4 focus:py-2 focus:bg-primary focus:text-white focus:rounded-md focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-bg-main"
      >
        {t('nav.skip_to_content')}
      </a>

      <Sidebar />

      {/* 主内容区 */}
      <div id="main-content" className="flex-1 flex flex-col min-w-0 bg-bg-main">
        <WorkspaceHeader currentPage={currentPage} activeWorkspace={activeWorkspace} />

        <div className="flex-1 overflow-hidden relative">
          <PageTransition pages={pages} currentPage={currentPage} />
        </div>
      </div>

      <Toaster position="bottom-right" toastOptions={toastConfig} />
    </div>
  );
}

// --- Main App (Wrapped with Provider) ---
export default function App() {
  // 初始化全局错误处理器
  useEffect(() => {
    const cleanup = initGlobalErrorHandlers();
    if (import.meta.env.DEV) {
      console.log('[App] Global error handlers initialized');
    }

    return () => {
      if (cleanup) {
        cleanup();
      }
    };
  }, []);

  return (
    <ErrorBoundary FallbackComponent={PageErrorFallback}>
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <AppStoreProvider>
            <AppContent />
          </AppStoreProvider>
        </MemoryRouter>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
