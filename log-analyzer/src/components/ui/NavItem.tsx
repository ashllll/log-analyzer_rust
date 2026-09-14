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
      aria-label={label}
      aria-current={active ? "page" : undefined}
      title={label}
      className={cn(
        "ui-pressable relative flex h-10 w-full items-center justify-center rounded-[10px] px-2 text-sm font-medium xl:justify-start xl:gap-3 xl:px-3",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-bg-sidebar",
        active
          ? "border border-primary/20 bg-primary/15 text-primary-text"
          : "text-text-muted hover:text-text-main hover:bg-bg-hover/60"
      )}
    >
      <Icon size={18} aria-hidden="true" className="shrink-0" />
      <span className="hidden truncate xl:inline">{label}</span>
    </button>
  );
};
