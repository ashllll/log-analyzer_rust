import React from 'react';
import { cn } from '../../utils/classNames';
import type { CardProps } from '../../types/ui';

/**
 * 通用卡片组件
 */
export const Card: React.FC<CardProps> = ({ children, className, ...props }) => {
  return (
    <div 
      className={cn(
        "bg-bg-card border border-border-base rounded-lg overflow-hidden",
        className
      )} 
      {...props}
    >
      {children}
    </div>
  );
};
