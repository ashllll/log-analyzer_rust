# 日志分析器文档中心

本目录包含日志分析器项目的所有文档。

## 📁 文档结构

### 📐 架构文档 (`architecture/`)
系统架构、设计决策和技术规范

- **CAS_ARCHITECTURE.md** - 内容寻址存储(CAS)架构详解
- **API.md** - API 接口文档
- **ADVANCED_SEARCH_FEATURES_EXPLANATION.md** - 高级搜索功能说明

### 📚 用户指南 (`guides/`)
用户使用指南和快速参考

- **QUICK_REFERENCE.md** - 快速参考指南
- **MULTI_KEYWORD_SEARCH_GUIDE.md** - 多关键词搜索指南

### 🛠️ 开发文档 (`development/`)
开发环境配置、工具和流程

- **AGENTS.md** - AI Agent 开发指南
- **CLAUDE.md** - Claude AI 使用说明
- **gitlab-local-testing.md** - GitLab 本地测试指南
- **jenkins-local-testing.md** - Jenkins 本地测试指南
- **upgrade-nodejs.md** - Node.js 升级指南

### 📊 项目报告 (`reports/`)
项目进展报告和验证文档

#### 当前报告 (`reports/current/`)
- **CAS_MIGRATION_IMPLEMENTATION.md** - CAS迁移实施总结
- **MIGRATION_IMPLEMENTATION_SUMMARY.md** - 迁移实施摘要
- **IPC_MONITORING_IMPLEMENTATION.md** - IPC监控实现
- **WORKSPACE_PROCESSING_FIX.md** - Workspace PROCESSING 停留问题修复报告
- **TASK_22_COMPLETION_SUMMARY.md** - 任务22完成总结

#### 历史归档 (`reports/archive/`)
已完成的历史报告、任务文档和验证文档

- **[归档文档索引](reports/archive/README.md)** - 查看所有归档文档
- **[fixes/](reports/archive/fixes/)** - Bug修复和问题解决文档（9个）
- **[plans/](reports/archive/plans/)** - 已完成的实施计划（6个）
- **[tasks/](reports/archive/tasks/)** - 任务完成报告（16个）
- **[validation/](reports/archive/validation/)** - 验证和测试报告（7个）

#### 验证报告
- **TASK_13_FINAL_VALIDATION_REPORT.md** - TaskManager 生产就绪验证报告

## 🔍 快速导航

### 新用户
1. 阅读项目根目录的 [README.md](../README.md)
2. 了解 [CAS架构](architecture/CAS_ARCHITECTURE.md)
3. 查看 [快速参考指南](guides/QUICK_REFERENCE.md)
4. 了解 [多关键词搜索](guides/MULTI_KEYWORD_SEARCH_GUIDE.md)

### 开发者
1. 了解 [CAS架构设计](architecture/CAS_ARCHITECTURE.md)
2. 查看 [API 接口](architecture/API.md)
3. 阅读 [开发文档](development/)
4. 查看 [架构说明](architecture/)

### 项目管理
1. 查看 [最新报告](reports/)
2. 了解项目变更历史 [CHANGELOG.md](../CHANGELOG.md)

## 📝 文档维护

### 文档分类原则
- **architecture/** - 长期有效的架构和设计文档
- **guides/** - 面向用户的使用指南
- **development/** - 开发环境和工具文档
- **reports/** - 项目报告（当前有效）
- **reports/archive/** - 历史报告（已完成/过期）

### 归档规则
当报告或状态文档不再活跃时，应移动到 `reports/archive/` 目录。

## 🏗️ 架构亮点

### 内容寻址存储(CAS)

Log Analyzer 采用类似Git的内容寻址存储架构：

- ✅ **自动去重**: 相同内容只存储一次
- ✅ **无路径限制**: 使用SHA-256哈希，不受路径长度限制
- ✅ **数据完整性**: 哈希验证确保内容未被篡改
- ✅ **高效查询**: SQLite + FTS5全文搜索，性能提升10倍+

详见 [CAS架构文档](architecture/CAS_ARCHITECTURE.md)

## 🔗 相关链接

- [项目主页](../README.md)
- [变更日志](../CHANGELOG.md)
- [CAS迁移完成规范](../.kiro/specs/complete-cas-migration/)
