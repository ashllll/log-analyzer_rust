# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
