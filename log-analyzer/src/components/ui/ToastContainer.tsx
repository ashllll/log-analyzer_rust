import React from 'react';
import { CheckCircle2, AlertCircle, X, Info } from 'lucide-react';
import { cn } from '../../utils/classNames';
import type { Toast } from '../../types/common';

interface ToastContainerProps {
  toasts: Toast[];
  removeToast: (id: number) => void;
}

/**
 * Toast通知容器组件
 */
export const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, removeToast }) => {
  return (
    <div className="fixed bottom-6 right-6 z-[100] flex flex-col gap-3 pointer-events-none">
      {toasts.map(toast => (
        <div 
          key={toast.id} 
          className={cn(
            "pointer-events-auto min-w-[300px] p-4 rounded-lg shadow-2xl border",
            "flex items-center gap-3 animate-in slide-in-from-right-full duration-300",
            toast.type === 'success' 
              ? "bg-bg-card border-emerald-500/30 text-emerald-400" 
              : toast.type === 'error' 
                ? "bg-bg-card border-red-500/30 text-red-400" 
                : "bg-bg-card border-blue-500/30 text-blue-400"
          )}
        >
          {toast.type === 'success' ? (
            <CheckCircle2 size={20} />
          ) : toast.type === 'error' ? (
            <AlertCircle size={20} />
          ) : (
            <Info size={20} />
          )}
          <span className="text-sm font-medium text-text-main">{toast.message}</span>
          <button 
            onClick={() => removeToast(toast.id)} 
            className="ml-auto text-text-dim hover:text-text-main"
          >
            <X size={16} />
          </button>
        </div>
      ))}
    </div>
  );
};
