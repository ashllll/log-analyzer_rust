import React from 'react';
import { cn } from '../../utils/classNames';

interface SkeletonProps {
  className?: string;
}

/**
 * 骨架屏组件
 * 用于替代加载时的旋转圈圈，提供更好的视觉体验
 */
export const Skeleton: React.FC<SkeletonProps> = ({ className }) => (
  <div className={cn("animate-pulse rounded bg-bg-hover/40", className)} />
);

/**
 * 页面级骨架屏 - 侧边栏 + 内容区形状
 */
export const PageSkeleton: React.FC = () => (
  <div className="flex h-full gap-0">
    {/* 内容区骨架 */}
    <div className="flex-1 p-8 space-y-6">
      {/* 标题行 */}
      <div className="flex justify-between items-center">
        <div className="space-y-2">
          <Skeleton className="h-7 w-48" />
          <Skeleton className="h-4 w-72" />
        </div>
        <Skeleton className="h-10 w-32" />
      </div>
      {/* 内容网格 */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="rounded-lg border border-border-base overflow-hidden">
            <Skeleton className="h-12 rounded-none" />
            <div className="p-4 space-y-3">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-3/4" />
            </div>
          </div>
        ))}
      </div>
    </div>
  </div>
);
