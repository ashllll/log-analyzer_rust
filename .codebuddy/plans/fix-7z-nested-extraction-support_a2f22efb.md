---
name: fix-7z-nested-extraction-support
overview: 在 ExtractionEngine 中补全 7z 格式的层级解压支持，确保移除外部二进制后功能完整。
todos:
  - id: update-is-archive-file
    content: 在 log-analyzer/src-tauri/src/archive/extraction_engine.rs 的 is_archive_file 函数中添加 7z 扩展名支持
    status: completed
  - id: register-sevenz-handler
    content: 在 log-analyzer/src-tauri/src/archive/extraction_engine.rs 的 create_handler_registry 中注册 SevenZHandler
    status: completed
    dependencies:
      - update-is-archive-file
  - id: verify-extraction-logic
    content: 添加或更新单元测试以验证 ExtractionEngine 对 7z 文件的识别和处理器匹配逻辑
    status: completed
    dependencies:
      - register-sevenz-handler
  - id: check-imports-consistency
    content: 检查 extraction_engine.rs 中是否需要显式导入 SevenZHandler 或使用正确的 crate 路径
    status: completed
---

## 产品概述

在 `ExtractionEngine` 中完善 7z 格式的层级解压支持，确保在移除外部二进制文件（如 unrar/7z）后，系统仍能利用纯 Rust 实现（`sevenz-rust`）对嵌套在其他压缩包内的 7z 文件进行自动识别和多层级解压。

## 核心功能

- **7z 扩展名识别**：在 `ExtractionEngine` 的压缩包识别逻辑中增加对 `.7z` 后缀的支持，使其能识别出嵌套的 7z 文件。
- **7z 处理器注册**：将 `SevenZHandler` 注册到 `ExtractionEngine` 的处理器注册表中，实现对 7z 格式的实际解压操作。
- **功能完整性验证**：确保 `ExtractionEngine` 的迭代解压流程（Iterative Extraction）能够正确调用 `SevenZHandler` 处理多层嵌套。

## 技术栈

- **后端框架**: Rust + Tauri
- **解压库**: `sevenz-rust` (纯 Rust 实现，用于处理 7z 格式)
- **并发与异步**: `tokio` (异步任务管理), `async-trait`
- **日志与追踪**: `tracing`

## 系统架构

### 模块划分

- **ExtractionEngine**: 核心提取引擎，负责管理解压深度、路径安全、并发控制及迭代遍历嵌套压缩包。
- **ArchiveHandler (SevenZHandler)**: 7z 格式的具体处理器，实现 `ArchiveHandler` 特性。
- **HandlerRegistry**: 存储在 `ExtractionEngine` 内部，用于根据文件扩展名动态匹配对应的处理器。

### 数据流

1. `ExtractionEngine` 提取主压缩包。
2. 发现提取的文件中包含 `.7z` 后缀。
3. `is_archive_file` 返回 `true`，将该 7z 文件路径压入 `ExtractionStack`。
4. 下一次循环从栈中弹出该 7z 文件。
5. 通过 `HandlerRegistry` 查找并获取 `SevenZHandler`。
6. 调用 `SevenZHandler::extract_with_limits` 进行解压。

## 实现细节

### 核心目录结构

```
log-analyzer/src-tauri/src/archive/
├── extraction_engine.rs  # 修改点：增加 7z 识别与处理器注册
├── sevenz_handler.rs     # 已存在：7z 处理器实现
└── mod.rs                # 模块导出（已完成）
```

### 关键代码逻辑修改

1. **`is_archive_file` 扩展**:
在 `is_archive_file` 函数的扩展名数组中增加 `"7z"`。
2. **`create_handler_registry` 注册**:
在 `create_handler_registry` 函数中使用 `registry.register(Box::new(crate::archive::sevenz_handler::SevenZHandler))` 进行注册。

## 测试策略

- **单元测试**: 在 `extraction_engine.rs` 中添加测试用例，模拟包含 7z 扩展名的路径，验证 `is_archive_file` 是否返回正确结果。
- **集成测试**: 如果环境允许，测试一个包含 7z 的嵌套压缩包（例如 `test.zip` -> `inner.7z` -> `log.txt`）的完整解压流程。