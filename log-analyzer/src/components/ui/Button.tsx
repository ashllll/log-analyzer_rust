import { forwardRef } from 'react';
import { cn } from '../../utils/classNames';
import type { ButtonProps } from '../../types/ui';

/**
 * 扩展的按钮属性接口
 */
export interface ExtendedButtonProps extends ButtonProps {
  loading?: boolean;
}

/**
 * 通用按钮组件
 * 支持多种变体样式、图标、加载状态
 *
 * 变体说明:
 * - primary: 主要操作按钮，蓝色背景
 * - secondary: 次要操作，带边框
 * - ghost: 透明背景，仅悬停时显示
 * - danger: 危险操作，红色调
 * - active: 激活状态，蓝色边框
 * - cta: 行动号召按钮，绿色强调
 * - icon: 图标按钮，圆形
 */
export const Button = forwardRef<HTMLButtonElement, ExtendedButtonProps>(
  ({
    children,
    variant = 'primary',
    className,
    icon: Icon,
    loading = false,
    disabled,
    onClick,
    ...props
  }, ref) => {
  const variants = {
    primary: "bg-primary hover:bg-primary-hover text-white shadow-sm hover:shadow-glow-primary active:scale-[0.98]",
    secondary: "bg-bg-card hover:bg-bg-hover text-text-main border border-border-base hover:border-border-light active:scale-[0.98]",
    ghost: "hover:bg-bg-hover text-text-muted hover:text-text-main active:bg-bg-hover/80",
    danger: "bg-status-error/10 text-status-error hover:bg-status-error/20 border border-status-error/20 hover:border-status-error/40 active:scale-[0.98]",
    active: "bg-primary/20 text-primary border border-primary/50 hover:bg-primary/30",
    cta: "bg-cta hover:bg-cta-hover text-white shadow-sm hover:shadow-glow-cta active:scale-[0.98]",
    icon: "h-11 w-11 p-0 bg-transparent hover:bg-bg-hover text-text-dim hover:text-text-main rounded-lg"
  };

  const isDisabled = disabled || loading;

  return (
    <button
      ref={ref}
      type="button"
      className={cn(
        "h-11 px-4 rounded-md text-sm font-medium transition-all duration-200 touch-target",
        "flex items-center justify-center gap-2",
        "disabled:opacity-50 disabled:cursor-not-allowed",
        "shrink-0 select-none cursor-pointer",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-bg-main",
        variants[variant],
        className
      )}
      disabled={isDisabled}
      onClick={(e) => {
        e.stopPropagation();
        if (!isDisabled && onClick) {
          onClick(e);
        }
      }}
      {...props}
    >
      {loading ? (
        <svg
          className="animate-spin h-4 w-4"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
      ) : (
        Icon && <Icon size={16} aria-hidden="true" />
      )}
      {children}
    </button>
  );
});

Button.displayName = 'Button';
