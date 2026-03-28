# 模块架构文档索引

> 本目录包含项目的完整模块架构文档

## 文档列表

| 文档 | 描述 | 最后更新 |
|-----|------|---------|
| [MODULE_ARCHITECTURE.md](./MODULE_ARCHITECTURE.md) | 完整模块架构文档 | 2026-03-28 |

## 快速导航

### Rust后端模块

- [命令层 (commands/)](./MODULE_ARCHITECTURE.md#1-commands---tauri-ipc命令层) - 15个Tauri IPC命令
- [服务层 (services/)](./MODULE_ARCHITECTURE.md#2-services---业务服务层) - 19个业务服务模块
- [搜索引擎 (search_engine/)](./MODULE_ARCHITECTURE.md#3-search_engine---搜索引擎) - 11个Tantivy搜索模块
- [存储层 (storage/)](./MODULE_ARCHITECTURE.md#4-storage---cas存储层) - 7个CAS存储模块
- [压缩处理 (archive/)](./MODULE_ARCHITECTURE.md#5-archive---压缩包处理) - 27个压缩处理模块
- [数据模型 (models/)](./MODULE_ARCHITECTURE.md#6-models---数据模型) - 15个数据模型
- [事件系统 (events/)](./MODULE_ARCHITECTURE.md#7-events---事件系统) - EventBus架构
- [任务管理 (task_manager/)](./MODULE_ARCHITECTURE.md#8-task_manager---任务管理器) - Actor模型
- [领域层 (domain/)](./MODULE_ARCHITECTURE.md#9-domain---领域层ddd) - DDD领域模型
- [应用层 (application/)](./MODULE_ARCHITECTURE.md#10-application---应用层ddd) - 应用服务+插件
- [基础设施 (infrastructure/)](./MODULE_ARCHITECTURE.md#11-infrastructure---基础设施层) - 配置管理
- [监控 (monitoring/)](./MODULE_ARCHITECTURE.md#12-monitoring---监控和可观测性) - 可观测性
- [状态同步 (state_sync/)](./MODULE_ARCHITECTURE.md#13-state_sync---状态同步) - 实时状态同步
- [工具模块 (utils/)](./MODULE_ARCHITECTURE.md#14-utils---工具模块) - 12个工具模块

### React前端模块

- [状态管理 (stores/)](./MODULE_ARCHITECTURE.md#1-stores---状态管理) - Zustand状态管理
- [自定义Hooks (hooks/)](./MODULE_ARCHITECTURE.md#2-hooks---自定义hooks) - 22个自定义Hooks
- [API服务 (services/)](./MODULE_ARCHITECTURE.md#3-services---api服务) - Tauri IPC封装
- [UI组件 (components/)](./MODULE_ARCHITECTURE.md#4-components---ui组件) - UI组件库
- [页面组件 (pages/)](./MODULE_ARCHITECTURE.md#5-pages---页面组件) - 6个页面组件
- [类型定义 (types/)](./MODULE_ARCHITECTURE.md#6-types---typescript类型) - TypeScript类型
- [常量定义 (constants/)](./MODULE_ARCHITECTURE.md#7-constants---常量定义) - 应用常量
- [国际化 (i18n/)](./MODULE_ARCHITECTURE.md#8-i18n---国际化) - 多语言支持
- [Provider (providers/)](./MODULE_ARCHITECTURE.md#9-providers---provider组件) - React Context

## 模块统计

### 后端统计

| 类别 | 数量 |
|-----|------|
| 顶层模块 | 15个 |
| 命令模块 | 15个 |
| 服务模块 | 19个 |
| 搜索模块 | 11个 |
| 存储模块 | 7个 |
| 压缩模块 | 27个 |
| 模型模块 | 15个 |
| 工具模块 | 12个 |
| **总计** | **121+个** |

### 前端统计

| 类别 | 数量 |
|-----|------|
| 页面组件 | 6个 |
| Hooks | 22个 |
| Store | 7个 |
| 服务模块 | 7个 |
| UI组件 | 15+个 |
| **总计** | **57+个** |

## 文档生成说明

本文档基于项目代码直接分析生成，不依赖已有文档内容。生成过程：

1. 分析 `src-tauri/src/lib.rs` 确定顶层模块
2. 读取各模块的 `mod.rs` 确定子模块列表
3. 验证模块数量与实际代码一致
4. 整理模块功能和依赖关系

---

*最后更新: 2026-03-28*
