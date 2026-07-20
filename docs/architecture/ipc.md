# IPC 与状态同步

Log Analyzer 是单机桌面应用。前端通过 Tauri `invoke` 发起请求，通过 Tauri Events 接收长任务与工作区变化；没有 WebSocket 服务。

## 两类通道

| 通道 | 方向 | 用途 |
| --- | --- | --- |
| Tauri commands | React → Rust | 搜索、导入、配置、工作区、导出等请求 / 响应操作 |
| Tauri Events | Rust → React | 任务进度、导入结果、工作区变化、文件就绪等异步通知 |

```mermaid
flowchart LR
  PAGE[React page] --> HOOK[hooks / React Query]
  HOOK --> API[services/api.ts]
  API -->|invoke| COMMAND[#[tauri::command]]
  COMMAND --> STATE[AppState + use cases]
  STATE --> PUBLISH[EventPublisher]
  PUBLISH -->|emit| PROJECTION[tauriEventProjection]
  PROJECTION --> STORE[Zustand stores]
  STORE --> PAGE
```

## 命令边界

`commands/` 负责把不可信 IPC 输入转为内部类型，执行必要校验，再委托 application / infrastructure。业务逻辑不应重新堆回 command 函数。

## 前端事件投影

`mountTauriEventProjection` 集中订阅 Tauri 事件、兼容 payload 形状并投影到前端状态。React hook 负责 mount / unmount 生命周期、依赖注入与清理，避免页面组件直接了解底层事件细节。

## 状态职责

- **Zustand**：工作区、任务、关键词和 UI 共享状态。
- **React Query**：请求缓存、失效与服务端状态读取。
- **EventBus / projection**：把 Rust 异步通知转换为前端可消费的事件和 store 更新。
- **AppState**：Rust 端服务、registry、scheduler 与配置的运行时容器。

## 一致性检查

仓库提供 `scripts/check_ipc_consistency.sh`，用于检查前后端 command / event 命名漂移。修改 IPC 名称或 payload 时，应同时更新 Rust 命令、前端 API、事件投影、类型和测试。

