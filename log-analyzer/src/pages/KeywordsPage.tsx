import React, { useState } from "react";
import { Plus, Edit2, Trash2, Tag, Power, PowerOff } from "lucide-react";
import { useKeywordManager } from "../hooks/useKeywordManager";
import { Button, Card, EmptyState } from "../components/ui";
import { KeywordModal } from "../components/modals";
import { COLOR_STYLES } from "../constants/colors";
import { cn } from "../utils/classNames";
import type { KeywordGroup } from "../types/common";

/**
 * 关键词配置页面
 * 核心功能:
 * 1. 显示所有关键词组列表
 * 2. 新建关键词组
 * 3. 编辑关键词组
 * 4. 删除关键词组
 */
const KeywordsPage: React.FC = () => {
  const {
    saveKeywordGroup,
    deleteKeywordGroup,
    toggleKeywordGroup,
    keywordGroups,
  } = useKeywordManager();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingGroup, setEditingGroup] = useState<KeywordGroup | null>(null);
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const selectedGroup =
    keywordGroups.find((group) => group.id === selectedGroupId) ??
    keywordGroups[0];

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
    <div className="mx-auto h-full max-w-6xl overflow-auto px-8 py-7">
      {/* 页面标题和操作 */}
      <div className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-[28px] font-semibold leading-tight text-text-main tracking-[-0.02em]">
            Keyword Groups
          </h1>
          <p className="mt-1 text-sm text-text-muted">
            Organize reusable rules for highlighting and filtering logs.
          </p>
        </div>
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

      {/* 关键词组列表或空状态 */}
      {keywordGroups.length === 0 ? (
        <EmptyState
          icon={Tag}
          title="还没有关键词组"
          description="创建关键词组来高亮和过滤日志中的重要内容"
          action={{
            label: "New Group",
            onClick: () => {
              setEditingGroup(null);
              setIsModalOpen(true);
            },
            icon: Plus,
            variant: "cta",
          }}
        />
      ) : (
        <div className="grid min-h-[440px] grid-cols-[260px_1fr] overflow-hidden rounded-[14px] border border-border-base bg-bg-card shadow-card">
          <div
            className="border-r border-border-base bg-bg-sidebar/40 p-2"
            aria-label="Keyword groups"
          >
            {keywordGroups.map((group: KeywordGroup) => (
              <div
                key={group.id}
                className={cn(
                  "flex items-center rounded-[10px] pr-1",
                  selectedGroup?.id === group.id
                    ? "bg-primary/12"
                    : "hover:bg-bg-hover"
                )}
              >
                <button
                  type="button"
                  onClick={() => setSelectedGroupId(group.id)}
                  className={cn(
                    "ui-pressable flex min-w-0 flex-1 items-center gap-3 rounded-[10px] px-3 py-2.5 text-left",
                    selectedGroup?.id === group.id
                      ? "text-text-main"
                      : "text-text-muted"
                  )}
                >
                  <span
                    className={cn(
                      "h-2.5 w-2.5 rounded-full",
                      COLOR_STYLES[group.color].dot
                    )}
                  />
                  <span className="min-w-0 flex-1 truncate text-sm font-medium">
                    {group.name}
                  </span>
                  <span className="text-xs tabular-nums text-text-dim">
                    {group.patterns.length}
                  </span>
                </button>
                <button
                  type="button"
                  aria-label={`toggle ${group.name}`}
                  aria-pressed={group.enabled}
                  onClick={() => toggleKeywordGroup(group.id)}
                  className={cn(
                    "ui-pressable rounded-full p-1.5",
                    group.enabled ? "text-status-success" : "text-text-dim"
                  )}
                >
                  {group.enabled ? <Power size={14} /> : <PowerOff size={14} />}
                </button>
              </div>
            ))}
          </div>

          {selectedGroup && (
            <Card variant="ghost" padding="none" className="rounded-none">
              <div className="flex items-center justify-between border-b border-border-base px-6 py-4">
                <div>
                  <h2 className="text-lg font-semibold text-text-main">
                    {selectedGroup.name}
                  </h2>
                  <p className="mt-0.5 text-xs text-text-muted">
                    {selectedGroup.patterns.length} matching rules
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="secondary"
                    icon={Edit2}
                    onClick={() => {
                      setEditingGroup(selectedGroup);
                      setIsModalOpen(true);
                    }}
                  >
                    Edit
                  </Button>
                  <Button
                    variant="danger"
                    icon={Trash2}
                    onClick={() => handleDelete(selectedGroup.id)}
                  >
                    Delete
                  </Button>
                </div>
              </div>
              <div className="divide-y divide-border-subtle px-6">
                {selectedGroup.patterns.map((pattern, index) => (
                  <div
                    key={`${selectedGroup.id}-${pattern.regex}-${index}`}
                    className="flex items-start gap-4 py-3"
                  >
                    <span className="w-8 shrink-0 pt-0.5 text-xs tabular-nums text-text-dim">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <code className="min-w-0 flex-1 break-all font-mono text-sm text-text-main">
                      {pattern.regex}
                    </code>
                    {pattern.comment && (
                      <span
                        className={cn(
                          "rounded-full px-2 py-0.5 text-[10px]",
                          COLOR_STYLES[selectedGroup.color].badge
                        )}
                      >
                        {pattern.comment}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </Card>
          )}
        </div>
      )}

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
