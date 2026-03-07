# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-03-07

### ✨ Features

#### Flutter 前端重构 (Phase 9-11)

- **Flutter FFI 集成**
  - 使用 `flutter_rust_bridge` 2.x 实现 Dart-Rust 跨语言调用
  - FFI + HTTP API 双通道架构 (FFI 优先)
  - 类型安全的代码生成 (Freezed sealed class)

- **高级搜索 UI (Phase 9)**
  - 多关键词组合搜索 (AND/OR/NOT 逻辑操作符)
  - 正则表达式搜索 (使用 Rust DFA 引擎)
  - 搜索历史记录 (乐观更新模式)
  - 搜索结果实时高亮
  - 搜索条件预览面板

- **虚拟文件系统 UI (Phase 10)**
  - 虚拟文件树组件 (TreeNodeCache LRU 缓存)
  - 归档文件浏览 (ZIP/TAR/GZ/RAR/7Z)
  - 文件预览面板 (CAS 存储读取)
  - 懒加载优化 (展开目录时按需加载)

- **性能优化 (Phase 11)**
  - 内存缓存优化 (LRU 淘汰 + TTL 过期策略)
  - 默认 cacheExtent = itemHeight * 10 保证流畅滚动

- **UX 完善 (Phase 11)**
  - 骨架屏加载动画 (shimmer 包)
  - 无障碍支持 (Flutter Semantics Widget)
  - 空状态统一组件 (EmptyStateWidget)

#### 技术栈更新

- **Flutter**: 3.27+
- **Riverpod**: 3.0 (状态管理)
- **Freezed**: sealed class (FFI 类型处理)
- **flutter_rust_bridge**: 2.11.1

### 🔧 代码审查

已审查核心组件代码:
- `SearchQueryProvider` - 多关键词搜索状态管理
- `SearchHistoryProvider` - 搜索历史 + 乐观更新
- `VirtualFileTreeProvider` - 虚拟文件树 + LRU 缓存
- `FilePreviewPanel` - 文件预览 + 状态处理

代码质量: 良好

### 📚 Documentation

- **docs/README.md**: 更新 v1.2 功能列表
- **log-analyzer_flutter/README.md**: 更新项目状态和功能完成情况
- **新增 FFI_INTEGRATION.md**: Flutter FFI 集成文档

---

## [0.0.140] - 2026-02-11

### ✨ Features

#### 增量索引优化（Task 3）
- **偏移量持久化**: 应用重启后从上次位置继续读取日志文件
  - 新增 `IndexState` 和 `IndexedFile` 数据结构
  - SQLite 表 `index_state` 和 `indexed_files` 存储索引状态
  - 10 个单元测试覆盖所有 CRUD 操作
  - 位置: `src-tauri/src/storage/metadata_store.rs`

- **索引实时更新**: 监听的新内容可立即搜索
  - 修改 `append_to_workspace_index` 函数集成 Tantivy 持久化
  - `AppState` 新增 `search_engine_managers` 字段
  - 自动 commit 确保数据持久化

- **智能变更检测**: 基于 SHA-256 CAS 哈希避免无效索引
  - 新建 `file_change_detector.rs` 服务模块
  - `FileChangeStatus` 枚举: NewFile, ContentChanged, Unchanged, Truncated
  - 7 个单元测试覆盖变更检测逻辑
  - 支持批量处理和缓存管理

- **删除文件处理**: 删除文件时自动清理索引结果
  - 新增 `delete_file_documents` 方法到 SearchEngineManager
  - 删除时同时清理 Tantivy 索引和 indexed_files 表
  - 使用 TermQuery 精确匹配文件路径

#### 性能监控命令增强
- **P95/P99 延迟计算**: 使用业内成熟的排序算法计算百分位数
- **历史数据存储**: SQLite 时序数据存储 (metrics_store)
- **定时快照**: tokio::time::interval 异步定时器
- **修复运行时恐慌**: `try_lock()` 避免异步运行时阻塞

#### 新增文件
- `src-tauri/src/services/file_change_detector.rs` - 智能变更检测
- `src-tauri/src/storage/metrics_store.rs` - 性能指标时序存储
- `src/components/charts/` - 图表组件
- `src/hooks/__tests__/` - Hooks 单元测试
- `src/pages/__tests__/` - 页面单元测试

### ♻️ Refactor

#### 代码质量改进
- 修复 Clippy 警告:
  - 移除未使用的 `AppError` 导入
  - 修复 `ptr_arg` 警告 (`&PathBuf` → `&Path`, `&mut Vec<u64>` → `&mut [u64]`)
  - 修复 `await_holding_lock` 警告 (metrics_store.rs)
- 修复运行时恐慌: performance.rs:209 使用 `try_lock()` 替代 `blocking_lock()`

#### 代码组织
- 更新 `storage/mod.rs` 导出新类型
- 更新 `models/state.rs` 添加 `search_engine_managers` 字段

### 📚 Documentation

- **TODO.md**: 新增未完成任务清单文档
  - 记录 10 项未完成任务（Rust: 8项，前端: 2项）
  - 按优先级分类（P0: 1项，P1: 3项，P2: 6项）

- **README.md**:
  - 版本号更新至 0.0.140
  - 添加"增量索引优化"到已完成功能列表
  - 从"进行中"移除"增量索引优化"

### 🐛 Bug Fixes

- **WebView2 崩溃**: `cargo clean` 重建解决链接错误
- **async 运行时恐慌**: 修复 performance.rs 中的阻塞锁问题

---

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
