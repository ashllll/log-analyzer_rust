import { useEffect, lazy, Suspense, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Search, LayoutGrid, ListTodo, Cog, Layers,
  Zap, FileText, CheckCircle2, RefreshCw
} from "lucide-react";
import { ErrorBoundary } from 'react-error-boundary';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Toaster } from 'react-hot-toast';
import { MemoryRouter, Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom';
import { AnimatePresence, LayoutGroup, motion } from 'framer-motion';

// 初始化i18n
import './i18n';

// 导入 React Query
import { QueryClientProvider } from '@tanstack/react-query';
import { queryClient } from './lib/queryClient';

// 导入全局Store和Hooks
import { AppStoreProvider } from './components/AppStoreProvider';
import { useAppStore } from './stores/appStore';
import { useWorkspaceSelection } from './hooks/useWorkspaceSelection';
import { useWorkspaceList } from './hooks/useWorkspaceList';
import { useToast } from './hooks/useToast';

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


// 懒加载页面加载骨架 - 使用 h-full 确保在 motion.div(h-full) 容器内撑满
const LoadingSkeleton: React.FC = () => (
  <div className="h-full overflow-auto p-8 space-y-6">
    <div className="flex justify-between items-center">
      <div className="space-y-2">
        <div className="animate-pulse h-7 w-48 rounded bg-bg-hover/40" />
        <div className="animate-pulse h-4 w-72 rounded bg-bg-hover/40" />
      </div>
      <div className="animate-pulse h-10 w-32 rounded bg-bg-hover/40" />
    </div>
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="rounded-lg border border-border-base overflow-hidden">
          <div className="animate-pulse h-12 bg-bg-hover/40 rounded-none" />
          <div className="p-4 space-y-3">
            <div className="animate-pulse h-4 w-full rounded bg-bg-hover/40" />
            <div className="animate-pulse h-4 w-3/4 rounded bg-bg-hover/40" />
          </div>
        </div>
      ))}
    </div>
  </div>
);

// 页面过渡动画 variants
const pageVariants = {
  initial: { opacity: 0, y: 6 },
  enter: { opacity: 1, y: 0, transition: { duration: 0.18, ease: 'easeOut' as const } },
  exit: { opacity: 0, y: -6, transition: { duration: 0.12, ease: 'easeIn' as const } },
};

// --- Main App Component (Internal) ---
function AppContent() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();

  // 当前页面路径
  const currentPage = location.pathname.slice(1) || 'workspaces';

  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  const isInitialized = useAppStore((state) => state.isInitialized);
  const initializationError = useAppStore((state) => state.initializationError);
  const { showToast: addToast } = useToast();

  const { workspaces } = useWorkspaceSelection();
  const { refreshWorkspaces } = useWorkspaceList();

  const activeWorkspace = workspaces.find(w => w.id === activeWorkspaceId) || null;

  // 导航函数 - 使用 useCallback 避免重复创建
  const setPage = useCallback((page: string) => {
    navigate(`/${page}`);
  }, [navigate]);

  // 导航点击处理器 - 使用 useCallback 缓存
  const handleNavClick = useCallback((page: string) => {
    return () => setPage(page);
  }, [setPage]);

  // 导航项配置 - 使用 useMemo 缓存
  const navItems = useMemo(() => [
    { icon: LayoutGrid, label: t('nav.workspaces'), page: "workspaces", testId: "nav-workspaces" },
    { icon: Search, label: t('nav.search'), page: "search", testId: "nav-search" },
    { icon: ListTodo, label: t('nav.keywords'), page: "keywords", testId: "nav-keywords" },
    { icon: Layers, label: t('nav.tasks'), page: "tasks", testId: "nav-tasks" },
  ], [t]);

  // 初始化状态同步并监听工作区事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupStateSync = async () => {
      try {
        // 初始化状态同步
        await invoke('init_state_sync');

        // 监听工作区事件
        unlisten = await listen('workspace-event', (event: { payload: { type: string; status?: { status: string } } }) => {
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

        if (import.meta.env.DEV) {
          console.log('[StateSync] Event listener registered');
        }
      } catch (error) {
        if (import.meta.env.DEV) {
          console.error('[StateSync] Failed to initialize:', error);
        }
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

      {/* 侧边栏 */}
      <div className="w-[240px] bg-gradient-to-b from-bg-sidebar to-bg-main border-r border-border-subtle flex flex-col shrink-0 z-50">
        {/* Logo 区域 */}
        <div className="h-14 flex items-center px-5 border-b border-border-subtle mb-2 select-none">
          <div className="h-8 w-8 bg-gradient-to-br from-primary to-cta rounded-lg flex items-center justify-center text-white mr-3 shadow-lg shadow-primary/30">
            <Zap size={18} fill="currentColor" />
          </div>
          <span className="font-bold text-lg tracking-tight bg-gradient-to-r from-primary-text to-cta-text bg-clip-text text-transparent">
            LogAnalyzer
          </span>
        </div>

        {/* 导航菜单 - LayoutGroup 确保 layoutId 动画跨组件共享 */}
        <LayoutGroup>
          <div className="flex-1 px-3 py-4 space-y-1">
            {navItems.map(({ icon, label, page, testId }) => (
              <NavItem
                key={page}
                icon={icon}
                label={label}
                active={currentPage === page}
                onClick={handleNavClick(page)}
                data-testid={testId}
              />
            ))}
          </div>
          <div className="p-3 border-t border-border-subtle">
            <NavItem
              icon={Cog}
              label={t('nav.settings')}
              active={currentPage === 'settings'}
              onClick={() => setPage('settings')}
              data-testid="nav-settings"
            />
          </div>
        </LayoutGroup>
      </div>

      {/* 主内容区 */}
      <div id="main-content" className="flex-1 flex flex-col min-w-0 bg-bg-main">
        {/* 顶部导航栏 */}
        <div className="h-14 border-b border-border-subtle bg-bg-main flex items-center justify-between px-6 shrink-0 z-40">
          <div className="flex items-center text-sm text-text-muted select-none">
            {currentPage === 'settings' ? (
              <span className="font-medium text-text-main flex items-center gap-2">
                <Cog size={14} className="text-primary-text"/> Settings
              </span>
            ) : (
              <>
                <span className="opacity-50">Workspace / </span>
                <span className="font-medium text-text-main ml-2 flex items-center gap-2">
                  <FileText size={14} className="text-primary-text"/>
                  {activeWorkspace ? activeWorkspace.name : "Select Workspace"}
                </span>
              </>
            )}
          </div>
          {/* 工作区状态 badge */}
          {activeWorkspace && currentPage !== 'settings' && (
            <div className="flex items-center gap-1.5 text-xs font-semibold">
              {activeWorkspace.status === 'READY' ? (
                <>
                  <CheckCircle2 size={12} className="text-cta" />
                  <span className="text-cta">READY</span>
                </>
              ) : (
                <>
                  <RefreshCw size={12} className="text-primary-text animate-spin" />
                  <span className="text-primary-text">PROCESSING</span>
                </>
              )}
            </div>
          )}
        </div>

        <div className="flex-1 overflow-hidden relative">
          {/* ErrorBoundary 移到 Suspense 外部 - React 19 最佳实践 */}
          <ErrorBoundary
            FallbackComponent={CompactErrorFallback}
            onReset={() => {
              if (import.meta.env.DEV) {
                console.log('Error boundary reset, staying on page:', currentPage);
              }
            }}
            resetKeys={[currentPage]}
          >
            {/* AnimatePresence 驱动页面切换过渡动画 */}
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={location.pathname}
                variants={pageVariants}
                initial="initial"
                animate="enter"
                exit="exit"
                className="h-full"
              >
                <Suspense fallback={<LoadingSkeleton />}>
                  {/* Routes 传入 location 以保证 AnimatePresence 的 exit 动画正确触发 */}
                  <Routes location={location}>
                    <Route path="/" element={<Navigate to="/workspaces" replace />} />
                    <Route path="/workspaces" element={<WorkspacesPage />} />
                    <Route path="/search" element={<SearchPage />} />
                    <Route path="/keywords" element={<KeywordsPage />} />
                    <Route path="/tasks" element={<TasksPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                  </Routes>
                </Suspense>
              </motion.div>
            </AnimatePresence>
          </ErrorBoundary>
        </div>
      </div>

      <Toaster
        position="bottom-right"
        toastOptions={{
          duration: 3000,
          style: {
            background: '#27272A', // Zinc-800
            color: '#F4F4F5', // Zinc-100
            border: '1px solid #3F3F46', // Zinc-700
            borderRadius: '0.5rem',
            padding: '0.75rem 1rem',
            fontSize: '0.875rem',
            maxWidth: '400px',
            boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.3)',
          },
          success: {
            duration: 2500,
            iconTheme: {
              primary: '#10B981', // Emerald-500
              secondary: '#27272A',
            },
            style: {
              border: '1px solid rgba(16, 185, 129, 0.4)',
            },
          },
          error: {
            duration: 4000,
            iconTheme: {
              primary: '#EF4444', // Red-500
              secondary: '#27272A',
            },
            style: {
              border: '1px solid rgba(239, 68, 68, 0.4)',
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
