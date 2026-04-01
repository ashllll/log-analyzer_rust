import React from 'react';
import { cn } from '../../utils/classNames';
import { Button } from './Button';
import type { ButtonVariant, LucideIcon } from '../../types/ui';

interface EmptyStateAction {
  label: string;
  onClick: () => void;
  icon?: LucideIcon;
  variant?: ButtonVariant;
}

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: EmptyStateAction;
  className?: string;
}

/**
 * 空状态组件
 * 统一展示列表/页面无数据时的引导界面
 */
export const EmptyState: React.FC<EmptyStateProps> = ({
  icon: Icon,
  title,
  description,
  action,
  className,
}) => {
  return (
    <div className={cn("flex flex-col items-center justify-center py-16 px-8 text-center", className)}>
      <div className="h-14 w-14 rounded-2xl bg-primary/10 border border-primary/20 flex items-center justify-center mb-4">
        <Icon size={26} className="text-primary-text" aria-hidden="true" />
      </div>
      <h3 className="text-base font-semibold text-text-main mb-1">{title}</h3>
      {description && (
        <p className="text-sm text-text-muted max-w-xs mb-5">{description}</p>
      )}
      {action && (
        <Button
          variant={action.variant ?? 'cta'}
          icon={action.icon}
          onClick={action.onClick}
        >
          {action.label}
        </Button>
      )}
    </div>
  );
};
