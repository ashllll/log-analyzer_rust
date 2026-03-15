import React from 'react';
import { cn } from '../../utils/classNames';
import type { InputProps } from '../../types/ui';

/**
 * 通用输入框组件
 * 支持多种状态、focus 效果和键盘导航
 */
export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={cn(
          "h-9 w-full bg-bg-card border border-border-base rounded-md px-3",
          "text-sm text-text-main placeholder:text-text-dim",
          // Focus 状态
          "focus:outline-none focus:border-primary focus:ring-2 focus:ring-primary/20",
          // Focus-visible 状态 (键盘导航)
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-bg-main",
          // Hover 状态
          "hover:border-border-light transition-colors duration-200",
          // 禁用状态
          "disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-bg-sidebar",
          className
        )}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';
