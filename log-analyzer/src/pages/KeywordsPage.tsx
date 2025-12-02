import React, { useState } from 'react';
import { Plus, Edit2, Trash2 } from 'lucide-react';
import { useKeywordManager } from '../hooks/useKeywordManager';
import { Button, Card } from '../components/ui';
import { KeywordModal } from '../components/modals';
import { COLOR_STYLES } from '../constants/colors';
import { cn } from '../utils/classNames';
import type { KeywordGroup } from '../types/common';

/**
 * 关键词配置页面
 * 核心功能:
 * 1. 显示所有关键词组列表
 * 2. 新建关键词组
 * 3. 编辑关键词组
 * 4. 删除关键词组
 */
const KeywordsPage: React.FC = () => {
  const { saveKeywordGroup, deleteKeywordGroup, keywordGroups } = useKeywordManager();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingGroup, setEditingGroup] = useState<KeywordGroup | null>(null);
  
  /**
   * 保存关键词组（新建或编辑）
   */
  const handleSave = (group: KeywordGroup) => {
    const isEditing = !!editingGroup;
    saveKeywordGroup(group, isEditing);
  };
  
  /**
   * 删除关键词组
   */
  const handleDelete = (id: string) => { 
    deleteKeywordGroup(id);
  };

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      {/* 页面标题和操作 */}
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-text-main">Keyword Configuration</h1>
        <Button 
          icon={Plus} 
          onClick={() => { 
            setEditingGroup(null); 
            setIsModalOpen(true); 
          }}
        >
          New Group
        </Button>
      </div>
      
      {/* 关键词组列表 */}
      <div className="space-y-4">
        {keywordGroups.map((group: KeywordGroup) => (
          <Card 
            key={group.id} 
            className="overflow-hidden hover:border-primary/50 transition-colors"
          >
            {/* 关键词组头部 */}
            <div className="px-6 py-4 flex items-center justify-between bg-bg-sidebar/30 border-b border-border-base/50">
              <div className="flex items-center gap-4">
                <div className={cn(
                  "w-3 h-3 rounded-full shadow-[0_0_8px_currentColor]", 
                  COLOR_STYLES[group.color].dot
                )}></div>
                <div>
                  <h3 className="text-sm font-bold text-text-main">{group.name}</h3>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Button 
                  variant="ghost" 
                  icon={Edit2} 
                  onClick={() => { 
                    setEditingGroup(group); 
                    setIsModalOpen(true); 
                  }}
                >
                  Edit
                </Button>
                <Button 
                  variant="danger" 
                  icon={Trash2} 
                  onClick={() => handleDelete(group.id)}
                >
                  Delete
                </Button>
              </div>
            </div>
            
            {/* 关键词模式列表 */}
            <div className="px-6 py-3 bg-bg-card flex flex-wrap gap-2">
              {group.patterns.map((p, i) => (
                <div 
                  key={i} 
                  className="flex items-center bg-bg-main border border-border-base rounded px-2 py-1 text-xs"
                >
                  <span className="font-mono text-text-main mr-2">{p.regex}</span>
                  {p.comment && (
                    <span className={cn(
                      "text-[10px] px-1.5 rounded", 
                      COLOR_STYLES[group.color].badge
                    )}>
                      {p.comment}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </Card>
        ))}
      </div>
      
      {/* 关键词编辑模态框 */}
      <KeywordModal 
        isOpen={isModalOpen} 
        onClose={() => setIsModalOpen(false)} 
        initialData={editingGroup} 
        onSave={handleSave} 
      />
    </div>
  );
};

export default KeywordsPage;
