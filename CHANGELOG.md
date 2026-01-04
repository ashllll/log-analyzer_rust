# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.96] - 2026-01-04

### ✨ Features

- **search**: 搜索关键词字符限制从 100 字符放宽到 500 字符
  - 前端警告阈值调整：`SearchQueryBuilder.ts` (100 → 500)
  - 用户现在可以使用更长的搜索词进行精确匹配
  - 后端硬限制仍为 1000 字符，保证系统稳定性

- **search-history**: 搜索历史功能
  - **核心特性**: 保存最近 50 条搜索记录，支持快速重用
  - **自动保存**: 每次搜索完成后自动保存到历史记录
  - **智能去重**: 相同查询只保留最新的记录
  - **工作区隔离**: 不同工作区的搜索历史独立管理
  - **历史操作**:
    - 点击历史记录快速重用搜索
    - 删除单条历史记录
    - 清空所有历史
  - **时间格式化**: 相对时间显示（刚刚、N分钟前、N小时前、N天前）
  - **结果统计**: 显示每次搜索的结果数量
  - **UI 组件**: `src/components/SearchHistory.tsx`
    - 时钟图标按钮，位于搜索输入框右侧
    - 下拉框展示历史记录列表
    - 悬停显示删除按钮
  - **后端实现**:
    - `src-tauri/src/models/search_history.rs` - 数据模型和管理器
    - `src-tauri/src/commands/search_history.rs` - 5 个 Tauri 命令
    - 12 个单元测试全部通过
  - **测试覆盖**: 包含添加、去重、限制、过滤、搜索前缀、大小写不敏感等测试

- **fuzzy-search**: 模糊搜索功能（基础框架）
  - **核心算法**: Levenshtein 距离（编辑距离）
  - **动态阈值**: 短词严格（≤4字符，最多1个差异），中等词（5-8字符，最多2个差异），长词宽松（>8字符，最多3个差异）
  - **拼写错误容忍**: 自动匹配相似关键词（如 "ERRO" → "ERROR", "connetion" → "connection"）
  - **UI 交互**: 搜索栏新增模糊搜索开关按钮（"模糊: 开/关"）
  - **后端实现**:
    - `src-tauri/src/services/fuzzy_matcher.rs` - Levenshtein 算法实现
    - 支持精确匹配、单字符差异、多字符差异检测
    - Unicode 字符支持
    - 最佳匹配查找
  - **前端实现**:
    - `src/types/search.ts` - 添加 `fuzzyEnabled` 字段
    - `src/pages/SearchPage.tsx` - 添加模糊搜索开关和状态管理
  - **状态**: 基础框架完成，算法实现，UI 就绪
    - 注：完整的模糊搜索集成（QueryPlanner 和 QueryExecutor）作为未来扩展预留

- **auto-word-boundary**: 智能自动单词边界检测
  - **问题解决**: 修复搜索 "DE H|DE N" 时错误匹配 "CODE HDEF" 的问题
  - **零用户配置**: 完全自动检测，用户无需手动切换模式
  - **智能启发式**: 5 条规则自动判断是否需要单词边界
    1. 用户手动输入 `\b` → 立即使用单词边界
    2. 常见日志关键词（ERROR, WARN, INFO, DE H, DE N）→ 自动单词边界
    3. 短的字母数字组合（≤10字符）→ 自动单词边界
    4. 包含空格的短语（≤30字符，无特殊字符）→ 自动单词边界
    5. 其他情况 → 保持子串匹配（向后兼容）
  - **实现文件**: `src-tauri/src/services/query_planner.rs`
  - **测试覆盖**: 19 个测试用例全部通过（包括关键的 `test_android_log_search`）
  - **性能影响**: < 15ms 延迟增加，缓存机制确保重复搜索无额外开销
  - **向后兼容**: 现有用户无感知，复杂模式（URL、特殊字符）自动保持子串匹配

- **file-filter**: 实现三层检测策略的文件类型过滤系统
  - 第1层：二进制文件检测（JPEG、PNG、EXE、MP3 等魔数检测）
  - 第2层：智能过滤规则（文件名模式 + 扩展名白名单/黑名单）
  - 防御性设计：失败安全、零侵入、Feature Flag（默认禁用第2层）
  - 新增 `FileFilterSettings` UI 组件用于配置过滤规则
  - 性能影响：<1ms/文件，导入总开销 <5%
  - 7个单元测试全部通过

- **ci**: 跨平台兼容性全面改进
  - 新增 `.github/workflows/cross-platform-tests.yml` 跨平台测试工作流
  - 支持 Linux/macOS/Windows 多平台 CI 测试
  - 修复多个平台特定的编译错误

- **encoding**: UTF-8编码容错处理
  - 统一事件源架构
  - 增强编码错误处理能力

### ♻️ Refactor

- **移除性能监控模块**: 移除 ~9500 行性能监控代码，简化代码库
  - 降低维护成本，提高代码可读性
  - 保留核心功能不受影响

### 📚 Documentation

- **CLAUDE.md**: 更新至版本 0.0.96
  - 更新版本号和日期
  - 文档结构优化

- **文档清理**: 统一文档管理
  - 删除重复文档目录 `log-analyzer/docs/`
  - 删除过时计划文件 `plans/`
  - 删除性能优化归档文档
  - 迁移 `CAS_ARCHITECTURE.md` 到 `docs/architecture/`
  - 更新 `docs/README.md` 文件计数

### 🐛 Fixes

- **eventbus**: 发送单调递增的任务事件版本号，防止幂等性检查导致工作区停留在 PROCESSING 状态
- **ci**: 修复跨平台测试 YAML 语法错误
- **test**: 修复 Windows 路径规范化测试

## [Unreleased]

### 🚧 Work in Progress

- 新功能开发中...

## [0.1.0] - 2025-12-27

### 🎉 Major Release: Complete CAS Architecture Migration

This release marks the completion of the Content-Addressable Storage (CAS) architecture migration,
replacing the legacy `path_map` based file indexing system.

### 🚀 Features

- **Complete CAS Architecture**: Migrated from legacy `path_map` system to Content-Addressable Storage
- **Unified Metadata Store**: New `MetadataStore` for efficient file metadata management
- **Streaming Archive Processing**: Improved archive handling with streaming support
- **Enhanced Search**: Search now uses CAS for file content retrieval

### 🔧 Changes

#### Removed Files

- `src-tauri/src/services/index_store.rs` - Old index storage system
- `src-tauri/src/services/metadata_db.rs` - Legacy path shortening (refactored)
- `src-tauri/src/migration/mod.rs` - Migration module (no longer needed)
- `src-tauri/src/commands/migration.rs` - Migration commands
- `src-tauri/tests/migration_tests.rs` - Legacy migration tests
- `temp_lib.rs` - Temporary library file
- `src/components/MigrationDialog.tsx` - Frontend migration UI
- `src/hooks/useMigration.ts` - Migration hook
- `src-tauri/migrations/20231221000001_create_path_mappings.sql` - Legacy schema
- `src-tauri/migrations/config_migration.rs` - Config migration
- `src-tauri/migrations/migrate_to_enhanced_archive.rs` - Archive migration

#### Modified Commands

- `commands/import.rs` - Updated to use `MetadataStore::insert_file()`
- `commands/workspace.rs` - Uses `MetadataStore::get_all_files()` instead of `load_index`
- `commands/async_search.rs` - Added `workspace_id` parameter, uses CAS for content

#### Updated Data Models

- Removed `IndexData` struct from `models/config.rs`
- Removed `PathMapType`, `MetadataMapType`, `IndexResult` from `models/state.rs`
- Removed `format` and `needsMigration` from frontend types

### 🧪 Testing

- Added property tests for CAS storage consistency
- Added property tests for search using CAS
- Added E2E tests for CAS migration workflows
- All existing tests updated to use CAS + MetadataStore

### 📚 Documentation

- Updated README.md with CAS architecture documentation
- Added `docs/architecture/CAS_ARCHITECTURE.md`
- Added migration guide for users
- Updated API documentation

### ⚠️ Breaking Changes

- **Legacy Format Support Dropped**: Old `.idx.gz` index files are no longer supported
- **No Migration Path**: Users with old workspace format must create new workspaces
- **Database Schema Change**: Replaced `path_mappings` table with `files` and `archives` tables

### 🛠️ Under the Hood

- CAS storage for content-addressable file storage
- SQLite-based metadata store with proper indexing
- Streaming file processing for better memory efficiency
- Parallel archive processing support

### 📦 Dependencies

- Updated `sqlx` for improved database operations
- Added `async-compression` for streaming compression

## [0.0.71] - Previous Versions

See [git history](https://github.com/joeash/log-analyzer/commits/main) for earlier changes.
