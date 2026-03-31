<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# hooks (自定义Hooks)

## Purpose
封装可复用的React逻辑，包括业务hooks和工具hooks。

## Key Files

| File | Description |
|------|-------------|
| `useTauriEventListeners.ts` | Tauri事件监听管理 |
| `useInfiniteSearch.ts` | 无限滚动搜索 |
| `useSearchListeners.ts` | 搜索结果监听 |
| `useConfigInitializer.ts` | 配置初始化 |
| `useKeyboardShortcuts.ts` | 键盘快捷键 |
| `useToast.ts` | 提示消息封装 |

## For AI Agents

### Working In This Directory
- Hooks使用 useCallback/useMemo优化性能
- 事件监听必须返回清理函数
- 异步操作使用 AbortController 支持取消

### Testing Requirements
- 使用 @testing-library/react-hooks 测试
- 模拟依赖的context和props

### Common Patterns
- useEffect 返回清理函数
- 使用 ref 存储不触发渲染的值
- 复杂逻辑拆分为多个小hooks

## Dependencies

### Internal
- `services/` - API调用
- `stores/` - 状态管理

### External
- **react** - Hooks API

<!-- MANUAL: -->
