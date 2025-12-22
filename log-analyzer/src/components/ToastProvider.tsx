import React from 'react';
import { Toaster } from 'react-hot-toast';

/**
 * Toast Provider using react-hot-toast
 * Industry-standard toast notification solution with automatic lifecycle management
 */
export const ToastProvider: React.FC = () => {
  return (
    <Toaster
      position="bottom-right"
      toastOptions={{
        duration: 3000,
        style: {
          background: '#1e293b', // bg-bg-card
          color: '#e2e8f0', // text-text-main
          border: '1px solid rgba(148, 163, 184, 0.2)',
          borderRadius: '0.5rem',
          padding: '1rem',
          minWidth: '300px',
        },
        success: {
          iconTheme: {
            primary: '#10b981', // emerald-500
            secondary: '#1e293b',
          },
          style: {
            border: '1px solid rgba(16, 185, 129, 0.3)',
          },
        },
        error: {
          iconTheme: {
            primary: '#ef4444', // red-500
            secondary: '#1e293b',
          },
          style: {
            border: '1px solid rgba(239, 68, 68, 0.3)',
          },
        },
      }}
    />
  );
};
