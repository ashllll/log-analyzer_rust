import React from 'react';
import { cn } from '../../utils/classNames';
import type { NavItemProps } from '../../types/ui';

/**
 * 导航项组件
 */
export const NavItem: React.FC<NavItemProps> = ({ icon: Icon, label, active, onClick }) => {
  return (
    <button 
      onClick={onClick}
      className={cn(
        "w-full flex items-center gap-3 px-3 py-2 rounded-md transition-all",
        active ? "bg-primary text-white" : "text-text-muted hover:bg-bg-hover"
      )}
    >
      <Icon size={18} />
      {label}
    </button>
  );
};
