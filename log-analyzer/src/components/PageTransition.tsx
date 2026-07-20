import { Suspense } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { ErrorBoundary } from "react-error-boundary";
import { CompactErrorFallback } from "./ErrorBoundary";
import { Skeleton } from "./ui";

// 懒加载页面加载骨架
const LoadingSkeleton: React.FC = () => (
  <div className="h-full overflow-auto p-8 space-y-6">
    <div className="flex justify-between items-center">
      <div className="space-y-2">
        <Skeleton className="h-7 w-48" />
        <Skeleton className="h-4 w-72" />
      </div>
      <Skeleton className="h-10 w-32" />
    </div>
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {Array.from({ length: 6 }).map((_, i) => (
        <div
          key={i}
          className="rounded-lg border border-border-base overflow-hidden"
        >
          <Skeleton className="h-12 rounded-none" />
          <div className="p-4 space-y-3">
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-3/4" />
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
 * 页面路由包装器：高频导航即时切换，同时保留错误与懒加载状态。
 */
export const PageTransition: React.FC<PageTransitionProps> = ({
  pages,
  currentPage,
}) => {
  return (
    <ErrorBoundary
      FallbackComponent={CompactErrorFallback}
      onReset={() => {
        // Error boundary reset, staying on current page
      }}
      resetKeys={[currentPage]}
    >
      <div className="h-full">
        <Suspense fallback={<LoadingSkeleton />}>
          <Routes>
            <Route path="/" element={<Navigate to="/workspaces" replace />} />
            <Route path="/workspaces" element={<pages.WorkspacesPage />} />
            <Route path="/search" element={<pages.SearchPage />} />
            <Route path="/keywords" element={<pages.KeywordsPage />} />
            <Route path="/tasks" element={<pages.TasksPage />} />
            <Route path="/settings" element={<pages.SettingsPage />} />
          </Routes>
        </Suspense>
      </div>
    </ErrorBoundary>
  );
};
