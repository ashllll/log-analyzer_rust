import React from "react";
import { cn } from "../../utils/classNames";
import type { NavItemProps } from "../../types/ui";

/**
 * 导航项组件
 * 支持活跃状态（带 Framer Motion layoutId 滑动指示器）、悬停效果和键盘导航
 */
export const NavItem: React.FC<NavItemProps> = ({
  icon: Icon,
  label,
  active,
  onClick,
  "data-testid": dataTestId,
}) => {
  return (
    <button
      onClick={onClick}
      data-testid={dataTestId}
      className={cn(
        "relative w-full flex items-center gap-3 px-3 py-2.5 rounded-[10px] text-sm font-medium transition-colors duration-150",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-bg-sidebar",
        active
          ? "text-text-main bg-primary/15 border border-primary/15"
          : "text-text-muted hover:text-text-main hover:bg-bg-hover"
      )}
    >
      <span className="flex items-center gap-3">
        <Icon size={18} aria-hidden="true" />
        <span>{label}</span>
      </span>
    </button>
  );
};
