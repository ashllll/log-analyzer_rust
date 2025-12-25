# Requirements Document

## Introduction

本规范旨在解决压缩包导入后无法搜索到文件内容的问题。该问题的根本原因是解压后的文件路径映射不正确，导致搜索引擎无法访问实际的文件内容。

本规范将采用业内成熟的文件索引和路径管理模式，确保压缩包解压、索引构建和搜索查询之间的数据一致性。

## Glossary

- **System**: 日志分析器应用程序
- **Archive File**: 压缩文件（ZIP、RAR、TAR等格式）
- **Extracted Directory**: 解压后文件存储的临时目录
- **Path Map**: 真实文件路径到虚拟路径的映射表
- **File Metadata**: 文件的元数据信息（大小、修改时间等）
- **Search Index**: 用于快速搜索的文件索引
- **Workspace**: 工作区，包含一组相关的日志文件
- **Virtual Path**: 用户可见的逻辑路径
- **Real Path**: 文件系统中的实际物理路径

## Requirements

### Requirement 1

**User Story:** 作为用户，我希望导入压缩包后能够立即搜索其中的日志内容，以便快速定位问题。

#### Acceptance Criteria

1. WHEN 用户导入压缩包文件 THEN System SHALL 将压缩包解压到持久化的工作区目录
2. WHEN 解压完成后 THEN System SHALL 将所有解压文件的真实路径记录到 Path Map 中
3. WHEN 构建索引时 THEN System SHALL 使用解压后的真实文件路径作为 Path Map 的键
4. WHEN 用户执行搜索操作 THEN System SHALL 能够通过 Path Map 访问解压后的文件并返回搜索结果
5. WHEN 搜索引擎打开文件时 THEN System SHALL 验证文件路径存在且可访问

### Requirement 2

**User Story:** 作为系统架构师，我希望使用业内成熟的文件索引模式，以确保系统的可维护性和可靠性。

#### Acceptance Criteria

1. WHEN 设计文件索引系统时 THEN System SHALL 采用 Lucene/Tantivy 的文档索引模式
2. WHEN 管理文件路径时 THEN System SHALL 使用规范化的绝对路径作为文件标识符
3. WHEN 处理临时文件时 THEN System SHALL 遵循操作系统的临时文件管理最佳实践
4. WHEN 构建路径映射时 THEN System SHALL 使用不可变的路径标识符避免竞态条件
5. WHEN 验证文件访问时 THEN System SHALL 在索引构建和搜索执行时都进行路径有效性检查

### Requirement 3

**User Story:** 作为开发者，我希望系统能够清晰地记录文件处理过程，以便快速诊断和修复问题。

#### Acceptance Criteria

1. WHEN 解压文件时 THEN System SHALL 记录每个解压文件的路径和状态
2. WHEN 构建索引时 THEN System SHALL 记录添加到 Path Map 的每个条目
3. WHEN 搜索失败时 THEN System SHALL 记录无法访问的文件路径和失败原因
4. WHEN 文件路径无效时 THEN System SHALL 发出警告并跳过该文件
5. WHEN 导入完成时 THEN System SHALL 提供索引统计信息（文件总数、成功数、失败数）

### Requirement 4

**User Story:** 作为用户，我希望系统能够处理嵌套压缩包，以便分析复杂的日志归档结构。

#### Acceptance Criteria

1. WHEN 压缩包内包含其他压缩包时 THEN System SHALL 递归解压所有嵌套的压缩包
2. WHEN 构建虚拟路径时 THEN System SHALL 保持嵌套结构的层次关系
3. WHEN 解压嵌套压缩包时 THEN System SHALL 为每层压缩包创建独立的解压目录
4. WHEN 所有嵌套解压完成后 THEN System SHALL 确保所有文件都可通过 Path Map 访问
5. WHEN 嵌套层级超过限制时 THEN System SHALL 停止递归并记录警告信息

### Requirement 5

**User Story:** 作为系统管理员，我希望解压后的文件能够被正确清理，以避免磁盘空间浪费。

#### Acceptance Criteria

1. WHEN 工作区被删除时 THEN System SHALL 删除该工作区的所有解压文件
2. WHEN 删除解压目录失败时 THEN System SHALL 将其加入清理队列稍后重试
3. WHEN 应用启动时 THEN System SHALL 检查并清理孤立的解压目录
4. WHEN 解压目录被占用时 THEN System SHALL 使用重试机制尝试删除
5. WHEN 清理失败超过阈值时 THEN System SHALL 记录错误并通知用户

### Requirement 6

**User Story:** 作为用户，我希望系统能够处理大型压缩包，而不会导致内存溢出或性能下降。

#### Acceptance Criteria

1. WHEN 解压大型压缩包时 THEN System SHALL 使用流式处理避免一次性加载所有内容到内存
2. WHEN 构建索引时 THEN System SHALL 分批处理文件避免内存峰值
3. WHEN 文件数量超过阈值时 THEN System SHALL 显示进度信息
4. WHEN 单个文件超过大小限制时 THEN System SHALL 跳过该文件并记录警告
5. WHEN 总解压大小超过限制时 THEN System SHALL 停止解压并提示用户

### Requirement 7

**User Story:** 作为开发者，我希望使用成熟的路径规范化库，以确保跨平台兼容性。

#### Acceptance Criteria

1. WHEN 处理文件路径时 THEN System SHALL 使用 Rust 标准库的 Path 和 PathBuf 类型
2. WHEN 规范化路径时 THEN System SHALL 使用 canonicalize 方法获取绝对路径
3. WHEN 比较路径时 THEN System SHALL 使用规范化后的路径避免大小写和分隔符差异
4. WHEN 存储路径时 THEN System SHALL 使用平台无关的路径分隔符
5. WHEN 显示路径时 THEN System SHALL 转换为用户友好的格式

### Requirement 8

**User Story:** 作为质量保证工程师，我希望系统具有完善的错误处理机制，以确保单个文件失败不影响整体导入。

#### Acceptance Criteria

1. WHEN 单个文件解压失败时 THEN System SHALL 记录错误并继续处理其他文件
2. WHEN 文件路径无效时 THEN System SHALL 跳过该文件并记录警告
3. WHEN 文件无法访问时 THEN System SHALL 提供详细的错误信息（权限、不存在等）
4. WHEN 导入过程中发生错误时 THEN System SHALL 保持已处理文件的索引可用
5. WHEN 关键错误发生时 THEN System SHALL 回滚部分操作并通知用户

## Testing Strategy

本规范将采用以下测试策略：

1. **单元测试**: 测试路径规范化、文件验证等核心功能
2. **集成测试**: 测试解压、索引构建、搜索的完整流程
3. **属性测试**: 验证路径映射的一致性和完整性
4. **端到端测试**: 模拟真实用户场景，从导入到搜索的完整流程

## References

- **Lucene/Tantivy**: 成熟的全文搜索引擎，提供文档索引模式
- **Rust std::path**: Rust 标准库的路径处理模块
- **ZIP/RAR 规范**: 压缩文件格式的官方规范
- **POSIX 文件系统**: 文件系统操作的标准规范
