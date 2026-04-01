import React from 'react';
import { useTranslation } from 'react-i18next';
import { CheckCircle2, AlertCircle, RefreshCw, Trash2, X, ListTodo } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTaskManager } from '../hooks/useTaskManager';
import { Button, EmptyState } from '../components/ui';
import { cn } from '../utils/classNames';
import type { Task } from '../types/common';

const containerVariants = {
  hidden: {},
  visible: { transition: { staggerChildren: 0.06, delayChildren: 0.04 } },
};

const itemVariants = {
  hidden: { opacity: 0, x: -8 },
  visible: { opacity: 1, x: 0, transition: { duration: 0.18, ease: 'easeOut' as const } },
};

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
  const { t } = useTranslation();
  const { tasks, deleteTask, cancelTask } = useTaskManager();

  const handleDelete = (id: string) => {
    deleteTask(id);
  };

  const handleCancel = async (id: string) => {
    cancelTask(id);
  };

  const validTasks = tasks.filter((task: Task) => Boolean(task.id));

  return (
    <div className="p-8 max-w-4xl mx-auto h-full overflow-auto">
      <h1 className="text-2xl font-bold mb-6 text-text-main tracking-tight">{t('tasks.title')}</h1>

      {validTasks.length === 0 ? (
        <EmptyState
          icon={ListTodo}
          title={t('tasks.no_tasks', '暂无后台任务')}
          description="导入工作区或执行搜索时，任务会出现在这里"
        />
      ) : (
        <motion.div
          className="space-y-4"
          variants={containerVariants}
          initial="hidden"
          animate="visible"
        >
          {validTasks.map((task: Task) => (
            <motion.div
              key={task.id}
              variants={itemVariants}
              className="p-4 bg-bg-card border border-border-base rounded-lg flex items-center gap-4"
            >
              <div className={cn(
                "p-2 rounded-full bg-bg-hover",
                task.status === 'RUNNING' ? "text-primary-text" :
                task.status === 'FAILED' ? "text-status-error" : "text-cta"
              )}>
                {task.status === 'RUNNING' ? (
                  <RefreshCw size={20} className="animate-spin"/>
                ) : task.status === 'FAILED' ? (
                  <AlertCircle size={20}/>
                ) : (
                  <CheckCircle2 size={20}/>
                )}
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex justify-between mb-1">
                  <h3 className="font-semibold text-sm text-text-main truncate">{task.type}: {task.target}</h3>
                  <span className="text-xs font-mono text-text-dim font-bold">{task.status}</span>
                </div>
                <div className="w-full bg-bg-main h-2 rounded-full overflow-hidden relative">
                  <div
                    className={cn(
                      "h-full transition-all duration-500",
                      task.status === 'FAILED' ? 'bg-status-error' :
                      task.status === 'COMPLETED' ? 'bg-cta' : 'bg-primary'
                    )}
                    style={{ width: `${task.progress || 5}%` }}
                  />
                </div>
                <div className="flex justify-between mt-1 text-xs text-text-dim">
                  <span className="truncate max-w-[300px]">{task.message}</span>
                  <span>{task.progress}%</span>
                </div>
              </div>
              <div className="flex gap-2">
                {task.status === 'RUNNING' && (
                  <Button
                    variant="ghost"
                    className="h-8 px-2 text-status-warn hover:text-status-warn/80"
                    onClick={() => handleCancel(task.id)}
                    data-testid={`cancel-task-${task.id}`}
                  >
                    <X size={14} />
                  </Button>
                )}
                <Button
                  variant="ghost"
                  className="h-8 w-8 p-0 text-status-error/70 hover:text-status-error"
                  onClick={() => handleDelete(task.id)}
                  data-testid={`delete-task-${task.id}`}
                >
                  <Trash2 size={16}/>
                </Button>
              </div>
            </motion.div>
          ))}
        </motion.div>
      )}
    </div>
  );
};

export default TasksPage;
