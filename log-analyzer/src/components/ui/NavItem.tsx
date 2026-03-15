import React from 'react';
import { cn } from '../../utils/classNames';
import type { NavItemProps } from '../../types/ui';

/**
 * 导航项组件
 * 支持活跃状态、悬停效果和键盘导航
 */
export const NavItem: React.FC<NavItemProps> = ({
  icon: Icon,
  label,
  active,
  onClick,
  'data-testid': dataTestId,
}) => {
  return (
    <button
      onClick={onClick}
      data-testid={dataTestId}
      className={cn(
        "w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-200",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-bg-sidebar",
        active
          ? "bg-primary text-white shadow-sm hover:bg-primary-hover"
          : "text-text-muted hover:text-text-main hover:bg-bg-hover"
      )}
    >
      <Icon size={18} aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
};
