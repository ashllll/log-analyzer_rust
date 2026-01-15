// UI 组件相关类型定义
import React from 'react';
import { KeywordGroup } from './common';

// 按钮变体类型
export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'active' | 'icon';

// Lucide 图标类型
export type LucideIcon = React.ComponentType<{ size?: number; className?: string }>;

// 导航项属性
export interface NavItemProps {
  icon: LucideIcon;
  label: string;
  active: boolean;
  onClick: () => void;
  'data-testid'?: string;
}

// 按钮属性
export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children?: React.ReactNode;
  variant?: ButtonVariant;
  icon?: LucideIcon;
}

// 输入框属性
export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  ref?: React.Ref<HTMLInputElement | null>;
}

// 卡片属性
export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

// 关键词模态框属性
export interface KeywordModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (group: KeywordGroup) => void;
  initialData?: KeywordGroup | null;
}

// 日志渲染器属性
export interface HybridLogRendererProps {
  text: string;
  query: string;
  keywordGroups: KeywordGroup[];
}

// 过滤面板属性
export interface FilterPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  groups: KeywordGroup[];
  currentQuery: string;
  onToggleRule: (regex: string) => void;
}
