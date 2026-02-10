import React, { useState, useEffect, useCallback } from 'react';
import { X, Plus, Trash2, AlertCircle } from 'lucide-react';
import { Button, Input } from '../ui';
import { cn } from '../../utils/classNames';
import type { KeywordModalProps } from '../../types/ui';
import type { KeywordPattern, ColorKey } from '../../types/common';
import { COLOR_STYLES } from '../../constants/colors';
import {
  validateKeywordGroup,
  formatValidationErrors,
  type KeywordGroupFormData,
} from '../../schemas';

/**
 * 关键词配置模态框组件
 * 用于新建和编辑关键词组
 * 使用 Zod 进行类型安全的表单验证
 */
const KeywordModal: React.FC<KeywordModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
  const [name, setName] = useState(initialData?.name || "");
  const [color, setColor] = useState<ColorKey>(initialData?.color || "blue");
  const [patterns, setPatterns] = useState<KeywordPattern[]>(initialData?.patterns || [{ regex: "", comment: "" }]);

  // 验证状态
  const [errors, setErrors] = useState<{ name?: string; patterns?: string[] }>({});
  const [touched, setTouched] = useState<{ name?: boolean; patterns?: boolean }>({});

  // 当模态框打开或初始数据变化时，重置表单
  useEffect(() => {
    if (isOpen) {
      setName(initialData?.name || "");
      setColor(initialData?.color || "blue");
      setPatterns(initialData?.patterns || [{ regex: "", comment: "" }]);
      setErrors({});
      setTouched({});
    }
  }, [isOpen, initialData]);

  /**
   * 验证表单
   * 使用 Zod schema 进行类型安全的验证
   */
  const validateForm = useCallback((): boolean => {
    const formData: KeywordGroupFormData = {
      name,
      color,
      patterns,
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
  const handlePatternChange = useCallback((index: number, field: 'regex' | 'comment', value: string) => {
    const newPatterns = [...patterns];
    newPatterns[index][field] = value;
    setPatterns(newPatterns);

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

    const validPatterns = patterns.filter(p => p.regex.trim() !== "");
    onSave({
      id: initialData?.id || Date.now().toString(),
      name: name.trim(),
      color,
      patterns: validPatterns,
      enabled: true
    });
    onClose();
  }, [name, color, patterns, initialData, onSave, onClose, validateForm]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="w-[600px] bg-bg-card border border-border-base rounded-lg shadow-2xl flex flex-col max-h-[85vh] animate-in fade-in zoom-in-95 duration-200"
        onClick={e => e.stopPropagation()}
      >
        {/* 标题栏 */}
        <div className="px-6 py-4 border-b border-border-base flex justify-between items-center bg-bg-sidebar">
          <h2 className="text-lg font-bold text-text-main">
            {initialData ? 'Edit Keyword Group' : 'New Keyword Group'}
          </h2>
          <Button variant="icon" icon={X} onClick={onClose} />
        </div>

        {/* 表单内容 */}
        <div className="p-6 overflow-y-auto flex-1 space-y-6">
          {/* 组名和颜色选择 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-xs text-text-dim uppercase font-bold mb-1.5 block">
                Group Name
              </label>
              <Input
                value={name}
                onChange={(e: any) => handleNameChange(e.target.value)}
                onBlur={handleNameBlur}
                placeholder="Name"
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
                Highlight Color
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
                Patterns & Comments
              </label>
              <Button
                variant="ghost"
                className="h-6 text-xs"
                icon={Plus}
                onClick={() => setPatterns([...patterns, { regex: "", comment: "" }])}
              >
                Add
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
              {patterns.map((p, i) => (
                <div key={i} className="flex gap-2 items-center group">
                  <div className="flex-1">
                    <Input
                      value={p.regex}
                      onChange={(e: any) => handlePatternChange(i, 'regex', e.target.value)}
                      onBlur={handlePatternBlur}
                      placeholder="RegEx"
                      className={cn(
                        "font-mono text-xs",
                        errors.patterns?.[i] ? "border-red-500 focus:ring-red-500/50" : ""
                      )}
                    />
                  </div>
                  <div className="flex-1">
                    <Input
                      value={p.comment}
                      onChange={(e: any) => {
                        const n = [...patterns];
                        n[i].comment = e.target.value;
                        setPatterns(n);
                      }}
                      placeholder="Comment"
                      className="text-xs"
                    />
                  </div>
                  <Button
                    variant="icon"
                    icon={Trash2}
                    className="text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => setPatterns(patterns.filter((_, idx) => idx !== i))}
                  />
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* 底部按钮 */}
        <div className="px-6 py-4 border-t border-border-base bg-bg-sidebar flex justify-end gap-3">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave}>
            Save Configuration
          </Button>
        </div>
      </div>
    </div>
  );
};

export default KeywordModal;
