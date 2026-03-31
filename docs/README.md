# 文档索引

本目录只保留当前项目仍需维护的核心文档。原则是：

- 保留会随着代码一起长期维护的说明
- 删除一次性分析、修复计划、工具迁移残留和重复指南
- 文档内容以当前代码真实行为为准

## 建议阅读顺序

1. [项目总览](../README.md)
2. [贡献指南](./CONTRIB.md)
3. [IPC API 概览](./architecture/API.md)
4. [模块架构](./architecture/modules/MODULE_ARCHITECTURE.md)
5. [CAS 架构](./architecture/CAS_ARCHITECTURE.md)

## 保留文档

### 项目与流程

- [CONTRIB.md](./CONTRIB.md)
  - 开发环境、提交流程、测试约定
- [RUNBOOK.md](./RUNBOOK.md)
  - 运行、排障、回滚与常见问题
- [search-optimization-review.md](./search-optimization-review.md)
  - 搜索性能和边界条件的最新审核记录

### 架构

- [architecture/API.md](./architecture/API.md)
  - 前后端 IPC 命令与事件的核心接口说明
- [architecture/CAS_ARCHITECTURE.md](./architecture/CAS_ARCHITECTURE.md)
  - 工作区存储、CAS、元数据和导入链路
- [architecture/modules/MODULE_ARCHITECTURE.md](./architecture/modules/MODULE_ARCHITECTURE.md)
  - 当前代码模块、职责边界与调用关系
- [architecture/modules/README.md](./architecture/modules/README.md)
  - 模块架构文档入口

### 仓库级文档

- [../CHANGELOG.md](../CHANGELOG.md)
  - 历史版本变更记录
- [../RELEASE_PROCESS.md](../RELEASE_PROCESS.md)
  - 发布步骤与校验要求

## 已移除的文档类型

以下类型不再作为主文档保留：

- 一次性架构分析报告
- 代码审查报告与修复计划
- AI 工具专用 `CLAUDE.md` 说明
- 与当前 CI 不匹配的本地 GitLab / Jenkins 模拟文档
- 与主 README 重复的用户快速参考页

如后续需要恢复某类文档，要求先确认它仍有持续维护价值。
