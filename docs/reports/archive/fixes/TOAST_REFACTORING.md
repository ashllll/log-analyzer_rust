# Toast 通知系统重构

## 问题描述

原有的 toast 通知系统存在以下问题：
1. Toast 不会自动消失，需要手动点击关闭
2. 自定义实现的定时器管理存在 ID 不匹配的 bug
3. 代码复杂度高，维护困难

## 解决方案

采用业内成熟的 **react-hot-toast** 库替换自定义实现。

### 为什么选择 react-hot-toast？

1. **业内标准**：React 生态中最流行的 toast 库之一（GitHub 9k+ stars）
2. **自动生命周期管理**：内置自动显示/隐藏逻辑，无需手动管理定时器
3. **性能优化**：使用 React Portal 和优化的渲染策略
4. **丰富功能**：
   - 自动堆叠管理
   - 流畅的进入/退出动画
   - 可自定义样式
   - 支持 Promise 状态
   - 无障碍访问支持
5. **轻量级**：仅 ~3KB gzipped
6. **零依赖**：除了 React 本身

## 实现变更

### 1. 安装依赖

```bash
npm install react-hot-toast
```

### 2. 创建 ToastProvider 组件

`src/components/ToastProvider.tsx` - 配置全局 toast 样式和行为

### 3. 重构 useToastManager Hook

`src/hooks/useToastManager.ts` - 使用 react-hot-toast API 替代自定义实现

### 4. 更新 App.tsx

- 移除旧的 `ToastContainer` 组件
- 添加 `ToastProvider` 组件
- 使用 `useToastManager` hook

### 5. 更新 EventManager

`src/components/EventManager.tsx` - 直接使用 `toast` API 而不是 store 的 `addToast`

## 迁移指南

### 旧代码
```typescript
const { addToast, removeToast } = useApp();
addToast('success', 'Operation completed');
```

### 新代码
```typescript
const { showSuccess, showError, showInfo } = useToastManager();
showSuccess('Operation completed');
```

或者直接使用：
```typescript
import toast from 'react-hot-toast';
toast.success('Operation completed');
```

## 配置选项

Toast 的默认配置在 `ToastProvider.tsx` 中：
- **位置**：右下角 (bottom-right)
- **持续时间**：3000ms (3秒)
- **样式**：深色主题，匹配应用整体设计

可以在调用时覆盖：
```typescript
showSuccess('Message', 5000); // 显示 5 秒
```

## 优势

1. ✅ **自动消失**：Toast 会在指定时间后自动消失
2. ✅ **无 Bug**：使用经过充分测试的库，避免自定义实现的 bug
3. ✅ **更好的 UX**：流畅的动画和过渡效果
4. ✅ **易于维护**：减少自定义代码，依赖成熟的开源方案
5. ✅ **功能丰富**：支持更多高级功能（如 Promise 状态、自定义渲染等）

## 向后兼容

为了保持向后兼容，`addToast` 函数仍然可用，但内部使用 `useToastManager`：

```typescript
const addToast = (type: 'success' | 'error' | 'info', message: string) => {
  showToast(type, message);
};
```

## 测试

构建测试通过：
```bash
npm run build
✓ built in 1.61s
```

## 参考资料

- [react-hot-toast 官方文档](https://react-hot-toast.com/)
- [GitHub 仓库](https://github.com/timolins/react-hot-toast)
