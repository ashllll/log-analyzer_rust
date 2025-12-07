import React from 'react';
import { cn } from '../utils/classNames';

/**
 * 骨架屏组件 - 替代空白加载状态
 */

// 基础骨架元素
export const Skeleton = ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => {
  return (
    <div
      className={cn(
        "animate-pulse rounded bg-bg-hover/50",
        className
      )}
      {...props}
    />
  );
};

// 工作区卡片骨架屏
export const WorkspaceCardSkeleton = () => {
  return (
    <div className="bg-bg-card border border-border-base rounded-lg p-4 space-y-4">
      <div className="flex justify-between items-center">
        <Skeleton className="h-5 w-32" />
        <div className="flex gap-1">
          <Skeleton className="h-6 w-6 rounded-full" />
          <Skeleton className="h-6 w-6 rounded-full" />
          <Skeleton className="h-6 w-6 rounded-full" />
        </div>
      </div>
      <Skeleton className="h-8 w-full" />
      <div className="flex items-center gap-2">
        <Skeleton className="h-4 w-4 rounded-full" />
        <Skeleton className="h-4 w-24" />
      </div>
    </div>
  );
};

// 任务卡片骨架屏
export const TaskCardSkeleton = () => {
  return (
    <div className="p-4 bg-bg-card border border-border-base rounded-lg flex items-center gap-4">
      <Skeleton className="h-10 w-10 rounded-full" />
      <div className="flex-1 space-y-2">
        <div className="flex justify-between">
          <Skeleton className="h-4 w-48" />
          <Skeleton className="h-4 w-20" />
        </div>
        <Skeleton className="h-2 w-full rounded-full" />
        <div className="flex justify-between">
          <Skeleton className="h-3 w-32" />
          <Skeleton className="h-3 w-12" />
        </div>
      </div>
      <Skeleton className="h-8 w-8 rounded" />
    </div>
  );
};

// 关键词组卡片骨架屏
export const KeywordCardSkeleton = () => {
  return (
    <div className="bg-bg-card border border-border-base rounded-lg overflow-hidden">
      <div className="px-6 py-4 flex items-center justify-between bg-bg-sidebar/30 border-b border-border-base/50">
        <div className="flex items-center gap-4">
          <Skeleton className="w-3 h-3 rounded-full" />
          <Skeleton className="h-4 w-32" />
        </div>
        <div className="flex gap-2">
          <Skeleton className="h-8 w-16" />
          <Skeleton className="h-8 w-16" />
        </div>
      </div>
      <div className="px-6 py-3 flex flex-wrap gap-2">
        <Skeleton className="h-8 w-24" />
        <Skeleton className="h-8 w-32" />
        <Skeleton className="h-8 w-28" />
      </div>
    </div>
  );
};

// 日志列表骨架屏
export const LogListSkeleton = () => {
  return (
    <div className="space-y-1">
      {[...Array(10)].map((_, i) => (
        <div key={i} className="grid grid-cols-[50px_160px_200px_1fr] px-3 py-2 border-b border-border-base/40">
          <Skeleton className="h-4 w-8" />
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-4 w-40" />
          <Skeleton className="h-4 w-full" />
        </div>
      ))}
    </div>
  );
};

// 统计卡片骨架屏
export const StatsCardSkeleton = () => {
  return (
    <div className="bg-bg-card border border-border-base rounded-lg p-6 space-y-3">
      <Skeleton className="h-3 w-24" />
      <Skeleton className="h-10 w-32" />
      <Skeleton className="h-3 w-20" />
    </div>
  );
};

// 通用列表骨架屏
export const ListSkeleton = ({ count = 5, ItemComponent = WorkspaceCardSkeleton }) => {
  return (
    <div className="space-y-4">
      {[...Array(count)].map((_, i) => (
        <ItemComponent key={i} />
      ))}
    </div>
  );
};
