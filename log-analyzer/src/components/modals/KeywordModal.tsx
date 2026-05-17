import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Plus, Trash2, AlertCircle } from 'lucide-react';
import { Button, Input } from '../ui';
import { cn } from '../../utils/classNames';
import type { KeywordModalProps } from '../../types/ui';
import type { KeywordPattern, ColorKey } from '../../types/common';
import { COLOR_STYLES } from '../../constants/colors';
import {
  validateKeywordGroup,
  formatValidationErrors,
  validateRegexPattern,
  type KeywordGroupFormData,
} from '../../schemas';

// FIX(HI-25): Internal pattern type with unique id for stable keys
interface PatternItem extends KeywordPattern {
  id: string;
}

function generatePatternId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return Date.now().toString(36) + Math.random().toString(36).slice(2);
}

function patternsToItems(patterns: KeywordPattern[]): PatternItem[] {
  return patterns.map(p => ({ ...p, id: generatePatternId() }));
}

function itemsToPatterns(items: PatternItem[]): KeywordPattern[] {
  return items.map(({ id: _id, ...p }) => p);
}

/**
 * 关键词配置模态框组件
 * 用于新建和编辑关键词组
 * 使用 Zod 进行类型安全的表单验证
 */
const KeywordModal: React.FC<KeywordModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
  const { t } = useTranslation();
  const [name, setName] = useState(initialData?.name || "");
  const [color, setColor] = useState<ColorKey>(initialData?.color || "blue");
  const [patterns, setPatterns] = useState<PatternItem[]>(() =>
    patternsToItems(initialData?.patterns || [{ regex: "", comment: "" }])
  );

  // 验证状态
  const [errors, setErrors] = useState<{ name?: string; patterns?: string[] }>({});
  const [touched, setTouched] = useState<{ name?: boolean; patterns?: boolean }>({});
  // 每个 pattern 的实时正则语法错误
  const [regexErrors, setRegexErrors] = useState<(string | undefined)[]>([]);

  // FIX(HI-21): Refs for accessibility
  const modalRef = useRef<HTMLDivElement>(null);
  const firstInputRef = useRef<HTMLInputElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  // 当模态框打开或初始数据变化时，重置表单
  useEffect(() => {
    if (isOpen) {
      setName(initialData?.name || "");
      setColor(initialData?.color || "blue");
      setPatterns(patternsToItems(initialData?.patterns || [{ regex: "", comment: "" }]));
      setErrors({});
      setTouched({});
      setRegexErrors([]);
    }
  }, [isOpen, initialData]);

  // FIX(HI-21): Escape key to close
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

  // FIX(HI-21): Focus trap, auto-focus, and focus restoration
  useEffect(() => {
    if (!isOpen) return;

    // Save previous focus
    previousFocusRef.current = document.activeElement as HTMLElement;

    // Auto-focus first input (Group Name) after a short delay for animation
    const timer = setTimeout(() => {
      firstInputRef.current?.focus();
    }, 50);

    // Focus trap
    const handleTab = (e: KeyboardEvent) => {
      if (e.key !== 'Tab' || !modalRef.current) return;

      const focusableElements = modalRef.current.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusableElements.length === 0) return;

      const first = focusableElements[0];
      const last = focusableElements[focusableElements.length - 1];

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    document.addEventListener('keydown', handleTab);

    return () => {
      clearTimeout(timer);
      document.removeEventListener('keydown', handleTab);
      // Restore focus on close/unmount
      previousFocusRef.current?.focus();
    };
  }, [isOpen]);

  /**
   * 验证表单
   * 使用 Zod schema 进行类型安全的验证
   */
  const validateForm = useCallback((): boolean => {
    const formData: KeywordGroupFormData = {
      name,
      color,
      patterns: itemsToPatterns(patterns),
    };

    const result = validateKeywordGroup(formData);

    if (!result.success) {
      const formattedErrors = formatValidationErrors(result);
      setErrors(formattedErrors);
      return false;
    }

    setErrors({});
    return true;
  }, [name, color, patterns]);

  /**
   * 处理名称变更
   * 使用防抖优化：仅在失焦或停止输入后验证
   */
  const handleNameChange = useCallback((value: string) => {
    setName(value);
    // 仅在已触摸状态下验证
    if (touched.name) {
      validateForm();
    }
  }, [touched.name, validateForm]);

  /**
   * 处理名称失焦事件
   * 在失焦时触发验证，提供更好的用户体验
   */
  const handleNameBlur = useCallback(() => {
    setTouched(prev => ({ ...prev, name: true }));
    validateForm();
  }, [validateForm]);

  /**
   * 处理模式变更
   */
  const handlePatternChange = useCallback((id: string, field: 'regex' | 'comment', value: string) => {
    setPatterns(prev => {
      const idx = prev.findIndex(p => p.id === id);
      if (idx === -1) return prev;
      const next = [...prev];
      next[idx] = { ...next[idx], [field]: value };
      return next;
    });

    // regex 字段：实时验证正则语法
    if (field === 'regex') {
      const result = validateRegexPattern(value);
      setRegexErrors(prev => {
        const idx = patterns.findIndex(p => p.id === id);
        if (idx === -1) return prev;
        const next = [...prev];
        next[idx] = result.valid ? undefined : result.error;
        return next;
      });
    }

    // 仅在已触摸状态下验证
    if (touched.patterns) {
      validateForm();
    }
  }, [patterns, touched.patterns, validateForm]);

  /**
   * 处理模式失焦事件
   */
  const handlePatternBlur = useCallback(() => {
    setTouched(prev => ({ ...prev, patterns: true }));
    validateForm();
  }, [validateForm]);

  /**
   * 处理保存操作
   */
  const handleSave = useCallback(() => {
    // 标记所有字段为已触摸，确保显示所有验证错误
    setTouched({ name: true, patterns: true });

    if (!validateForm()) {
      return;
    }

    const validPatterns = itemsToPatterns(patterns.filter(p => p.regex.trim() !== ""));
    onSave({
      id: initialData?.id || Date.now().toString(),
      name: name.trim(),
      color,
      patterns: validPatterns,
      enabled: initialData?.enabled ?? true
    });
    onClose();
  }, [name, color, patterns, initialData, onSave, onClose, validateForm]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      {/* FIX(HI-21): ref for focus trap */}
      <div
        ref={modalRef}
        className="w-[600px] bg-bg-card border border-border-base rounded-lg shadow-2xl flex flex-col max-h-[85vh] animate-in fade-in zoom-in-95 duration-200"
        onClick={e => e.stopPropagation()}
      >
        {/* 标题栏 */}
        <div className="px-6 py-4 border-b border-border-base flex justify-between items-center bg-bg-sidebar">
          <h2 className="text-lg font-bold text-text-main">
            {/* FIX(HI-28): translate title */}
            {initialData ? t('keywords.modal.title_edit') : t('keywords.modal.title_new')}
          </h2>
          <Button variant="icon" icon={X} onClick={onClose} />
        </div>

        {/* 表单内容 */}
        <div className="p-6 overflow-y-auto flex-1 space-y-6">
          {/* 组名和颜色选择 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-xs text-text-dim uppercase font-bold mb-1.5 block">
                {/* FIX(HI-28): translate label */}
                {t('keywords.modal.group_name')}
              </label>
              <Input
                ref={firstInputRef}
                value={name}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => handleNameChange(e.target.value)}
                onBlur={handleNameBlur}
                placeholder={t('keywords.modal.name_placeholder')}
                className={cn(errors.name ? "border-red-500 focus:ring-red-500/50" : "")}
              />
              {errors.name && touched.name && (
                <div className="flex items-center gap-1 mt-1 text-red-500 text-xs">
                  <AlertCircle size={12} />
                  <span>{errors.name}</span>
                </div>
              )}
            </div>
            <div>
              <label className="text-xs text-text-dim uppercase font-bold mb-1.5 block">
                {/* FIX(HI-28): translate label */}
                {t('keywords.modal.highlight_color')}
              </label>
              <div className="flex gap-2 h-9 items-center">
                {(['blue', 'green', 'orange', 'red', 'purple'] as ColorKey[]).map((c) => (
                  <button
                    key={c}
                    type="button"
                    onClick={() => setColor(c)}
                    className={cn(
                      "w-6 h-6 rounded-full border-2 transition-all cursor-pointer",
                      COLOR_STYLES[c].dot,
                      color === c
                        ? "border-white scale-110 shadow-lg"
                        : "border-transparent opacity-40 hover:opacity-100"
                    )}
                    aria-label={`Select ${c} color`}
                  />
                ))}
              </div>
            </div>
          </div>

          {/* 模式和注释列表 */}
          <div>
            <div className="flex justify-between items-center mb-2">
              <label className="text-xs text-text-dim uppercase font-bold">
                {/* FIX(HI-28): translate label */}
                {t('keywords.modal.patterns_title')}
              </label>
              <Button
                variant="ghost"
                className="h-6 text-xs"
                icon={Plus}
                onClick={() => {
                  setPatterns(prev => [...prev, { id: generatePatternId(), regex: "", comment: "" }]);
                  setRegexErrors(prev => [...prev, undefined]);
                }}
              >
                {/* FIX(HI-28): translate button */}
                {t('keywords.modal.add_pattern')}
              </Button>
            </div>
            {errors.patterns && touched.patterns && errors.patterns.length > 0 && (
              <div className="mb-2 p-2 bg-red-500/10 border border-red-500/50 rounded">
                {errors.patterns.map((error, index) => (
                  <div key={index} className="flex items-center gap-1 text-red-500 text-xs">
                    <AlertCircle size={12} />
                    <span>{error}</span>
                  </div>
                ))}
              </div>
            )}
            <div className="space-y-2">
              {/* FIX(HI-25): use stable id as key instead of index */}
              {patterns.map((p) => (
                <div key={p.id} className="flex gap-2 items-center group">
                  <div className="flex-1">
                    <Input
                      value={p.regex}
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) => handlePatternChange(p.id, 'regex', e.target.value)}
                      onBlur={handlePatternBlur}
                      placeholder={t('keywords.modal.regex_placeholder')}
                      className={cn(
                        "font-mono text-xs",
                        (errors.patterns?.[patterns.indexOf(p)] || regexErrors[patterns.indexOf(p)]) ? "border-red-500 focus:ring-red-500/50" : ""
                      )}
                    />
                    {regexErrors[patterns.indexOf(p)] && (
                      <p className="mt-1 text-xs text-red-500">{regexErrors[patterns.indexOf(p)]}</p>
                    )}
                  </div>
                  <div className="flex-1">
                    <Input
                      value={p.comment}
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                        handlePatternChange(p.id, 'comment', e.target.value);
                      }}
                      placeholder={t('keywords.modal.comment_placeholder')}
                      className="text-xs"
                    />
                  </div>
                  <Button
                    variant="icon"
                    icon={Trash2}
                    className="text-log-error opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => {
                      const idx = patterns.findIndex(item => item.id === p.id);
                      setPatterns(patterns.filter((_, i) => i !== idx));
                      setRegexErrors(regexErrors.filter((_, i) => i !== idx));
                    }}
                  />
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* 底部按钮 */}
        <div className="px-6 py-4 border-t border-border-base bg-bg-sidebar flex justify-end gap-3">
          <Button variant="secondary" onClick={onClose}>
            {/* FIX(HI-28): translate button */}
            {t('keywords.modal.cancel')}
          </Button>
          <Button onClick={handleSave}>
            {/* FIX(HI-28): translate button */}
            {t('keywords.modal.save')}
          </Button>
        </div>
      </div>
    </div>
  );
};

export default KeywordModal;
