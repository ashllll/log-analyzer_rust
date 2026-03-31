[根目录](../../../CLAUDE.md) > [src-tauri](../) > **archive (压缩包处理)**

# 压缩包处理模块

> ZIP/TAR/GZ/RAR/7Z 递归解压 | 最后更新: 2026-03-31

## 架构说明

此模块为 **la-archive crate 的 re-export 包装**。实际实现位于 workspace crate：

```
crates/la-archive/src/
├── lib.rs                  # crate 入口，ArchiveManager
├── archive_handler.rs      # ArchiveHandler Trait
├── extraction_engine.rs    # 解压引擎
├── extraction_orchestrator.rs # 编排器
├── processor.rs            # CAS 集成处理
├── public_api.rs           # 对外 API
├── security_detector.rs    # 安全检查（解压炸弹防护）
├── checkpoint_manager.rs   # 断点续传
├── path_manager.rs         # 路径管理
├── zip_handler.rs          # ZIP 处理器
├── tar_handler.rs          # TAR/TAR.GZ 处理器
├── gz_handler.rs           # GZ 处理器
├── rar_handler.rs          # RAR 处理器
└── sevenz_handler.rs       # 7Z 处理器
```

## 使用方式

通过 `la_archive` crate 直接使用：

```rust
use la_archive::ArchiveManager;

let manager = ArchiveManager::new();
let summary = manager.extract_archive(source, target).await?;
```

## 支持格式

| 格式 | 处理器 | 扩展名 |
|------|--------|--------|
| ZIP | ZipHandler | `.zip` |
| TAR | TarHandler | `.tar`, `.tar.gz`, `.tgz` |
| GZ | GzHandler | `.gz` |
| RAR | RarHandler | `.rar` (需启用 `rar-support` feature) |
| 7Z | SevenZHandler | `.7z` |

## 安全特性

- 单文件大小限制（默认 100MB）
- 总大小限制（默认 1GB）
- 文件数量限制（默认 1000）
- 递归深度限制
- 解压炸弹检测
- 路径穿越防护

---

*详细架构规范请参见根目录 [CLAUDE.md](../../../CLAUDE.md)*
