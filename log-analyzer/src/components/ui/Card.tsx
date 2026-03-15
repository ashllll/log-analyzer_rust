import React from 'react';
import { cn } from '../../utils/classNames';
import type { CardProps } from '../../types/ui';

/**
 * 通用卡片组件
 *
 * 变体说明:
 * - default: 标准卡片，带边框和阴影
 * - interactive: 可点击卡片，带悬停效果
 * - elevated: 浮动卡片，更强的阴影
 * - ghost: 透明卡片，无边框
 */
export const Card: React.FC<CardProps & {
  variant?: 'default' | 'interactive' | 'elevated' | 'ghost';
  padding?: 'none' | 'sm' | 'md' | 'lg';
}> = ({
  children,
  className,
  variant = 'default',
  padding = 'md',
  ...props
}) => {
  const variants = {
    default: "bg-bg-card border border-border-base shadow-card",
    interactive: "bg-bg-card border border-border-base shadow-card hover:shadow-card-hover hover:border-border-light cursor-pointer transition-all duration-200",
    elevated: "bg-bg-elevated border border-border-subtle shadow-elevated",
    ghost: "bg-transparent",
  };

  const paddings = {
    none: "",
    sm: "p-2",
    md: "p-4",
    lg: "p-6",
  };

  return (
    <div
      className={cn(
        "rounded-lg overflow-hidden transition-colors duration-200",
        variants[variant],
        paddings[padding],
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
};

/**
 * 卡片头部组件
 */
export const CardHeader: React.FC<{ children: React.ReactNode; className?: string }> = ({
  children,
  className
}) => (
  <div className={cn("px-4 py-3 border-b border-border-base", className)}>
    {children}
  </div>
);

/**
 * 卡片内容组件
 */
export const CardContent: React.FC<{ children: React.ReactNode; className?: string }> = ({
  children,
  className
}) => (
  <div className={cn("p-4", className)}>
    {children}
  </div>
);

/**
 * 卡片底部组件
 */
export const CardFooter: React.FC<{ children: React.ReactNode; className?: string }> = ({
  children,
  className
}) => (
  <div className={cn("px-4 py-3 border-t border-border-base bg-bg-sidebar/50", className)}>
    {children}
  </div>
);
