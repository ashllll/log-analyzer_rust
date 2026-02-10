# 项目概述

## 项目名称
log-analyzer_rust - 高性能桌面日志分析工具

## 版本
0.0.137

## 项目目的
高性能桌面日志分析工具，支持多格式压缩包解压、全文搜索、实时监听等功能。

## 技术栈

### 后端 (Rust)
- **框架**: Tauri 2.0
- **版本**: Rust 1.70+
- **核心依赖**:
  - tokio 1.x (异步运行时)
  - aho-corasick 1.1 (多模式搜索)
  - tantivy 0.22 (全文搜索引擎)
  - sqlx 0.7 (SQLite数据库)
  - rayon 1.8 (并行处理)
  - regex 1.11 (正则表达式)

### 前端 (React)
- **框架**: React 19.1.0
- **语言**: TypeScript 5.8.3
- **状态管理**: Zustand 5.0.9
- **样式**: Tailwind CSS 3.4.17

### 压缩包支持
- ZIP (zip crate)
- TAR/GZ (tar, flate2 crates)
- RAR (unrar crate - libunrar绑定)
- 7Z (sevenz-rust - 纯Rust实现)

## 代码风格和约定

### Rust编码规范
- **命名**: `snake_case` (模块/函数), `CamelCase` (类型/Trait)
- **错误处理**: 使用 `thiserror` 创建统一错误类型
- **异步**: 使用 `async/await` + tokio运行时
- **日志**: 使用 `tracing` crate
- **测试**: 80%+ 覆盖率要求

### TypeScript编码规范
- **命名**: `PascalCase` (组件/类型), `camelCase` (变量/函数)
- **组件**: 函数式组件 + Hooks
- **样式**: Tailwind Utility类
- **国际化**: 文案走 `i18n` 字典

### 前后端集成规范
- **字段名必须一致**: Rust `task_id` = JSON `task_id` = TypeScript `task_id`
- **使用snake_case**: 前后端统一使用snake_case命名
