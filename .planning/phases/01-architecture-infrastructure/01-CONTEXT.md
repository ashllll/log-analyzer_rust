# Phase 1: 架构基础设施 - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Flutter 应用具备与 Rust 后端通信的基础设施，包括项目结构、共享服务、错误处理框架。项目使用 FFI 与 Rust 后端通信，支持 Riverpod 状态管理，具备用户友好的错误处理能力。

</domain>

<decisions>
## Implementation Decisions

### 通信模式 (FFI)
- **模式**: 仅使用 FFI 与 Rust 后端通信（不使用 HTTP）
- **初始化**: 延迟加载 - 首次调用时初始化 FFI
- **库路径**: 标准路径加载（Windows: exe 同目录, macOS: Contents/MacOS）
- **调试**: 纯 FFI 模式，不保留 HTTP 备选
- **失败处理**: 显示错误页面，提供重试按钮

### 错误处理
- **错误码**: 分段设计 (0-999 通用错误, 1000+ 模块特定错误)
- **UI 展示**: 统一错误页面，显示错误码和解决方案
- **日志收集**: 使用 Sentry 上报错误（Flutter 端已集成）
- **连接错误**: 显示"后端未连接"错误页面，提供启动后端指引

### Provider 架构
- **文件组织**: 单文件模块（workspace_provider.dart, search_provider.dart 等）
- **状态类**: 使用 Freezed 生成不可变状态类
- **异步模式**: 使用 AsyncNotifier 管理异步状态
- **依赖注入**: Provider 构造函数注入依赖

### 启动流程
- **Splash**: 显示应用 logo + "正在连接后端..." 文字
- **检测内容**: 检测 FFI 库是否可加载
- **超时时间**: 10秒
- **失败处理**: 显示错误页面，有"重试"按钮

### Claude's Discretion
- FFI 方法通道的具体命名规范
- 错误码的具体分段数值范围
- Splash 界面的具体设计和动画
- Provider 文件的具体拆分粒度

</decisions>

<specifics>
## Specific Ideas

- 当前代码已有 ApiService 和 BridgeService，需要重构为纯 FFI 模式
- BridgeService 中 `const bool _useHttpClient = true` 需要改为 `false` 并移除 Dio 依赖
- FFI 生成的代码已在 `lib/shared/services/generated/` 目录，需要启用
- 错误处理需要与后端约定错误码协议

</specifics>

<deferred>
## Deferred Ideas

- HTTP 调试模式 - Phase 1 后如有需要可恢复
- 离线模式支持 - 当前不需要，后端必须可用

</deferred>

---

*Phase: 01-architecture-infrastructure*
*Context gathered: 2026-02-28*
