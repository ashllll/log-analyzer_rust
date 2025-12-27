# Design Document - Complete CAS Migration

## Overview

本设计文档描述了完全移除旧 path_map 系统并确保100%使用 Content-Addressable Storage (CAS) 架构的技术方案。

### 当前 CAS 实现评估

**✅ 已达到业内标准的部分**:

1. **CAS 实现 (storage/cas.rs)**:
   - ✅ 使用 SHA-256 哈希（业内标准，Git 同款）
   - ✅ Git 风格的对象存储（2字符前缀分片）
   - ✅ 自动去重（相同内容 = 相同哈希）
   - ✅ 流式处理大文件（8KB 缓冲区）
   - ✅ 完整性验证（重新计算哈希）
   - ✅ 属性测试覆盖（100个测试用例）

2. **MetadataStore 实现 (storage/metadata_store.rs)**:
   - ✅ SQLite 数据库（成熟可靠）
   - ✅ FTS5 全文搜索（高性能）
   - ✅ 事务支持（ACID 保证）
   - ✅ 外键约束（级联删除）
   - ✅ 索引优化（virtual_path, hash, depth）
   - ✅ 异步操作（sqlx + tokio）

3. **AppState 架构 (models/state.rs)**:
   - ✅ 已使用 `cas_instances` 和 `metadata_stores`
   - ✅ 按 workspace_id 动态加载
   - ✅ 无全局 path_map（已移除）

**🎯 当前实现已经是成熟的业内方案！**

### 当前问题

虽然 CAS 架构已经实现且达到业内标准，但系统中仍保留大量旧代码：

1. **index_store.rs**: 使用 bincode 序列化的旧索引系统（已废弃）
2. **metadata_db.rs**: 包含 `path_mappings` 表的旧路径映射系统（已废弃）
3. **migration/mod.rs**: 用于从旧格式迁移的临时代码（不再需要）
4. **测试代码**: 包含 `create_traditional_workspace` 等旧测试辅助函数
5. **前端代码**: 迁移 UI 组件和逻辑（不再需要）

**这些旧代码的存在**:
- ❌ 增加系统复杂度
- ❌ 混淆代码意图
- ❌ 浪费维护精力
- ❌ 可能引入 bug

### 解决方案概述

采用"完全切换"策略：

1. **移除旧代码**: 删除所有 path_map 相关代码
2. **统一架构**: 所有功能使用 CAS + MetadataStore
3. **清理测试**: 更新所有测试使用新架构
4. **简化系统**: 移除迁移代码，不再支持旧格式

**目标**: 代码库100%使用成熟的 CAS 架构，无任何旧代码残留。

## Complete File Removal Checklist

### 后端文件（完全删除）

1. **src-tauri/src/services/index_store.rs** - 旧的 bincode 索引系统
2. **src-tauri/src/services/metadata_db.rs** - 旧的 path_mappings 表系统
3. **src-tauri/src/migration/mod.rs** - 迁移代码
4. **src-tauri/src/commands/migration.rs** - 迁移命令
5. **src-tauri/tests/migration_tests.rs** - 迁移测试
6. **temp_lib.rs** - 临时文件（根目录）

### 前端文件（完全删除）

1. **src/components/MigrationDialog.tsx** - 迁移对话框组件
2. **src/hooks/useMigration.ts** - 迁移 Hook

### 需要修改的文件

#### 后端

1. **src-tauri/src/services/mod.rs**
   - 移除: `pub use index_store::{load_index, save_index};`
   - 移除: `pub use metadata_db::MetadataDB;`
   - 移除: `mod index_store;`
   - 移除: `mod metadata_db;`

2. **src-tauri/src/lib.rs** 或 **src-tauri/src/main.rs**
   - 移除: `mod migration;`

3. **src-tauri/src/commands/import.rs**
   - 移除: `use crate::services::save_index;`
   - 移除: `save_index()` 调用
   - 确保使用 MetadataStore 持久化

4. **src-tauri/src/commands/workspace.rs**
   - 移除: `use crate::services::{load_index, save_index};`
   - 移除: `load_index()` 和 `save_index()` 调用
   - 使用 MetadataStore 替代

5. **src-tauri/src/models/config.rs**
   - 删除: `IndexData` 结构体
   - 删除: `FileMetadata` 结构体（如果只用于旧系统）

6. **src-tauri/src/models/state.rs**
   - 删除: `PathMapType` 类型别名
   - 删除: `MetadataMapType` 类型别名
   - 删除: `IndexResult` 类型别名

7. **src-tauri/Cargo.toml**
   - 检查并移除: `bincode` 依赖（如果只用于旧索引）
   - 检查并移除: `flate2` 依赖（如果只用于旧索引压缩）

#### 前端

1. **src/pages/WorkspacesPage.tsx**
   - 移除: `import { MigrationDialog } from '../components/MigrationDialog';`
   - 移除: `import { useMigration } from '../hooks/useMigration';`
   - 移除: 所有迁移相关的状态和逻辑
   - 移除: 迁移横幅 UI

2. **src/stores/workspaceStore.ts**
   - 移除: `format?: 'traditional' | 'cas' | 'unknown';`
   - 移除: `needsMigration?: boolean;`

3. **src/types/common.ts** (如果存在)
   - 移除: Workspace 类型中的迁移相关字段

### 数据库清理

1. **删除旧的 migration 表**（如果存在）
   - 检查 `migrations/` 目录
   - 移除创建 `path_mappings` 表的迁移文件

2. **清理旧的索引文件**
   - 在应用启动时检测并删除 `.idx.gz` 文件
   - 提示用户旧格式不再支持

## Architecture

### 当前架构（混合状态）

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  ┌──────────────┐         ┌──────────────┐              │
│  │ Import       │         │ Search       │              │
│  │ Commands     │         │ Commands     │              │
│  └──────┬───────┘         └──────┬───────┘              │
│         │                        │                       │
│         ├────────────────────────┤                       │
│         │                        │                       │
│  ┌──────▼────────┐        ┌─────▼──────┐               │
│  │ index_store   │        │ CAS +      │               │
│  │ (OLD)         │        │ Metadata   │               │
│  │ - bincode     │        │ (NEW)      │               │
│  │ - .idx.gz     │        │            │               │
│  └───────────────┘        └────────────┘               │
│         ❌                       ✅                      │
└─────────────────────────────────────────────────────────┘
```

### 目标架构（纯 CAS）

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  ┌──────────────┐         ┌──────────────┐              │
│  │ Import       │         │ Search       │              │
│  │ Commands     │         │ Commands     │              │
│  └──────┬───────┘         └──────┬───────┘              │
│         │                        │                       │
│         └────────────┬───────────┘                       │
│                      │                                   │
│               ┌──────▼──────┐                            │
│               │ CAS +       │                            │
│               │ Metadata    │                            │
│               │ Store       │                            │
│               │             │                            │
│               │ - SQLite    │                            │
│               │ - SHA-256   │                            │
│               │ - FTS5      │                            │
│               └─────────────┘                            │
│                      ✅                                   │
└─────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### 需要移除的组件

#### 1. index_store.rs (完全删除)

**文件路径**: `src-tauri/src/services/index_store.rs`

**移除原因**:
- 使用 bincode 序列化，不如 SQLite 可靠
- 使用 .idx.gz 文件，不支持并发访问
- 功能已被 MetadataStore 完全替代

**影响范围**:
- `src-tauri/src/services/mod.rs`: 移除 `pub use index_store::{load_index, save_index};`
- `src-tauri/src/commands/import.rs`: 移除 `save_index` 调用
- `src-tauri/src/commands/workspace.rs`: 移除 `load_index` 和 `save_index` 调用
- `src-tauri/src/migration/mod.rs`: 整个文件删除

#### 2. metadata_db.rs (完全删除)

**文件路径**: `src-tauri/src/services/metadata_db.rs`

**移除原因**:
- 此文件实现的是旧的 `path_mappings` 表系统
- 功能已被 `storage/metadata_store.rs` 完全替代
- 新系统使用 `files` 和 `archives` 表，不需要 `path_mappings`

**验证**: 检查是否有其他代码引用此文件，如果没有则完全删除

#### 3. migration/mod.rs (完全删除)

**文件路径**: `src-tauri/src/migration/mod.rs`

**移除原因**:
- 不再支持旧格式工作区
- 用户必须使用新版本创建工作区
- 简化系统复杂度

**影响范围**:
- `src-tauri/src/lib.rs` 或 `main.rs`: 移除 migration 模块声明
- `src-tauri/src/commands/migration.rs`: 删除文件
- 前端: 移除迁移相关 UI 组件

#### 4. 前端迁移组件 (完全删除)

**文件路径**:
- `src/components/MigrationDialog.tsx`
- `src/hooks/useMigration.ts`

**移除原因**:
- 不再支持旧格式迁移
- 简化前端代码

**影响范围**:
- `src/pages/WorkspacesPage.tsx`: 移除迁移相关 UI 和逻辑
- `src/stores/workspaceStore.ts`: 移除 `needsMigration` 字段

#### 5. 测试辅助函数 (完全删除)

**文件路径**: `src-tauri/tests/migration_tests.rs`

**移除原因**:
- 测试旧的迁移功能
- 不再需要

**影响范围**:
- 移除 `create_traditional_workspace_with_index` 函数
- 移除所有迁移相关测试
- 创建新的 `create_cas_workspace` 测试辅助函数

#### 6. temp_lib.rs (完全删除)

**文件路径**: `temp_lib.rs` (根目录)

**移除原因**:
- 包含旧的 AppState 定义（使用 path_map）
- 看起来是临时文件或备份文件

**验证**: 确认此文件不被使用后删除

### 需要更新的组件

#### 1. commands/import.rs

**当前问题**:
```rust
// ❌ 旧代码 - 还在使用 save_index
use crate::services::save_index;

let map_guard = state.path_map.lock();  // ❌ 使用旧的 path_map
let metadata_guard = state.file_metadata.lock();  // ❌ 使用旧的 file_metadata

match save_index(
    &app_handle,
    &workspace_id_clone,
    &map_guard,
    &metadata_guard,
) {
    Ok(index_path) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

**修改方案**:
```rust
// ✅ 新代码 - 使用 CAS + MetadataStore
// 不需要 save_index，MetadataStore 已经在导入过程中持久化所有数据

// 导入完成后的验证
let workspace_dir = get_workspace_dir(&workspace_id)?;
let metadata_store = MetadataStore::new(&workspace_dir).await?;
let file_count = metadata_store.count_files().await?;

info!(
    workspace_id = %workspace_id,
    file_count = file_count,
    "Import completed successfully"
);
```

**关键点**: 
- 移除 `use crate::services::save_index;`
- 移除对 `state.path_map` 和 `state.file_metadata` 的访问
- 移除 `state.workspace_indices` 的更新
- 确保导入过程中已经调用 `metadata_store.insert_file()`

#### 2. commands/workspace.rs

**当前问题**:
```rust
// ❌ 旧代码 - 还在使用 load_index 和 save_index
use crate::services::{load_index, save_index};

let (path_map, file_metadata) = load_index(&index_path)?;

// 使用 path_map 进行操作
for (real_path, virtual_path) in path_map.iter() {
    // ...
}

// 保存更新
save_index(&app_handle, &workspace_id, &path_map, &file_metadata)?;
```

**修改方案**:
```rust
// ✅ 新代码 - 使用 MetadataStore
let workspace_dir = get_workspace_dir(&workspace_id)?;
let metadata_store = MetadataStore::new(&workspace_dir).await?;
let all_files = metadata_store.get_all_files().await?;

// 使用 FileMetadata 进行操作
for file in all_files.iter() {
    let hash = &file.sha256_hash;
    let virtual_path = &file.virtual_path;
    // ...
}

// 不需要显式保存，MetadataStore 自动持久化
```

**关键点**:
- 移除 `use crate::services::{load_index, save_index};`
- 使用 `MetadataStore::get_all_files()` 替代 `load_index()`
- 移除所有 `save_index()` 调用
- 使用 `metadata_store.insert_file()` 或 `update_file()` 进行更新

#### 3. commands/async_search.rs

**当前问题**:
```rust
// ❌ 旧代码 - 使用 path_map 参数
async fn perform_async_search(
    path_map: Arc<parking_lot::Mutex<HashMap<String, String>>>,  // ❌
    // ...
) -> Result<usize, String> {
    // 获取文件列表
    let files: Vec<(String, String)> = {
        let guard = path_map.lock();
        guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    };
    
    // 搜索文件
    for (real_path, virtual_path) in files.iter() {
        search_file_async(real_path, virtual_path, &query, results_count).await?;
    }
}
```

**修改方案**:
```rust
// ✅ 新代码 - 使用 MetadataStore
async fn perform_async_search(
    workspace_id: String,  // ✅ 传递 workspace_id
    // ...
) -> Result<usize, String> {
    // 获取文件列表
    let workspace_dir = get_workspace_dir(&workspace_id)?;
    let metadata_store = MetadataStore::new(&workspace_dir).await?;
    let files = metadata_store.get_all_files().await?;
    
    // 获取 CAS 实例
    let cas = ContentAddressableStorage::new(workspace_dir);
    
    // 搜索文件
    for file in files.iter() {
        let content = cas.read_content(&file.sha256_hash).await?;
        search_content(&content, &file.virtual_path, &query, results_count).await?;
    }
}
```

**关键点**:
- 移除 `path_map` 参数
- 添加 `workspace_id` 参数
- 使用 `MetadataStore` 获取文件列表
- 使用 `CAS` 读取文件内容（通过 hash）

#### 4. commands/search.rs

**检查点**: 确保搜索命令使用 CAS

**验证**:
```rust
// ✅ 应该使用这种模式
let workspace_dir = get_workspace_dir(&workspace_id)?;
let metadata_store = MetadataStore::new(&workspace_dir).await?;
let cas = ContentAddressableStorage::new(workspace_dir);

// 查询文件
let files = metadata_store.search_files(&query).await?;

// 读取内容
for file in files {
    let content = cas.read_content(&file.sha256_hash).await?;
    // 搜索内容
}
```

#### 5. archive/processor.rs

**检查点**: 确保所有文件处理都使用 CAS

**验证**:
```rust
// ✅ 确保使用 CAS 存储
let hash = cas.store_file_streaming(file_path).await?;

// ✅ 确保使用 MetadataStore 记录
metadata_store.insert_file(&file_metadata).await?;

// ❌ 不应该有这样的代码
// path_map.insert(real_path, virtual_path);
```

### 需要更新的数据模型

#### 1. AppState (models/state.rs)

**当前定义**:
```rust
pub struct AppState {
    pub temp_dir: Mutex<Option<TempDir>>,
    pub cas_instances: Arc<Mutex<HashMap<String, Arc<ContentAddressableStorage>>>>,  // ✅ 保留
    pub metadata_stores: Arc<Mutex<HashMap<String, Arc<MetadataStore>>>>,  // ✅ 保留
    pub workspace_dirs: Arc<Mutex<HashMap<String, PathBuf>>>,  // ✅ 保留
    // ... 其他字段
}
```

**优化**: AppState 已经使用 CAS 架构，无需修改！这是好消息。

**验证**: 确认没有使用 `path_map` 或 `file_metadata` 字段

#### 2. IndexData (models/config.rs)

**当前定义**:
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexData {
    pub path_map: HashMap<String, String>,  // ❌ 移除
    pub file_metadata: HashMap<String, FileMetadata>,  // ❌ 移除
    pub workspace_id: String,
    pub created_at: i64,
}
```

**处理方案**: 完全删除此结构体，不再需要

**影响范围**:
- `src-tauri/src/services/index_store.rs`: 使用此结构体（整个文件删除）
- `src-tauri/src/migration/mod.rs`: 使用此结构体（整个文件删除）

#### 3. 类型别名 (models/state.rs)

**当前定义**:
```rust
/// 路径映射类型
/// real_path -> virtual_path
pub type PathMapType = HashMap<String, String>;  // ❌ 移除

/// 元数据映射类型
/// file_path -> FileMetadata
pub type MetadataMapType = HashMap<String, FileMetadata>;  // ❌ 移除

/// 索引操作结果类型
pub type IndexResult = Result<(PathMapType, MetadataMapType), String>;  // ❌ 移除
```

**处理方案**: 删除这些类型别名，不再需要

#### 4. Workspace 类型 (前端)

**当前定义** (`src/stores/workspaceStore.ts`):
```typescript
interface Workspace {
  id: string;
  name: string;
  format?: 'traditional' | 'cas' | 'unknown';  // ❌ 移除
  needsMigration?: boolean;  // ❌ 移除
  // ...
}
```

**新定义**:
```typescript
interface Workspace {
  id: string;
  name: string;
  // 所有工作区都是 CAS 格式，不需要 format 字段
  // ...
}
```

## Data Models

### 移除的数据模型

#### 1. IndexData

```rust
// ❌ 删除
pub struct IndexData {
    pub path_map: HashMap<String, String>,
    pub file_metadata: HashMap<String, FileMetadata>,
    pub workspace_id: String,
    pub created_at: i64,
}
```

#### 2. PathMapping

```rust
// ❌ 删除
pub struct PathMapping {
    pub id: i64,
    pub workspace_id: String,
    pub short_path: String,
    pub original_path: String,
    pub created_at: i64,
    pub access_count: i64,
}
```

### 保留的数据模型

#### 1. FileMetadata (storage/metadata_store.rs)

```rust
// ✅ 保留并使用
pub struct FileMetadata {
    pub id: i64,
    pub sha256_hash: String,
    pub virtual_path: String,
    pub original_name: String,
    pub size: i64,
    pub modified_time: i64,
    pub mime_type: Option<String>,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
}
```

#### 2. ArchiveMetadata (storage/metadata_store.rs)

```rust
// ✅ 保留并使用
pub struct ArchiveMetadata {
    pub id: i64,
    pub virtual_path: String,
    pub archive_type: String,
    pub parent_archive_id: Option<i64>,
    pub depth_level: i32,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: No Legacy Code References

*For any* source file in the codebase, it must not contain references to `path_map`, `PathMap`, `index_store`, `save_index`, or `load_index` (except in documentation)

**Validates: Requirements 1.1, 1.2**

**Rationale**: 确保旧代码完全移除，没有残留引用

### Property 2: CAS Storage Consistency

*For any* imported file, it must be stored in CAS and have a corresponding entry in MetadataStore

**Validates: Requirements 2.1, 2.2**

**Rationale**: 确保所有文件都使用新架构存储

### Property 3: Search Uses CAS

*For any* search operation, it must query MetadataStore and read content from CAS using SHA-256 hash

**Validates: Requirements 2.3**

**Rationale**: 确保搜索功能完全使用新架构

### Property 4: No Migration Code

*For any* source file, it must not contain migration-related code or references

**Validates: Requirements 3.1, 3.2, 3.3**

**Rationale**: 简化系统，不再支持旧格式

### Property 5: Test Coverage

*For any* test file, it must use CAS + MetadataStore for test setup and assertions

**Validates: Requirements 4.1, 4.2**

**Rationale**: 确保测试覆盖新架构

### Property 6: Clean Compilation

*For any* compilation, it must succeed without warnings related to unused imports or dead code

**Validates: Requirements 6.4**

**Rationale**: 确保代码库干净整洁

### Property 7: Database Schema Purity

*For any* workspace database, it must only contain CAS-related tables (files, archives, fts_files)

**Validates: Requirements 7.1**

**Rationale**: 确保数据库 schema 只包含新架构的表

### Property 8: API Consistency

*For any* Tauri command, it must use CAS architecture for data access

**Validates: Requirements 8.1, 8.2**

**Rationale**: 确保 API 接口一致性

## Error Handling

### 错误处理策略

#### 1. 旧格式工作区检测

**场景**: 用户尝试打开旧格式工作区

**处理**:
```rust
fn detect_workspace_format(workspace_dir: &Path) -> WorkspaceFormat {
    let metadata_db = workspace_dir.join("metadata.db");
    let objects_dir = workspace_dir.join("objects");
    
    if metadata_db.exists() && objects_dir.exists() {
        return WorkspaceFormat::CAS;
    }
    
    // 检测旧格式
    if workspace_dir.join("some_old_marker").exists() {
        return WorkspaceFormat::Legacy;
    }
    
    WorkspaceFormat::Unknown
}

// 在打开工作区时
match detect_workspace_format(&workspace_dir) {
    WorkspaceFormat::CAS => {
        // 正常打开
    }
    WorkspaceFormat::Legacy => {
        return Err(AppError::validation_error(
            "This workspace uses an old format that is no longer supported. \
             Please create a new workspace and re-import your files."
        ));
    }
    WorkspaceFormat::Unknown => {
        return Err(AppError::validation_error(
            "Unknown workspace format"
        ));
    }
}
```

#### 2. 编译错误处理

**场景**: 移除代码后可能出现的编译错误

**处理步骤**:
1. 移除 `index_store.rs` 后，修复所有 `use crate::services::index_store` 的引用
2. 移除 `migration` 模块后，修复所有 `use crate::migration` 的引用
3. 更新 `AppState` 后，修复所有访问 `path_map` 的代码
4. 运行 `cargo check` 确保没有编译错误

#### 3. 测试失败处理

**场景**: 移除旧代码后测试失败

**处理步骤**:
1. 识别依赖旧代码的测试
2. 更新测试使用 CAS + MetadataStore
3. 移除 `create_traditional_workspace` 等旧测试辅助函数
4. 创建新的测试辅助函数 `create_cas_workspace`

## Testing Strategy

### 代码搜索验证

**目标**: 确保没有旧代码残留

**方法**:
```bash
# 搜索 path_map 引用
rg "path_map|PathMap" --type rust

# 搜索 index_store 引用
rg "index_store|save_index|load_index" --type rust

# 搜索 migration 引用
rg "migration|migrate_workspace" --type rust

# 搜索 bincode 引用（用于旧索引）
rg "bincode" --type rust
```

**期望结果**: 只在文档和注释中出现

### 编译验证

**目标**: 确保系统可以编译

**方法**:
```bash
cd log-analyzer/src-tauri
cargo clean
cargo check
cargo build --release
```

**期望结果**: 无错误，无警告

### 单元测试

**目标**: 验证所有模块使用 CAS

**测试用例**:

1. **test_import_uses_cas**
```rust
#[tokio::test]
async fn test_import_uses_cas() {
    let workspace_id = "test_workspace";
    let workspace_dir = create_test_workspace_dir(workspace_id);
    
    // 导入文件
    import_folder(&test_folder, workspace_id).await.unwrap();
    
    // 验证 CAS 存储
    let cas = ContentAddressableStorage::new(workspace_dir.clone());
    let metadata_store = MetadataStore::new(&workspace_dir).await.unwrap();
    
    let files = metadata_store.get_all_files().await.unwrap();
    assert!(!files.is_empty());
    
    // 验证每个文件都在 CAS 中
    for file in files {
        let object_path = cas.get_object_path(&file.sha256_hash);
        assert!(object_path.exists());
    }
}
```

2. **test_search_uses_cas**
```rust
#[tokio::test]
async fn test_search_uses_cas() {
    let workspace_id = setup_test_workspace().await;
    
    // 执行搜索
    let results = search_logs("test query", workspace_id).await.unwrap();
    
    // 验证结果来自 CAS
    for result in results {
        // 结果应该包含 SHA-256 hash
        assert!(result.file_hash.is_some());
        assert_eq!(result.file_hash.unwrap().len(), 64); // SHA-256 长度
    }
}
```

3. **test_no_legacy_references**
```rust
#[test]
fn test_no_legacy_references() {
    // 搜索源代码中的旧引用
    let output = std::process::Command::new("rg")
        .args(&["path_map|PathMap|index_store", "--type", "rust", "src/"])
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // 只允许在文档和注释中出现
    for line in stdout.lines() {
        assert!(
            line.contains("//") || line.contains("/*") || line.contains("*/"),
            "Found legacy reference in code: {}",
            line
        );
    }
}
```

### 集成测试

**目标**: 验证完整工作流

**测试场景**:

1. **完整导入-搜索-删除流程**
```rust
#[tokio::test]
async fn test_complete_workflow() {
    // 1. 导入压缩包
    let workspace_id = import_archive("test.zip").await.unwrap();
    
    // 2. 验证 CAS 存储
    let workspace_dir = get_workspace_dir(&workspace_id);
    assert!(workspace_dir.join("metadata.db").exists());
    assert!(workspace_dir.join("objects").exists());
    
    // 3. 执行搜索
    let results = search_logs("error", &workspace_id).await.unwrap();
    assert!(!results.is_empty());
    
    // 4. 删除工作区
    delete_workspace(&workspace_id).await.unwrap();
    
    // 5. 验证清理
    assert!(!workspace_dir.exists());
}
```

2. **嵌套压缩包处理**
```rust
#[tokio::test]
async fn test_nested_archive() {
    let workspace_id = import_archive("nested.zip").await.unwrap();
    
    let metadata_store = MetadataStore::new(&get_workspace_dir(&workspace_id))
        .await
        .unwrap();
    
    // 验证所有文件都被索引
    let files = metadata_store.get_all_files().await.unwrap();
    assert!(files.len() > 0);
    
    // 验证嵌套结构
    let archives = metadata_store.get_all_archives().await.unwrap();
    assert!(archives.len() > 1); // 至少有2层
}
```

### 性能测试

**目标**: 确保性能不退化

**测试用例**:

1. **大文件导入性能**
```rust
#[tokio::test]
async fn test_large_file_import_performance() {
    let start = std::time::Instant::now();
    
    let workspace_id = import_large_archive("large.zip").await.unwrap();
    
    let duration = start.elapsed();
    
    // 应该在合理时间内完成
    assert!(duration.as_secs() < 60, "Import took too long: {:?}", duration);
    
    // 验证去重效果
    let metrics = get_workspace_metrics(&workspace_id).await.unwrap();
    assert!(metrics.deduplication_ratio > 0.0);
}
```

2. **搜索性能**
```rust
#[tokio::test]
async fn test_search_performance() {
    let workspace_id = setup_large_workspace().await;
    
    let start = std::time::Instant::now();
    let results = search_logs("test query", &workspace_id).await.unwrap();
    let duration = start.elapsed();
    
    // 搜索应该很快
    assert!(duration.as_millis() < 1000, "Search took too long: {:?}", duration);
    assert!(!results.is_empty());
}
```

## Migration Strategy

### Phase 1: 准备和分析（1天）

**任务**:
1. 完整代码审计，列出所有需要修改的文件
2. 创建备份分支
3. 运行所有现有测试，记录基线

**验证**:
- 所有测试通过
- 代码审计文档完成

### Phase 2: 移除旧代码（2天）

**任务**:
1. 删除 `index_store.rs`
2. 删除 `migration/mod.rs`
3. 清理 `metadata_db.rs` 中的 `path_mappings` 相关代码
4. 更新 `services/mod.rs` 移除旧导出

**验证**:
- 编译失败（预期）
- 记录所有编译错误

### Phase 3: 修复编译错误（3天）

**任务**:
1. 更新 `commands/import.rs`
2. 更新 `commands/workspace.rs`
3. 更新 `AppState` 定义
4. 移除 `IndexData` 结构体
5. 修复所有编译错误

**验证**:
- `cargo check` 通过
- `cargo build` 成功

### Phase 4: 更新测试（2天）

**任务**:
1. 移除旧测试辅助函数
2. 创建新的 CAS 测试辅助函数
3. 更新所有测试使用新架构
4. 添加新的验证测试

**验证**:
- 所有单元测试通过
- 所有集成测试通过

### Phase 5: 前端更新（1天）

**任务**:
1. 移除迁移相关 UI 组件
2. 更新错误提示信息
3. 测试前端功能

**验证**:
- 前端编译成功
- E2E 测试通过

### Phase 6: 文档和清理（1天）

**任务**:
1. 更新 README 和文档
2. 运行 linter 清理警告
3. 移除注释掉的代码
4. 最终代码审查

**验证**:
- 文档更新完成
- 无 linter 警告
- 代码审查通过

### Phase 7: 最终验证（1天）

**任务**:
1. 运行完整测试套件
2. 性能回归测试
3. 手动测试关键功能
4. 准备发布

**验证**:
- 所有测试通过
- 性能达标
- 功能正常

## Dependencies

### 需要移除的依赖

检查 `src-tauri/Cargo.toml` 中以下依赖是否只用于旧系统：

1. **bincode** - 用于旧索引序列化
   ```toml
   # ❌ 如果只用于 index_store.rs，则移除
   bincode = "1.3"
   ```

2. **flate2** - 用于旧索引压缩
   ```toml
   # ❌ 如果只用于 index_store.rs，则移除
   flate2 = "1.0"
   ```

**验证方法**:
```bash
# 搜索 bincode 使用
rg "bincode::" --type rust

# 搜索 flate2 使用
rg "flate2::" --type rust
```

如果只在 `index_store.rs` 和 `migration/mod.rs` 中使用，则可以安全移除。

### 保留的依赖

以下依赖用于 CAS 架构，必须保留：

1. **sqlx** - SQLite 数据库
2. **sha2** - SHA-256 哈希
3. **tokio** - 异步运行时
4. **serde** - 序列化（用于 JSON）

## Performance Considerations

### 1. 移除 bincode 序列化开销

**优势**: SQLite 比 bincode 更高效
- 支持索引查询
- 支持并发访问
- 支持事务

**测试**: 对比导入和搜索性能

### 2. 移除内存中的 HashMap

**优势**: 不再需要全局 `path_map` 和 `file_metadata`
- 减少内存占用
- 避免锁竞争
- 按需加载数据

**测试**: 监控内存使用

### 3. 简化代码路径

**优势**: 移除迁移代码减少复杂度
- 更快的编译时间
- 更少的运行时开销
- 更容易维护

**测试**: 对比编译时间和二进制大小

## Security Considerations

### 1. 移除旧格式支持

**安全优势**:
- 减少攻击面
- 避免旧代码的潜在漏洞
- 统一安全策略

**实现**:
```rust
// 拒绝旧格式工作区
if is_legacy_format(&workspace_dir) {
    return Err(AppError::security_error(
        "Legacy workspace format is not supported for security reasons"
    ));
}
```

### 2. 路径遍历防护

**确保**: CAS 架构天然防止路径遍历
- 所有文件通过 SHA-256 hash 访问
- 虚拟路径只用于显示
- 物理路径由系统控制

### 3. 数据完整性

**确保**: SQLite 提供 ACID 保证
- 事务支持
- 崩溃恢复
- 数据一致性

## Monitoring and Observability

### 关键指标

1. **代码质量指标**
   - 代码行数减少量
   - 编译警告数量
   - 测试覆盖率

2. **性能指标**
   - 导入速度
   - 搜索速度
   - 内存使用

3. **用户体验指标**
   - 错误率
   - 响应时间
   - 功能可用性

### 日志记录

```rust
// 记录关键操作
info!(
    workspace_id = %workspace_id,
    file_count = file_count,
    duration_ms = duration.as_millis(),
    "Import completed using CAS architecture"
);

// 记录性能指标
debug!(
    deduplication_ratio = metrics.deduplication_ratio,
    storage_efficiency = metrics.storage_efficiency,
    "CAS metrics collected"
);
```

## References

- **SQLite Documentation**: https://www.sqlite.org/docs.html
- **Git Object Storage**: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
- **Rust std::fs**: https://doc.rust-lang.org/std/fs/
- **Content-Addressable Storage**: https://en.wikipedia.org/wiki/Content-addressable_storage
