import React from 'react';
import { CheckCircle2, AlertCircle, RefreshCw, Trash2, X } from 'lucide-react';
import { useTaskManager } from '../hooks/useTaskManager';
import { Button } from '../components/ui';
import { cn } from '../utils/classNames';
import type { Task } from '../types/common';

/**
 * 后台任务页面
 * 核心功能:
 * 1. 显示后台任务列表
 * 2. 实时更新任务进度
 * 3. 任务状态展示(运行中、完成、失败)
 * 4. 删除已完成或失败的任务
 * 5. 取消运行中的任务
 */
const TasksPage: React.FC = () => {
  const { tasks, deleteTask, cancelTask } = useTaskManager();

  const handleDelete = (id: string) => {
    deleteTask(id);
  };

  const handleCancel = async (id: string) => {
    cancelTask(id);
  };

  return (
    <div className="p-8 max-w-4xl mx-auto h-full overflow-auto">
      <h1 className="text-2xl font-bold mb-6 text-text-main">后台任务</h1>
      <div className="space-y-4">
        {tasks.length === 0 && <div className="text-text-dim text-center py-10">暂无活动任务</div>}
        {tasks.map((t: Task, index: number) => {
          // 诊断日志：检查任务ID和索引
          if (!t.id || t.id === '') {
            console.warn('[TasksPage] Task with empty ID found:', t, 'at index:', index);
          }
          // 检查是否有重复的ID
          const duplicateCount = tasks.filter(task => task.id === t.id).length;
          if (duplicateCount > 1) {
            console.warn('[TasksPage] Duplicate task ID found:', t.id, 'count:', duplicateCount);
          }

          return (
          <div key={t.id || `task-${index}`} className="p-4 bg-bg-card border border-border-base rounded-lg flex items-center gap-4 animate-in fade-in slide-in-from-bottom-2">
            <div className={cn("p-2 rounded-full bg-bg-hover", t.status === 'RUNNING' ? "text-blue-500" : t.status === 'FAILED' ? "text-red-500" : "text-emerald-500")}>
              {t.status === 'RUNNING' ? <RefreshCw size={20} className="animate-spin"/> : t.status === 'FAILED' ? <AlertCircle size={20}/> : <CheckCircle2 size={20}/>}
            </div>
            <div className="flex-1 min-w-0">
               <div className="flex justify-between mb-1"><h3 className="font-semibold text-sm text-text-main truncate">{t.type}: {t.target}</h3><span className="text-xs font-mono text-text-dim font-bold">{t.status}</span></div>
               <div className="w-full bg-bg-main h-2 rounded-full overflow-hidden relative">
                  <div className={cn("h-full transition-all duration-500", t.status==='FAILED'?'bg-red-500':t.status==='COMPLETED'?'bg-emerald-500':'bg-blue-500')} style={{width: `${t.progress || 5}%`}}></div>
               </div>
               <div className="flex justify-between mt-1 text-xs text-text-dim">
                  <span className="truncate max-w-[300px]">{t.message}</span>
                  <span>{t.progress}%</span>
               </div>
            </div>
            <div className="flex gap-2">
               {/* 只为运行中的任务添加取消按钮 */}
               {t.status === 'RUNNING' && (
                  <Button
                    variant="ghost"
                    className="h-8 px-2 text-amber-400 hover:text-amber-300"
                    onClick={() => handleCancel(t.id)}
                    data-testid={`cancel-task-${t.id}`}
                  >
                    <X size={14} />
                  </Button>
               )}
               <Button
                 variant="ghost"
                 className="h-8 w-8 p-0 text-red-400 hover:text-red-300"
                 onClick={() => handleDelete(t.id)}
                 data-testid={`delete-task-${t.id}`}
               >
                 <Trash2 size={16}/>
               </Button>
            </div>
          </div>
          );
        })}
      </div>
    </div>
  );
};

export default TasksPage;
