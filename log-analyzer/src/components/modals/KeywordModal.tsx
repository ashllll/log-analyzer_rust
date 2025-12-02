import React, { useState, useEffect } from 'react';
import { X, Plus, Trash2 } from 'lucide-react';
import { Button, Input } from '../ui';
import { cn } from '../../utils/classNames';
import type { KeywordModalProps } from '../../types/ui';
import type { KeywordGroup, KeywordPattern, ColorKey } from '../../types/common';
import { COLOR_STYLES } from '../../constants/colors';

/**
 * 关键词配置模态框组件
 * 用于新建和编辑关键词组
 */
const KeywordModal: React.FC<KeywordModalProps> = ({ isOpen, onClose, onSave, initialData }) => {
  const [name, setName] = useState(initialData?.name || "");
  const [color, setColor] = useState<ColorKey>(initialData?.color || "blue");
  const [patterns, setPatterns] = useState<KeywordPattern[]>(initialData?.patterns || [{ regex: "", comment: "" }]);

  // 当模态框打开或初始数据变化时，重置表单
  useEffect(() => {
    if (isOpen) {
      setName(initialData?.name || "");
      setColor(initialData?.color || "blue");
      setPatterns(initialData?.patterns || [{ regex: "", comment: "" }]);
    }
  }, [isOpen, initialData]);

  const handleSave = () => {
    const validPatterns = patterns.filter(p => p.regex.trim() !== "");
    if (!name || validPatterns.length === 0) return;

    onSave({
      id: initialData?.id || Date.now().toString(),
      name,
      color,
      patterns: validPatterns,
      enabled: true
    });
    onClose();
  };

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
                onChange={(e: any) => setName(e.target.value)} 
                placeholder="Name" 
              />
            </div>
            <div>
              <label className="text-xs text-text-dim uppercase font-bold mb-1.5 block">
                Highlight Color
              </label>
              <div className="flex gap-2 h-9 items-center">
                {(['blue', 'green', 'orange', 'red', 'purple'] as ColorKey[]).map((c) => (
                  <button 
                    key={c} 
                    onClick={() => setColor(c)} 
                    className={cn(
                      "w-6 h-6 rounded-full border-2 transition-all cursor-pointer", 
                      COLOR_STYLES[c].dot, 
                      color === c 
                        ? "border-white scale-110 shadow-lg" 
                        : "border-transparent opacity-40 hover:opacity-100"
                    )} 
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
            <div className="space-y-2">
              {patterns.map((p, i) => (
                <div key={i} className="flex gap-2 items-center group">
                  <div className="flex-1">
                    <Input 
                      value={p.regex} 
                      onChange={(e: any) => { 
                        const n = [...patterns]; 
                        n[i].regex = e.target.value; 
                        setPatterns(n); 
                      }} 
                      placeholder="RegEx" 
                      className="font-mono text-xs"
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
