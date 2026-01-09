# RAR处理器纯Rust重构报告

> **日期**: 2026-01-09
> **版本**: 0.0.104 → 0.0.109

## 重构概述

将RAR文件处理器从依赖外部unrar二进制方案改为纯Rust实现（rar crate），同时保持向后兼容性。

## 技术方案

### 双模式策略

```
┌─────────────────┐     ┌──────────────────┐
│   RAR文件输入    │────▶│  rar crate (主)   │
│                 │     │  纯Rust RAR4支持  │
└─────────────────┘     └──────────────────┘
                              │
                              ▼ Fallback
                        ┌──────────────────┐
                        │  unrar 二进制    │
                        │  RAR5/加密/多部分 │
                        └──────────────────┘
```

### 技术选型

| 方案 | 优点 | 缺点 |
|------|------|------|
| rar crate 0.4 | 纯Rust，无C依赖 | 仅支持RAR4基础格式 |
| unarc-rs | 功能完整 | 依赖冲突 (lzma-rust2) |
| unrar C绑定 | 完整支持 | 需要C库，跨平台问题 |

**决策**: 采用 `rar crate` + `unrar fallback` 双模式

## 实现要点

### 1. Cargo依赖

```toml
# Phase 5: Archive Support (Pure Rust Priority)
rar = "0.4"  # Pure Rust RAR library
```

### 2. RAR处理器架构

```rust
pub struct RarHandler;

#[async_trait]
impl ArchiveHandler for RarHandler {
    async fn extract_with_limits(
        &self,
        source: &Path,
        target_dir: &Path,
        max_file_size: u64,
        max_total_size: u64,
        max_file_count: usize,
    ) -> Result<ExtractionSummary> {
        // 优先使用纯Rust rar crate
        match RarHandler::extract_with_rar_crate(...).await {
            Ok(summary) if summary.files_extracted > 0 => Ok(summary),
            _ => RarHandler::extract_with_unrar_fallback(...).await,
        }
    }
}
```

### 3. 平台支持

- **Windows**: unrar-x86_64-pc-windows-msvc.exe
- **macOS**: unrar-aarch64-apple-darwin / unrar-x86_64-apple-darwin
- **Linux**: unrar-x86_64-unknown-linux-gnu

## 解决的问题

### macOS ARM64 构建问题

**问题**: unrar Rust crate依赖C库，在macOS ARM64上构建失败。

**解决方案**: 采用sidecar二进制方案
- 纯Rust主库 (rar crate)
- 平台特定unrar二进制作为fallback

## 测试验证

### 测试用例

```rust
#[test]
fn test_rar_handler_can_handle() {
    let handler = RarHandler;
    assert!(handler.can_handle(Path::new("test.rar")));
    assert!(handler.can_handle(Path::new("test.RAR")));
}

#[test]
fn test_parse_unrar_output_basic() {
    // unrar输出解析测试
}
```

### 测试结果

- ✅ 4个单元测试通过
- ✅ cargo check 通过
- ✅ cargo clippy 无警告

## 相关文件

- `src/archive/rar_handler.rs` - RAR处理器实现
- `Cargo.toml` - 依赖配置
