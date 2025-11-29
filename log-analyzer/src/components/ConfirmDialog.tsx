import React from 'react';
import { AlertCircle, X } from 'lucide-react';
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)); }

/**
 * 确认对话框组件
 * 用于危险操作的二次确认
 */

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export const ConfirmDialog = ({
  isOpen,
  title,
  message,
  confirmText = '确认',
  cancelText = '取消',
  danger = false,
  onConfirm,
  onCancel
}: ConfirmDialogProps) => {
  if (!isOpen) return null;

  return (
    <div 
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onCancel}
    >
      <div 
        className="w-[400px] bg-bg-card border border-border-base rounded-lg shadow-2xl animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* 标题栏 */}
        <div className="px-6 py-4 border-b border-border-base flex justify-between items-center bg-bg-sidebar">
          <div className="flex items-center gap-3">
            {danger && <AlertCircle size={20} className="text-red-400" />}
            <h2 className="text-lg font-bold text-text-main">{title}</h2>
          </div>
          <button
            onClick={onCancel}
            className="h-8 w-8 p-0 bg-transparent hover:bg-bg-hover text-text-dim hover:text-text-main rounded-full flex items-center justify-center transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        {/* 内容区 */}
        <div className="p-6">
          <p className="text-sm text-text-muted leading-relaxed">{message}</p>
        </div>

        {/* 操作按钮 */}
        <div className="px-6 py-4 border-t border-border-base bg-bg-sidebar flex justify-end gap-3">
          <button
            onClick={onCancel}
            className="h-9 px-4 rounded-md text-sm font-medium bg-bg-card hover:bg-bg-hover text-text-main border border-border-base active:scale-95 transition-all"
            autoFocus
          >
            {cancelText}
          </button>
          <button
            onClick={() => {
              onConfirm();
              onCancel();
            }}
            className={cn(
              "h-9 px-4 rounded-md text-sm font-medium transition-all active:scale-95",
              danger
                ? "bg-red-500/10 text-red-400 hover:bg-red-500/20 border border-red-500/20 hover:text-red-300"
                : "bg-primary hover:bg-primary-hover text-white shadow-sm"
            )}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
};

/**
 * 使用示例Hook
 */
export const useConfirmDialog = () => {
  const [dialogState, setDialogState] = React.useState<{
    isOpen: boolean;
    title: string;
    message: string;
    danger: boolean;
    onConfirm: () => void;
  }>({
    isOpen: false,
    title: '',
    message: '',
    danger: false,
    onConfirm: () => {}
  });

  const showConfirm = (
    title: string,
    message: string,
    onConfirm: () => void,
    danger = false
  ) => {
    setDialogState({
      isOpen: true,
      title,
      message,
      danger,
      onConfirm
    });
  };

  const closeDialog = () => {
    setDialogState(prev => ({ ...prev, isOpen: false }));
  };

  const Dialog = () => (
    <ConfirmDialog
      {...dialogState}
      onCancel={closeDialog}
    />
  );

  return {
    showConfirm,
    Dialog
  };
};
