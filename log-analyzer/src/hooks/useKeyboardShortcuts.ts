import { useEffect } from 'react';

/**
 * 键盘快捷键Hook
 * 提供全局键盘快捷键支持
 */

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
  handler: (e: KeyboardEvent) => void;
  description?: string;
}

export const useKeyboardShortcuts = (shortcuts: KeyboardShortcut[]) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      for (const shortcut of shortcuts) {
        const ctrlMatch = shortcut.ctrl === undefined || shortcut.ctrl === (e.ctrlKey || e.metaKey);
        const metaMatch = shortcut.meta === undefined || shortcut.meta === e.metaKey;
        const shiftMatch = shortcut.shift === undefined || shortcut.shift === e.shiftKey;
        const altMatch = shortcut.alt === undefined || shortcut.alt === e.altKey;
        const keyMatch = shortcut.key.toLowerCase() === e.key.toLowerCase();

        if (ctrlMatch && metaMatch && shiftMatch && altMatch && keyMatch) {
          e.preventDefault();
          shortcut.handler(e);
          break;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [shortcuts]);
};

/**
 * 全局快捷键常量
 */
export const SHORTCUTS = {
  FOCUS_SEARCH: { key: 'k', ctrl: true, description: '聚焦搜索框' },
  OPEN_SETTINGS: { key: ',', ctrl: true, description: '打开设置' },
  CLOSE_PANEL: { key: 'Escape', description: '关闭面板/对话框' },
  NEW_WORKSPACE: { key: 'n', ctrl: true, description: '新建工作区' },
  REFRESH: { key: 'r', ctrl: true, description: '刷新当前工作区' },
};
