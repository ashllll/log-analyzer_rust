import React from 'react';
import { cn } from '../../utils/classNames';
import type { InputProps } from '../../types/ui';

/**
 * 通用输入框组件
 */
export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, ...props }, ref) => {
    return (
      <input 
        ref={ref}
        className={cn(
          "h-9 w-full bg-bg-main border border-border-base rounded-md px-3",
          "text-sm text-text-main placeholder:text-text-dim",
          "focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/50",
          "transition-all",
          className
        )} 
        {...props} 
      />
    );
  }
);

Input.displayName = 'Input';
