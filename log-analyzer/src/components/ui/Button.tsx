import React from 'react';
import { cn } from '../../utils/classNames';
import type { ButtonProps } from '../../types/ui';

/**
 * 通用按钮组件
 * 支持多种变体样式和图标
 */
export const Button: React.FC<ButtonProps> = ({ 
  children, 
  variant = 'primary', 
  className, 
  icon: Icon, 
  onClick, 
  ...props 
}) => {
  const variants = {
    primary: "bg-primary hover:bg-primary-hover text-white shadow-sm active:scale-95",
    secondary: "bg-bg-card hover:bg-bg-hover text-text-main border border-border-base active:scale-95",
    ghost: "hover:bg-bg-hover text-text-muted hover:text-text-main active:bg-bg-hover/80",
    danger: "bg-red-500/10 text-red-400 hover:bg-red-500/20 border border-red-500/20 hover:text-red-300 active:scale-95",
    active: "bg-primary/20 text-primary border border-primary/50", 
    icon: "h-8 w-8 p-0 bg-transparent hover:bg-bg-hover text-text-dim hover:text-text-main rounded-full"
  };

  return (
    <button 
      type="button" 
      className={cn(
        "h-9 px-4 rounded-md text-sm font-medium transition-colors",
        "flex items-center justify-center gap-2",
        "disabled:opacity-50 disabled:cursor-not-allowed",
        "shrink-0 select-none cursor-pointer",
        variants[variant],
        className
      )} 
      onClick={(e) => { 
        e.stopPropagation(); 
        onClick && onClick(e); 
      }} 
      {...props}
    >
      {Icon && <Icon size={16} />}
      {children}
    </button>
  );
};
