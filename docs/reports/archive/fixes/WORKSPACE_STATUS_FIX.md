# Workspace 状态更新问题修复总结

## 问题描述

Workspace 导入完成后，状态一直显示 "PROCESSING"，无法更新为 "READY"。

## 根本原因

1. **事件监听重复**：多个组件同时监听 `task-update` 和 `import-complete` 事件
   - `EventManager.tsx` (新架构)
   - `useTaskManager.ts` (旧架构，已废弃)
   - `AppContext.tsx` (旧架构，未使用但仍存在)

2. **React 闭包陷阱**：`EventManager` 的 `useEffect` 依赖数组包含 `tasks`，导致每次 tasks 变化时重新创建事件监听器

3. **Toast ID 重复**：`addToast` 使用 `Date.now()` 生成 ID，在快速连续调用时会产生重复 ID

4. **配置保存循环**：`useConfigManager` 的 `useEffect` 依赖数组包含 `debouncedSave` 函数，导致无限循环

## 解决方案

### 1. 采用单一事件源模式 (Single Source of Truth)

**修改文件**：
- `log-analyzer/src/components/EventManager.tsx`
- `log-analyzer/src/hooks/useTaskManager.ts`

**关键改进**：
- 只在 `EventManager` 中监听后端事件
- 使用 Zustand 的 `getState()` 模式避免闭包陷阱
- 移除 `useTaskManager` 中的事件监听，改为纯状态访问

```typescript
// EventManager.tsx - 使用 getState() 避免闭包
const store = useAppStore.getState();
store.updateWorkspace(workspace_id, { status: 'READY' });
```

### 2. 防止重复初始化

使用 `useRef` 防止 React StrictMode 下的重复初始化：

```typescript
const isInitializedRef = useRef(false);

useEffect(() => {
  if (isInitializedRef.current) return;
  isInitializedRef.current = true;
  // ... 初始化逻辑
}, []); // 空依赖数组
```

### 3. 修复 Toast ID 重复

**修改文件**：`log-analyzer/src/stores/appStore.ts`

使用单调递增计数器替代 `Date.now()`：

```typescript
let toastIdCounter = 0;

addToast: (type, message) => set((state) => {
  const id = ++toastIdCounter;
  state.toasts.push({ id, type, message });
}),
```

### 4. 修复配置保存循环

**修改文件**：`log-analyzer/src/hooks/useConfigManager.ts`

**关键改进**：
- 使用 `useRef` + `setTimeout` 手动实现防抖，避免函数引用变化
- 使用 `useCallback` 创建稳定的 `saveConfig` 函数
- 添加指纹比对 (`lastFingerprintRef`) 跳过未变更的配置

```typescript
const lastFingerprintRef = useRef<string>('');
const saveTimeoutRef = useRef<NodeJS.Timeout>();

const saveConfig = useCallback(() => {
  const configFingerprint = JSON.stringify({
    keywords: keywordGroups.map(g => ({ id: g.id, enabled: g.enabled })),
    workspaces: workspaces.map(w => ({ id: w.id, status: w.status }))
  });

  if (configFingerprint === lastFingerprintRef.current) {
    logger.debug('[CONFIG_MANAGER] Configuration unchanged, skipping save');
    return;
  }

  lastFingerprintRef.current = configFingerprint;
  configMutation.mutate({ keyword_groups: keywordGroups, workspaces });
}, [keywordGroups, workspaces, configMutation]);

useEffect(() => {
  if (saveTimeoutRef.current) clearTimeout(saveTimeoutRef.current);
  saveTimeoutRef.current = setTimeout(() => saveConfig(), 1000);
  return () => clearTimeout(saveTimeoutRef.current);
}, [saveConfig]);
```

### 5. 增强事件处理

- 添加详细日志用于调试
- 支持从任务中回退查找 `workspace_id`
- 在完成时显示成功 toast

## 当前状态

✅ Workspace 状态更新正常（显示 READY）
✅ Toast ID 重复警告已解决
✅ 配置保存循环已修复

## 已解决问题

1. **Toast ID 重复**：使用单调递增计数器 (`toastIdCounter`) 替代 `Date.now()`
2. **配置保存循环**：使用 `useRef` + `setTimeout` 手动防抖，配合指纹比对避免重复保存

## 测试验证

- [x] Workspace 导入后状态更新为 READY
- [x] 搜索功能正常工作（12744 条日志）
- [x] Toast 不再有重复 key 警告
- [x] 配置保存不再循环触发（显示 "Configuration unchanged, skipping save"）

## 补充修复：任务事件版本导致状态停留 PROCESSING

### 现象

- Tasks 页面无任务显示，但 Workspace 状态持续为 PROCESSING

### 根本原因

- 前端 EventBus 启用幂等性校验，依赖 `version` 单调递增
- 后端 `task-update` 事件未携带 `version`，前端默认回退为 `1`
- 更新事件与创建事件版本相同，被判定为重复事件并跳过
- `COMPLETED` 更新无法落地，Workspace 无法切回 READY

### 解决方案（业内成熟方案）

- 采用事件版本号（Event Versioning）与幂等性校验配套
- 由 TaskManager 统一维护任务版本，创建事件从 `1` 开始，后续更新递增
- 避免前端猜测版本，保持事件源单一职责

### 实施步骤（最小可执行）

1. 在 TaskInfo 增加 `version` 字段并初始化为 `1`
2. 每次 `UpdateTask` 时递增版本号（使用 `saturating_add` 防溢出）
3. 在 `task-update` 事件 payload 中携带 `version`
4. 保持前端逻辑不变，仅修复事件源数据

### 变更文件

- `log-analyzer/src-tauri/src/task_manager/mod.rs`

### 验证结果

- 导入完成后 Workspace 状态从 PROCESSING 正常切换为 READY
- `task-update` 不再被幂等性过滤，任务状态可持续更新
