# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.54] - 2026-03-25

### 🐛 Bug Fixes

- **CI/CD 修复**: 修复 GitHub Actions 工作流失败
  - `ilammy/msvc-toolchain@v1` 改为 `@v1.13.0` 固定版本
  - 移除已弃用的 `toolset: latest` 参数
  - 解决 action 无法解析的问题

- **Clippy 警告修复**: 解决 CI 中 `-D warnings` 导致的构建失败
  - `boolean_query_processor.rs`: 使用 `clamp()` 替换 `max().min()` 模式
  - `disk_result_store.rs`: 使用 `io::Error::other()` 替换 `io::Error::new(Other, ...)`
  - `cas.rs`: 为 `TempFileGuard` 添加 `#[allow(dead_code)]` 属性
  - `cache_monitor.rs`: 移除未使用的 `tempfile::TempDir` 导入
  - `gc.rs`: 移除未使用的 `tempfile::TempDir` 导入
  - `advanced_features.rs`: 修复未使用变量 `negative_ts` 警告

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

## [1.2.53] - 2026-03-25

### 🛡️ Security Audit & Reliability Improvements

本次发布包含全面的 CAS（内容寻址存储）系统安全审计修复，提升了数据完整性和系统可靠性。

#### 🔧 Bug Fixes

- **编译错误修复 (P0)**
  - `search.rs:1427`: 修复元组字段访问错误 `status.total_count` → `status.0`
  - `extractor.rs:86,92`: 移除 `Result` 类型上错误的 `.await` 调用
  - `zip_handler.rs:75`: 使用 `unix_mode()` 位掩码替代不存在的 `is_symlink()` 方法

- **数据完整性修复 (P1)**
  - **BUG-005**: CAS 缓存与文件系统状态竞态条件
    - 实现双检查模式 (Double-Check Pattern)
    - `exists()` 和 `exists_async()` 方法缓存命中后验证文件系统状态
    - 不匹配时自动使缓存失效，防止 TOCTOU 竞态条件
  - **BUG-006**: 孤儿文件问题 - Saga 补偿事务模式
    - CAS 写入成功但元数据提交失败时自动清理孤儿文件
    - 引用计数检查防止误删去重文件
    - 详细日志记录清理操作
  - **BUG-007**: 临时文件泄漏 - RAII 清理机制
    - 新增 `TempFileGuard` RAII 结构
    - Drop trait 确保临时文件清理（即使 panic 也能保证）
    - 支持异步和同步两种清理模式

#### ✨ Features

- **垃圾回收系统** (`storage/gc.rs`)
  - 自动清理无引用的 CAS 对象
  - 可配置 GC 策略：间隔（默认1小时）、年龄阈值（默认5分钟）、批大小（默认1000）
  - 支持手动触发和自动后台运行
  - 详细的 GC 统计信息（扫描文件数、清理文件数、回收字节数）
  - 安全删除：零引用验证、年龄检查、空目录清理

- **缓存一致性监控** (`storage/cache_monitor.rs`)
  - 实时监控 CAS 存在性缓存健康状态
  - 跟踪指标：总查询数、命中数、未命中数、陈旧条目数、不一致修复数
  - 自动检测并修复缓存与文件系统不一致
  - 可配置的检查间隔和自动修复策略

- **Saga 事务协调器增强** (`storage/coordinator.rs`)
  - 改进错误处理和事务回滚日志
  - 详细的错误分类（FOREIGN KEY 约束、UNIQUE 约束等）
  - 事务回滚失败时提供详细的诊断信息

#### ♻️ Refactor

- **架构模块化**
  - 新增 `storage/gc.rs` - 垃圾回收模块
  - 新增 `storage/cache_monitor.rs` - 缓存监控模块
  - 更新 `storage/mod.rs` 导出新类型
  - 为 CAS 添加 `objects_dir()` 公共方法

#### 📚 Documentation

- **CAS_ARCHITECTURE.md**: 更新架构文档
  - 添加 Storage Coordinator (Saga Pattern) 章节
  - 添加 Garbage Collector 章节
  - 添加 Cache Consistency Monitor 章节
  - 添加可靠性保障机制说明

- **CODE_REVIEW_REPORT.md**: 添加安全审计修复报告
  - 记录所有 P0/P1 问题修复详情
  - 记录架构设计改进
  - 验证结果汇总

#### 🧪 Testing

- 所有 662+ 现有测试通过
- 新增垃圾回收模块单元测试
- 新增缓存监控模块单元测试
- 验证 Saga 事务回滚和孤儿清理逻辑

#### 📊 Performance Impact

- 双检查缓存模式：增加一次文件系统检查，但显著提升数据一致性
- 垃圾回收：后台运行，对正常操作无性能影响
- 缓存监控：低开销后台任务，可配置关闭

### 🔍 Search Engine Optimizations

#### Bug Fixes

- **WalkDir 目录遍历深度限制** (processor.rs)
  - 修复 `max_depth(1)` 只遍历一层目录的问题
  - 默认改为无限制遍历（`usize::MAX`），可通过 `PROCESSOR_MAX_DEPTH` 环境变量配置
  - 确保深层嵌套目录中的日志文件能被正确处理

- **时间戳验证改进** (advanced_features.rs: ADV-H1)
  - 改进无效时间戳处理策略：使用特殊标记 `i64::MIN` 而非默认值 0
  - 区分处理：解析失败、零值、负时间戳分别进入 "unknown" 分区
  - 避免无效时间戳污染正常时间线，提升时间范围过滤准确性

- **递归改迭代实现** (advanced_features.rs: ADV-M3)
  - `collect_suggestions()` 已改为 BFS 迭代实现，避免深层 Trie 递归栈溢出
  - `count_nodes()` 已改为 BFS 迭代实现，防止栈溢出风险

#### ♻️ Refactor

- **取消机制改进** (boolean_query_processor.rs: BQP-H3)
  - 细粒度取消检查：从每 1024 个文档改为每 256 个文档检查一次
  - 新增高分文档（score > 0.9）立即检查策略，确保快速响应取消请求
  - 添加详细的注释说明取消策略和优化原理

- **成本估计算法优化** (boolean_query_processor.rs: BQP-H5)
  - 重新设计成本模型：考虑布尔操作符类型、选择性、项数量、交集/并集复杂度
  - 使用反比关系计算扫描成本（选择性越低，成本越高）
  - 添加布尔复杂度因子：Must 项越多，交集成本呈次线性增长（+30%/项）
  - 添加 Should 项惩罚：超过 5 个后每个额外增加 10% 成本
  - 详细的调试日志输出，便于性能调优分析

- **字符计数缓存优化** (highlighting_engine.rs: HLE-H4)
  - 大文档内容提取优化：使用字节位置切片替代字符遍历
  - 限制搜索范围：只搜索文档前 10KB，避免超大文档全文扫描
  - 新增 `truncate_by_bytes()` 辅助函数，使用 `char_indices()` 高效截断
  - 性能提升：对于大文档，提取相关内容的复杂度从 O(n) 降至 O(搜索范围)

#### 🧪 Testing

- 所有 677 个单元测试通过
- 验证 WalkDir 遍历深层目录结构
- 验证时间戳分区正确处理无效时间
- 验证搜索取消机制在各种场景下正常工作

---

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
