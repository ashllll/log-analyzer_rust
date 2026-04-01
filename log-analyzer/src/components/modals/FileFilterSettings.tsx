import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Plus, Trash2, Info } from 'lucide-react';
import { Button } from '../ui';
import type { FileFilterConfig } from '../../types/common';
import { FilterMode } from '../../types/common';
import { api } from '../../services/api';
import { getFullErrorMessage } from '../../services/errors';

interface FileFilterSettingsProps {
  isOpen: boolean;
  onClose: () => void;
  onSaved?: () => void;
}

/**
 * 文件过滤设置模态框组件
 * 用于配置文件类型过滤规则（三层检测策略）
 */
const FileFilterSettings: React.FC<FileFilterSettingsProps> = ({
  isOpen,
  onClose,
  onSaved
}) => {
  const { t } = useTranslation();
  const [config, setConfig] = useState<FileFilterConfig>({
    enabled: false,
    binary_detection_enabled: true,
    mode: FilterMode.Whitelist,
    filename_patterns: [],
    allowed_extensions: [],
    forbidden_extensions: []
  });

  const [newPattern, setNewPattern] = useState('');
  const [newExtension, setNewExtension] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // AbortController ref 用于管理异步操作的取消
  const abortControllerRef = useRef<AbortController | null>(null);

  // ESC 键关闭模态框
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  // 当模态框打开时，加载当前配置（使用 AbortController 防止竞态条件）
  useEffect(() => {
    if (!isOpen) return;

    // 取消之前的请求
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    const controller = new AbortController();
    abortControllerRef.current = controller;

    const loadConfig = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const loadedConfig = await api.getFileFilterConfig();
        if (!controller.signal.aborted) {
          setConfig(loadedConfig);
        }
      } catch (err) {
        if (!controller.signal.aborted) {
          console.error('Failed to load file filter config:', err);
          setError(t('file_filter.load_error', { error: getFullErrorMessage(err) }));
        }
      } finally {
        if (!controller.signal.aborted) {
          setIsLoading(false);
        }
      }
    };

    loadConfig();

    // 组件卸载或 isOpen 变化时取消请求
    return () => {
      controller.abort();
      if (abortControllerRef.current === controller) {
        abortControllerRef.current = null;
      }
    };
  }, [isOpen, t]);

  // 组件卸载时清理所有进行中的操作
  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }
    };
  }, []);

  const handleSave = useCallback(async () => {
    // 取消之前的保存操作
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    const controller = new AbortController();
    abortControllerRef.current = controller;

    try {
      setIsLoading(true);
      setError(null);
      await api.saveFileFilterConfig(config);

      // 检查是否被取消
      if (controller.signal.aborted) {
        return;
      }

      onSaved?.();
      onClose();
    } catch (err) {
      // 忽略取消错误
      if (err instanceof Error && err.name === 'AbortError') {
        return;
      }
      console.error('Failed to save file filter config:', err);
      setError(t('file_filter.save_error', { error: getFullErrorMessage(err) }));
    } finally {
      // 检查当前 controller 是否仍然有效（避免状态更新到已卸载组件）
      if (abortControllerRef.current === controller) {
        setIsLoading(false);
      }
    }
  }, [config, onSaved, onClose, t]);

  const addPattern = useCallback(() => {
    const pattern = newPattern.trim();
    if (pattern) {
      setConfig(prev => {
        if (prev.filename_patterns.includes(pattern)) return prev;
        return {
          ...prev,
          filename_patterns: [...prev.filename_patterns, pattern]
        };
      });
      setNewPattern('');
    }
  }, [newPattern]);

  const removePattern = useCallback((pattern: string) => {
    setConfig(prev => ({
      ...prev,
      filename_patterns: prev.filename_patterns.filter(p => p !== pattern)
    }));
  }, []);

  const addExtension = useCallback(() => {
    const ext = newExtension.trim().toLowerCase().replace(/^\./, '');
    if (ext) {
      setConfig(prev => {
        if (prev.allowed_extensions.includes(ext)) return prev;
        return {
          ...prev,
          allowed_extensions: [...prev.allowed_extensions, ext]
        };
      });
      setNewExtension('');
    }
  }, [newExtension]);

  const removeExtension = useCallback((ext: string) => {
    setConfig(prev => ({
      ...prev,
      allowed_extensions: prev.allowed_extensions.filter(e => e !== ext)
    }));
  }, []);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="w-[700px] bg-bg-card border border-border-base rounded-lg shadow-2xl flex flex-col max-h-[85vh] animate-in fade-in zoom-in-95 duration-200"
        onClick={e => e.stopPropagation()}
      >
        {/* 标题栏 */}
        <div className="px-6 py-4 border-b border-border-base flex justify-between items-center bg-bg-sidebar">
          <h2 className="text-lg font-bold text-text-main">
            {t('file_filter.title')}
          </h2>
          <Button variant="icon" icon={X} onClick={onClose} aria-label={t('file_filter.close')} />
        </div>

        {/* 表单内容 */}
        <div className="p-6 overflow-y-auto flex-1 space-y-6">
          {error && (
            <div className="bg-red-500/10 border border-red-500/30 text-red-500 px-4 py-2 rounded text-sm">
              {error}
            </div>
          )}

          {/* 说明信息 */}
          <div className="bg-log-info/10 border border-log-info/30 text-log-info px-4 py-3 rounded text-sm">
            <div className="flex items-start gap-2">
              <Info className="w-4 h-4 mt-0.5 flex-shrink-0" />
              <div className="space-y-1">
                <p className="font-semibold">{t('file_filter.three_layer_strategy')}</p>
                <ul className="text-xs space-y-0.5 text-log-info/80">
                  <li>• {t('file_filter.layer1_binary')}</li>
                  <li>• {t('file_filter.layer2_smart')}</li>
                  <li>• {t('file_filter.defensive_design')}</li>
                </ul>
              </div>
            </div>
          </div>

          {/* 第1层：二进制检测 */}
          <div className="space-y-2">
            <label className="text-xs text-text-dim uppercase font-bold block">
              {t('file_filter.binary_detection')}
            </label>
            <div className="flex items-center gap-3">
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={config.binary_detection_enabled}
                  onChange={e => setConfig({ ...config, binary_detection_enabled: e.target.checked })}
                  className="sr-only peer"
                />
                <div className="w-11 h-6 bg-gray-600 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
              </label>
              <span className="text-sm text-text-main">
                {t('file_filter.binary_detection_label')}
              </span>
            </div>
            <p className="text-xs text-text-dim">
              {t('file_filter.binary_detection_hint')}
            </p>
          </div>

          {/* 第2层：智能过滤开关 */}
          <div className="space-y-2">
            <label className="text-xs text-text-dim uppercase font-bold block">
              {t('file_filter.smart_filter')}
            </label>
            <div className="flex items-center gap-3">
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={config.enabled}
                  onChange={e => setConfig({ ...config, enabled: e.target.checked })}
                  className="sr-only peer"
                />
                <div className="w-11 h-6 bg-gray-600 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
              </label>
              <span className="text-sm text-text-main">
                {t('file_filter.smart_filter_label')}
              </span>
            </div>
            <p className="text-xs text-text-dim">
              {t('file_filter.smart_filter_hint')}
            </p>
          </div>

          {/* 过滤模式选择 */}
          {config.enabled && (
            <div className="space-y-2">
              <label className="text-xs text-text-dim uppercase font-bold block">
                {t('file_filter.filter_mode')}
              </label>
              <div className="flex gap-4">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="filterMode"
                    value={FilterMode.Whitelist}
                    checked={config.mode === FilterMode.Whitelist}
                    onChange={() => setConfig({ ...config, mode: FilterMode.Whitelist })}
                    className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 focus:ring-blue-600"
                  />
                  <span className="text-sm text-text-main">{t('file_filter.whitelist_mode')}</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="filterMode"
                    value={FilterMode.Blacklist}
                    checked={config.mode === FilterMode.Blacklist}
                    onChange={() => setConfig({ ...config, mode: FilterMode.Blacklist })}
                    className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 focus:ring-blue-600"
                  />
                  <span className="text-sm text-text-main">{t('file_filter.blacklist_mode')}</span>
                </label>
              </div>
              <p className="text-xs text-text-dim">
                {config.mode === FilterMode.Whitelist
                  ? t('file_filter.whitelist_description')
                  : t('file_filter.blacklist_description')}
              </p>
            </div>
          )}

          {/* 文件名模式列表 */}
          {config.enabled && (
            <div className="space-y-2">
              <label className="text-xs text-text-dim uppercase font-bold block">
                {t('file_filter.glob_patterns')}
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newPattern}
                  onChange={e => setNewPattern(e.target.value)}
                  onKeyPress={e => e.key === 'Enter' && addPattern()}
                  placeholder={t('file_filter.pattern_placeholder')}
                  className="flex-1 px-3 py-2 bg-bg-sidebar border border-border-base rounded text-sm text-text-main placeholder:text-text-dim focus:outline-none focus:border-primary focus:ring-2 focus:ring-primary/20"
                />
                <Button onClick={addPattern} variant="icon" icon={Plus} aria-label={t('file_filter.add_pattern')} />
              </div>
              <div className="text-xs text-text-dim">
                {t('file_filter.glob_hint')}
              </div>
              {config.filename_patterns.length > 0 && (
                <div className="flex flex-wrap gap-2 mt-2">
                  {config.filename_patterns.map(pattern => (
                    <div
                      key={pattern}
                      className="flex items-center gap-1 px-2 py-1 bg-bg-sidebar border border-border-base rounded text-xs text-text-main"
                    >
                      <code className="text-log-info">{pattern}</code>
                      <button
                        onClick={() => removePattern(pattern)}
                        className="ml-1 text-text-dim hover:text-log-error transition-colors"
                        aria-label={t('file_filter.remove_pattern', { pattern })}
                      >
                        <Trash2 className="w-3 h-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* 扩展名列表 */}
          {config.enabled && config.mode === FilterMode.Whitelist && (
            <div className="space-y-2">
              <label className="text-xs text-text-dim uppercase font-bold block">
                {t('file_filter.extension_whitelist')}
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newExtension}
                  onChange={e => setNewExtension(e.target.value)}
                  onKeyPress={e => e.key === 'Enter' && addExtension()}
                  placeholder={t('file_filter.extension_placeholder')}
                  className="flex-1 px-3 py-2 bg-bg-sidebar border border-border-base rounded text-sm text-text-main placeholder:text-text-dim focus:outline-none focus:border-primary focus:ring-2 focus:ring-primary/20"
                />
                <Button onClick={addExtension} variant="icon" icon={Plus} aria-label={t('file_filter.add_extension')} />
              </div>
              {config.allowed_extensions.length > 0 && (
                <div className="flex flex-wrap gap-2 mt-2">
                  {config.allowed_extensions.map(ext => (
                    <div
                      key={ext}
                      className="flex items-center gap-1 px-2 py-1 bg-bg-sidebar border border-border-base rounded text-xs text-text-main"
                    >
                      <code className="text-green-400">.{ext}</code>
                      <button
                        onClick={() => removeExtension(ext)}
                        className="ml-1 text-text-dim hover:text-log-error transition-colors"
                        aria-label={t('file_filter.remove_extension', { extension: ext })}
                      >
                        <Trash2 className="w-3 h-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* 底部按钮 */}
        <div className="px-6 py-4 border-t border-border-base flex justify-end gap-3 bg-bg-sidebar">
          <Button onClick={onClose} variant="secondary">
            {t('file_filter.cancel')}
          </Button>
          <Button onClick={handleSave} disabled={isLoading}>
            {isLoading ? t('file_filter.save_saving') : t('file_filter.save')}
          </Button>
        </div>
      </div>
    </div>
  );
};

export default FileFilterSettings;
