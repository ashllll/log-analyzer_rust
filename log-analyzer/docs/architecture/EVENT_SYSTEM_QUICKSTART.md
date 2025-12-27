# 事件系统快速入门指南

> **5分钟上手企业级事件系统** | **适用人群**: 前端开发者

---

## 🚀 快速开始

### 步骤1: 导入EventBus

```typescript
// 推荐使用全局单例
import { eventBus } from './events/EventBus';
import type { TaskUpdateEvent } from './events/types';
```

### 步骤2: 注册事件监听器

```typescript
const handleTaskUpdate = (event: TaskUpdateEvent) => {
  console.log('任务更新:', event);
};

// 注册监听器
const unsubscribe = eventBus.on('task-update', handleTaskUpdate);
```

### 步骤3: 发送事件

```typescript
await eventBus.processEvent('task-update', {
  task_id: 'task-1',
  task_type: 'Import',
  target: '/path/to/file.log',
  progress: 50,
  message: 'Processing...',
  status: 'RUNNING',
  version: 1,
});
```

### 步骤4: 清理

```typescript
// 组件卸载时取消订阅
useEffect(() => {
  return () => {
    unsubscribe();
  };
}, []);
```

---

## 💡 常见场景

### 场景1: React组件监听任务更新

```typescript
import { useEffect, useState } from 'react';
import { eventBus } from './events/EventBus';
import type { TaskUpdateEvent } from './events/types';

function TaskProgress() {
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    const handleUpdate = (event: TaskUpdateEvent) => {
      if (event.task_id === 'my-task') {
        setProgress(event.progress);
      }
    };

    const unsubscribe = eventBus.on('task-update', handleUpdate);

    // 清理
    return () => unsubscribe();
  }, []);

  return <div>进度: {progress}%</div>;
}
```

### 场景2: 多个组件监听同一事件

```typescript
// 组件A - 任务列表
function TaskList() {
  useEffect(() => {
    return eventBus.on('task-update', (event) => {
      // 更新任务列表
      updateTaskInList(event);
    });
  }, []);
}

// 组件B - 通知系统
function NotificationSystem() {
  useEffect(() => {
    return eventBus.on('task-update', (event) => {
      // 显示通知
      if (event.status === 'COMPLETED') {
        showNotification(`任务 ${event.task_id} 完成`);
      }
    });
  }, []);
}

// 组件C - 性能监控
function PerformanceMonitor() {
  useEffect(() => {
    return eventBus.on('task-update', (event) => {
      // 记录性能指标
      trackMetrics(event);
    });
  }, []);
}
```

### 场景3: 版本号管理

```typescript
class TaskManager {
  private version = 1;

  async updateProgress(taskId: string, progress: number) {
    // 递增版本号
    this.version++;

    await eventBus.processEvent('task-update', {
      task_id: taskId,
      version: this.version,
      progress,
      message: `进度: ${progress}%`,
      status: 'RUNNING',
      task_type: 'Import',
      target: '/path',
    });
  }
}
```

### 场景4: 错误处理

```typescript
eventBus.on('task-update', (event) => {
  try {
    // 处理事件
    updateUI(event);
  } catch (error) {
    // 处理器错误不会影响其他处理器
    console.error('Handler error:', error);
  }
});
```

### 场景5: 条件过滤

```typescript
eventBus.on('task-update', (event) => {
  // 只处理特定工作区的任务
  if (event.workspace_id === currentWorkspace) {
    updateTaskDisplay(event);
  }
});

eventBus.on('task-update', (event) => {
  // 只处理完成的任务
  if (event.status === 'COMPLETED') {
    showCompletionNotification(event);
  }
});
```

---

## ⚙️ 配置选项

### 禁用日志（生产环境）

```typescript
const productionBus = new EventBus({
  enableLogging: false,  // 禁用日志
});
```

### 自定义日志级别

```typescript
eventBus.updateConfig({
  logLevel: 'warn',  // 只显示警告和错误
});
```

### 禁用验证（不推荐）

```typescript
const debugBus = new EventBus({
  enableValidation: false,  // 跳过Schema验证
});
```

---

## 📊 监控指标

### 查看指标

```typescript
const metrics = eventBus.getMetrics();

console.table({
  '总事件数': metrics.totalEvents,
  '验证错误': metrics.validationErrors,
  '处理错误': metrics.processingErrors,
  '幂等跳过': metrics.idempotencySkips,
  '处理器数量': metrics.handlersCount,
  '缓存大小': metrics.idempotencyCacheSize,
});
```

### 重置指标

```typescript
eventBus.resetMetrics();
```

### 清理缓存

```typescript
eventBus.clearCache();
```

---

## 🧪 测试示例

### Jest单元测试

```typescript
import { EventBus } from './events/EventBus';

describe('My Feature', () => {
  let testBus: EventBus;

  beforeEach(() => {
    testBus = new EventBus({ enableLogging: false });
  });

  it('应该处理任务更新事件', async () => {
    const handler = jest.fn();

    testBus.on('task-update', handler);

    await testBus.processEvent('task-update', {
      task_id: 'test-1',
      task_type: 'Import',
      target: '/path',
      progress: 50,
      message: 'Test',
      status: 'RUNNING',
      version: 1,
    });

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({
        task_id: 'test-1',
        progress: 50,
      })
    );
  });
});
```

---

## ❓ 常见问题

### Q1: 事件处理器执行顺序？

**A**: 不保证顺序，所有处理器并发执行。

```typescript
// ❌ 不要依赖执行顺序
eventBus.on('task-update', handler1);  // 可能先执行
eventBus.on('task-update', handler2);  // 可能后执行

// ✅ 如果需要顺序，在处理器内部控制
eventBus.on('task-update', async (event) => {
  await handler1(event);
  await handler2(event);
});
```

### Q2: 如何异步处理事件？

**A**: 使用async函数。

```typescript
eventBus.on('task-update', async (event) => {
  // 异步操作
  await fetchData();
  await updateDatabase();
});
```

### Q3: 事件数据被修改了吗？

**A**: 事件数据是共享引用，小心修改。

```typescript
eventBus.on('task-update', (event) => {
  // ❌ 不要修改原始事件
  // event.progress = 100;

  // ✅ 如果需要修改，先复制
  const modifiedEvent = { ...event, progress: 100 };
});
```

### Q4: 如何停止处理事件？

**A**: 取消订阅。

```typescript
const unsubscribe = eventBus.on('task-update', handler);

// 停止监听
unsubscribe();
```

### Q5: 幂等性缓存会占用多少内存？

**A**: 最大100条记录，约10KB。

---

## 📚 下一步

- 📖 [完整架构文档](./EVENT_SYSTEM.md)
- 🔧 [API参考](../../src/events/EventBus.ts)
- 🧪 [测试用例](../../src/events/__tests__/EventBus.test.ts)

---

**最后更新**: 2025-12-27 | **作者**: Claude (老王)
