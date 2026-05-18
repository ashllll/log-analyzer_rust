import { Suspense } from 'react';
import { Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { AnimatePresence, motion } from 'framer-motion';
import { ErrorBoundary } from 'react-error-boundary';
import { CompactErrorFallback } from './ErrorBoundary';

// 页面过渡动画 variants
const pageVariants = {
  initial: { opacity: 0, y: 6 },
  enter: { opacity: 1, y: 0, transition: { duration: 0.18, ease: 'easeOut' as const } },
  exit: { opacity: 0, y: -6, transition: { duration: 0.12, ease: 'easeIn' as const } },
};

// 懒加载页面加载骨架
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

interface PageTransitionProps {
  pages: Record<string, React.LazyExoticComponent<React.ComponentType>>;
  currentPage: string;
}

/**
 * 页面过渡动画包装器
 * 集成 AnimatePresence、ErrorBoundary、Suspense 和 Routes
 */
export const PageTransition: React.FC<PageTransitionProps> = ({ pages, currentPage }) => {
  const location = useLocation();

  return (
    <ErrorBoundary
      FallbackComponent={CompactErrorFallback}
      onReset={() => {
        // Error boundary reset, staying on current page
      }}
      resetKeys={[currentPage]}
    >
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
            <Routes location={location}>
              <Route path="/" element={<Navigate to="/workspaces" replace />} />
              <Route path="/workspaces" element={<pages.WorkspacesPage />} />
              <Route path="/search" element={<pages.SearchPage />} />
              <Route path="/keywords" element={<pages.KeywordsPage />} />
              <Route path="/tasks" element={<pages.TasksPage />} />
              <Route path="/settings" element={<pages.SettingsPage />} />
            </Routes>
          </Suspense>
        </motion.div>
      </AnimatePresence>
    </ErrorBoundary>
  );
};
