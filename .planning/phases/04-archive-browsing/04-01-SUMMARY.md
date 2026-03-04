---
phase: 04-archive-browsing
plan: 01
subsystem: archive
tags: [archive, browsing, backend, tauri]
dependency_graph:
  requires:
    - ARCH-01
    - ARCH-02
    - ARCH-03
  provides:
    - list_archive_contents command
    - read_archive_file command
    - ArchiveEntry struct
    - ArchiveHandler trait extensions
  affects:
    - Frontend archive browsing UI
tech_stack:
  added:
    - ArchiveEntry struct
    - list_contents method (async)
    - read_file method (async)
    - find_handler helper function
    - list_archive_contents Tauri command
    - read_archive_file Tauri command
  patterns:
    - async_trait for async trait methods
    - spawn_blocking for synchronous archive operations
key_files:
  created:
    - log-analyzer/src-tauri/src/commands/archive.rs
  modified:
    - log-analyzer/src-tauri/src/archive/archive_handler.rs
    - log-analyzer/src-tauri/src/archive/zip_handler.rs
    - log-analyzer/src-tauri/src/archive/tar_handler.rs
    - log-analyzer/src-tauri/src/archive/gz_handler.rs
    - log-analyzer/src-tauri/src/archive/rar_handler.rs
    - log-analyzer/src-tauri/src/archive/sevenz_handler.rs
    - log-analyzer/src-tauri/src/archive/mod.rs
    - log-analyzer/src-tauri/src/commands/mod.rs
    - log-analyzer/src-tauri/src/main.rs
decisions:
  - "使用 async_trait 实现异步 trait 方法"
  - "使用 spawn_blocking 在 async 函数中调用同步的压缩库"
  - "大文件截断阈值设为 10MB"
metrics:
  duration: ~5 minutes
  completed_date: "2026-03-02"
---

# Phase 04 Plan 01: 压缩包内容浏览后端实现

## 概述

实现压缩包内容浏览后端功能：列出压缩包文件列表、读取单个文件内容、创建 Tauri 命令接口。

## 完成的任务

| Task | Name | Commit |
|------|------|--------|
| 1 | 扩展 ArchiveHandler trait | 68b23ec |
| 2 | 实现 ZIP 格式处理 | 68b23ec |
| 3 | 实现 TAR/GZ/RAR/7Z 格式处理 | 68b23ec |
| 4 | 添加 find_handler 辅助函数 | 68b23ec |
| 5 | 创建 Tauri 命令 | 1d49a41 |

## 实现的功能

### 1. ArchiveEntry 数据结构
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,           // 文件/目录名称
    pub path: String,           // 完整路径
    pub is_dir: bool,           // 是否为目录
    pub size: u64,             // 文件大小（字节）
    pub compressed_size: u64,   // 压缩后大小
}
```

### 2. ArchiveHandler trait 扩展
- `list_contents(&self, path: &Path) -> Result<Vec<ArchiveEntry>>` - 列出压缩包内容
- `read_file(&self, path: &Path, file_name: &str) -> Result<String>` - 读取文件内容（10MB截断）

### 3. Tauri 命令
- `list_archive_contents(archive_path: String)` - 列出压缩包内容
- `read_archive_file(archive_path: String, file_name: String)` - 读取压缩包内文件

### 4. 支持的格式
- ZIP (.zip)
- TAR (.tar, .tar.gz, .tgz)
- GZ (.gz)
- RAR (.rar)
- 7Z (.7z)

## 大文件处理
- 默认截断阈值: 10MB
- 超过阈值时返回截断内容并附带提示信息

## 验证结果
- cargo check 通过
- cargo build 通过
- 命令已注册到 Tauri invoke_handler

## 后续工作
- 前端 Flutter UI 实现压缩包浏览页面
- 实现压缩包内搜索功能 (ARCH-03)
