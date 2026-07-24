# Spec: 死代码清理与 ErrorBoundary 重构

> 基于 2026-07-25 架构评审的决策规格说明

## 1. 背景

使用 `/improve-codebase-architecture` 技能对 log-analyzer_rust 项目进行全面架构评审。通过 codebase-memory 图谱（7,058 节点 / 24,232 边）+ grep 双重验证，识别并执行了以下改进。

---

## 2. 死代码清理

### 2.1 决策依据

| 验证方法 | 结果 |
|----------|------|
| codebase-memory `max_degree=0` | 175 个候选 |
| grep 二次验证 | 仅 2 个真实死代码 |
| 图谱误报率 | ~98% |

### 2.2 误报根因分析

| 根因 | 影响函数数 | 示例 |
|------|-----------|------|
| `#[serde(default)]` 宏引用 | ~60 | `default_100mb()`, `default_true()` |
| trait 方法动态分派 | ~15 | `Filter::has_time_filter()` |
| Zustand state selector | 2 | `state.deleteWorkspace` |
| Iterator `next()` 隐式调用 | 5 | `MemchrMatches::next()` |
| Makefile targets | 5 | `check`, `clean`, `coverage` |

### 2.3 已清理代码

#### TaskManager::health_check

- **文件**: `src-tauri/src/task_manager/mod.rs`
- **类型**: 结构体方法
- **行数**: 4 行
- **验证**: 全项目 grep 无调用（包括测试）
- **影响**: 无，TaskManager 通过 `sender.is_closed()` 可获取同等信息

#### WatchService::is_watching

- **文件**: 
  - trait 定义: `src-tauri/src/application/workspace_service.rs`
  - impl 实现: `src-tauri/src/infrastructure/workspace_service_impl/watch.rs`
- **类型**: trait 方法 + 实现
- **行数**: trait 3 行 + impl 4 行
- **验证**: 全项目 grep 无调用（包括测试）
- **影响**: 无，WatchService 保留 `start_watch` / `stop_watch` 足够

### 2.4 保留代码（误判）

| 代码 | 保留原因 |
|------|----------|
| `SearchQueryBuilder::getSummary` | 测试在用，非死代码 |
| `SearchQueryBuilder::export` | 测试在用，非死代码 |

---

## 3. ErrorBoundary 重构

### 3.1 决策依据

| 指标 | 拆分前 | 阈值 | 判断 |
|------|--------|------|------|
| 文件行数 | 717 | >500 | ⚠️ 超标 |
| 导出数量 | 14 | >8 | ⚠️ 超标 |
| 职责数量 | 5 | >3 | ⚠️ 超标 |

### 3.2 候选自审

| # | 候选 | 原评估 | 自审后 | 依据 |
|---|------|--------|--------|------|
| 1 | SearchUseCase | Strong | 跳过 | 502 行实现已结构良好，ADR-0001 禁止 trait |
| 2 | Workspace 命令 | Worth | 跳过 | 447 行 / 7 函数 ≈ 64 行/函数，职责清晰 |
| 3 | EventBus | Worth | 跳过 | 458 行 / 2 类，幂等性复杂度是内在的 |
| 4 | ErrorBoundary | Speculative | **执行** | 717 行 / 14 导出 / 5 职责 |
| 5 | API 层 | Speculative | 跳过 | 608 行 / 25 方法 / 1 类，合理粒度 |

### 3.3 拆分规格

#### 模块结构

```
components/error/
├── index.ts                    # 统一导出
├── errorLogManager.ts          # 错误日志存储 + 报告器配置
├── ErrorCard.tsx               # 错误信息展示卡片
├── ErrorFallbacks.tsx          # CompactErrorFallback + PageErrorFallback
├── ErrorBoundaries.tsx         # AppErrorBoundary + PageErrorBoundary
└── initGlobalErrorHandlers.ts  # 全局错误处理器初始化
```

#### 依赖图

```
errorLogManager.ts (纯逻辑，零依赖)
       ↑
       ├── ErrorCard.tsx (仅 lucide-react + errors service)
       │        ↑
       │        └── ErrorFallbacks.tsx
       │                ↑
       │                └── ErrorBoundaries.tsx
       │
       └── initGlobalErrorHandlers.ts (react-hot-toast)
```

#### 模块职责

| 模块 | 职责 | 行数 | 依赖 |
|------|------|------|------|
| `errorLogManager.ts` | 错误日志 CRUD + 报告器配置 | 135 | 无 |
| `ErrorCard.tsx` | 错误信息展示 UI | 126 | lucide-react, errors service |
| `ErrorFallbacks.tsx` | 错误回退 UI 组件 | 119 | ErrorCard, errors service |
| `ErrorBoundaries.tsx` | React 错误边界类组件 | 215 | ErrorCard, ErrorFallbacks, errorLogManager |
| `initGlobalErrorHandlers.ts` | 全局错误处理初始化 | 121 | errorLogManager, react-hot-toast |
| `index.ts` | 统一导出 | 33 | 所有子模块 |

#### 向后兼容

`ErrorBoundary.tsx` 保留为薄重导出层（31 行），现有 import 不受影响：

```typescript
// ErrorBoundary.tsx
export {
  configureErrorReporter,
  getErrorLogs,
  clearErrorLogs,
  exportErrorLogs,
  ErrorCard,
  CompactErrorFallback,
  PageErrorFallback,
  AppErrorBoundary,
  PageErrorBoundary,
  initGlobalErrorHandlers,
  type ErrorReporter,
  type ErrorLogEntry,
  type ErrorReportContext,
  type ErrorInfoProps,
} from "./error";
```

### 3.4 验证清单

| 检查项 | 方法 | 结果 |
|--------|------|------|
| TypeScript 编译 | `tsc --noEmit` | ✅ 通过 |
| ESLint | `eslint src` | ✅ 通过 |
| 前端测试 | `jest` | ✅ 441 passed |
| Rust 编译 | `cargo check --workspace` | ✅ 通过 |
| Rust 测试 | `cargo test --workspace` | ✅ 530 passed |
| 向后兼容 | grep 验证导入点 | ✅ App.tsx, PageTransition.tsx 正常 |
| 依赖无循环 | 依赖图分析 | ✅ 单向流动 |

---

## 4. 图谱工具局限性记录

### 4.1 已知误报模式

| 模式 | 原因 | 预防方法 |
|------|------|----------|
| serde `default_*` 函数 | 编译期宏生成调用，图谱不追踪 | 排除 serde 相关函数 |
| trait 方法 | 动态分派调用链断裂 | grep 验证 impl 调用点 |
| Zustand selector | `state.xxx` 间接引用 | grep 验证 hook 使用 |
| Iterator `next()` | `for` 循环隐式调用 | 排除 Iterator 实现 |
| Makefile targets | 非函数调用 | 排除 Makefile |

### 4.2 推荐工作流

```
1. 图谱 max_degree=0 初筛（快速，但高误报）
2. grep 二次验证每个候选（精确，但慢）
3. 阅读实际代码确认上下文
4. 只有 grep + 代码审查双重确认才执行清理
```

---

## 5. 附录

### 5.1 相关 ADR

- ADR-0001: 文件处理策略不定义 trait

### 5.2 变更文件清单

| 文件 | 变更类型 |
|------|----------|
| `src-tauri/src/task_manager/mod.rs` | 删除 health_check |
| `src-tauri/src/application/workspace_service.rs` | 删除 is_watching trait 方法 |
| `src-tauri/src/infrastructure/workspace_service_impl/watch.rs` | 删除 is_watching impl |
| `src/components/ErrorBoundary.tsx` | 重写为重导出层 |
| `src/components/error/errorLogManager.ts` | 新增 |
| `src/components/error/ErrorCard.tsx` | 新增 |
| `src/components/error/ErrorFallbacks.tsx` | 新增 |
| `src/components/error/ErrorBoundaries.tsx` | 新增 |
| `src/components/error/initGlobalErrorHandlers.ts` | 新增 |
| `src/components/error/index.ts` | 新增 |

### 5.3 Commit

```
5de110e refactor: clean dead code and split ErrorBoundary module
```
