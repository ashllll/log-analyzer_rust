# Requirements Document - Complete CAS Migration

## Introduction

本规范旨在完成从旧的 path_map 系统到成熟的 Content-Addressable Storage (CAS) 架构的完全迁移。当前系统虽然已实现 CAS 功能，但仍保留了大量旧代码和过渡性代码，导致系统复杂度高、维护困难。

本规范将彻底移除所有旧功能代码，确保系统100%使用业内成熟的 CAS + SQLite 元数据存储方案，不再支持旧格式的向后兼容。

## Glossary

- **System**: 日志分析器应用程序
- **CAS**: Content-Addressable Storage，基于内容哈希的存储系统
- **MetadataStore**: SQLite 数据库，存储文件元数据和虚拟路径映射
- **Legacy Code**: 旧的 path_map 相关代码
- **Migration Code**: 用于从旧格式迁移到新格式的临时代码
- **Index Store**: 旧的索引存储系统（使用 bincode 序列化）
- **Path Mappings Table**: 旧的 SQLite 路径映射表

## Requirements

### Requirement 1

**User Story:** 作为开发者，我希望完全移除旧的 path_map 系统，以降低代码复杂度和维护成本。

#### Acceptance Criteria

1. WHEN 搜索代码库时 THEN System SHALL 不包含任何 `path_map` 或 `PathMap` 的引用（除了文档和注释）
2. WHEN 编译系统时 THEN System SHALL 不包含 `index_store.rs` 模块
3. WHEN 查看数据库 schema 时 THEN System SHALL 不包含 `path_mappings` 表
4. WHEN 检查依赖时 THEN System SHALL 不包含仅用于旧系统的依赖（如 bincode 用于索引序列化）
5. WHEN 运行所有测试时 THEN System SHALL 所有测试通过且不依赖旧代码

### Requirement 2

**User Story:** 作为系统架构师，我希望所有功能都使用 CAS + MetadataStore 架构，确保系统一致性。

#### Acceptance Criteria

1. WHEN 导入文件夹时 THEN System SHALL 使用 CAS 存储所有文件内容
2. WHEN 导入压缩包时 THEN System SHALL 使用 CAS 存储所有解压文件
3. WHEN 执行搜索时 THEN System SHALL 通过 MetadataStore 查询文件并从 CAS 读取内容
4. WHEN 显示文件树时 THEN System SHALL 从 MetadataStore 构建虚拟文件树
5. WHEN 删除工作区时 THEN System SHALL 清理 CAS 对象和 MetadataStore 记录

### Requirement 3

**User Story:** 作为开发者，我希望移除所有迁移相关代码，因为不再需要支持旧格式。

#### Acceptance Criteria

1. WHEN 搜索代码库时 THEN System SHALL 不包含 `migration` 模块
2. WHEN 启动应用时 THEN System SHALL 不检测或加载旧格式工作区
3. WHEN 用户打开工作区时 THEN System SHALL 只支持 CAS 格式
4. WHEN 发现旧格式工作区时 THEN System SHALL 提示用户该格式不再支持
5. WHEN 编译系统时 THEN System SHALL 不包含任何迁移相关代码

### Requirement 4

**User Story:** 作为质量保证工程师，我希望清理所有测试代码中的旧系统引用，确保测试覆盖新架构。

#### Acceptance Criteria

1. WHEN 运行测试时 THEN System SHALL 所有测试使用 CAS + MetadataStore
2. WHEN 检查测试代码时 THEN System SHALL 不包含 `create_traditional_workspace` 等旧测试辅助函数
3. WHEN 运行集成测试时 THEN System SHALL 测试完整的 CAS 工作流
4. WHEN 运行属性测试时 THEN System SHALL 验证 CAS 系统的正确性属性
5. WHEN 所有测试通过时 THEN System SHALL 确保新架构功能完整

### Requirement 5

**User Story:** 作为用户，我希望系统性能优异且稳定，不受旧代码影响。

#### Acceptance Criteria

1. WHEN 导入大型压缩包时 THEN System SHALL 使用 CAS 去重减少存储空间
2. WHEN 搜索文件时 THEN System SHALL 使用 SQLite FTS5 提供快速全文搜索
3. WHEN 处理嵌套压缩包时 THEN System SHALL 正确处理任意深度的嵌套结构
4. WHEN 系统运行时 THEN System SHALL 内存使用稳定不增长
5. WHEN 执行操作时 THEN System SHALL 响应时间符合性能要求

### Requirement 6

**User Story:** 作为开发者，我希望代码库干净整洁，易于理解和维护。

#### Acceptance Criteria

1. WHEN 阅读代码时 THEN System SHALL 代码结构清晰，模块职责明确
2. WHEN 查看文件时 THEN System SHALL 不包含注释掉的旧代码
3. WHEN 检查导入时 THEN System SHALL 不包含未使用的导入语句
4. WHEN 运行 linter 时 THEN System SHALL 没有警告或错误
5. WHEN 查看文档时 THEN System SHALL 文档反映当前架构，不包含旧系统描述

### Requirement 7

**User Story:** 作为系统管理员，我希望清理旧的数据库表和文件，释放存储空间。

#### Acceptance Criteria

1. WHEN 检查数据库 schema 时 THEN System SHALL 只包含 CAS 相关的表
2. WHEN 查看工作区目录时 THEN System SHALL 不包含旧的 `.idx.gz` 索引文件
3. WHEN 系统启动时 THEN System SHALL 自动清理遗留的旧格式文件
4. WHEN 删除工作区时 THEN System SHALL 完全清理所有相关数据
5. WHEN 检查磁盘使用时 THEN System SHALL 存储效率优于旧系统

### Requirement 8

**User Story:** 作为开发者，我希望更新所有 API 和命令，确保接口一致性。

#### Acceptance Criteria

1. WHEN 调用 Tauri 命令时 THEN System SHALL 所有命令使用 CAS 架构
2. WHEN 前端请求数据时 THEN System SHALL 返回基于 CAS 的数据结构
3. WHEN 查看 API 文档时 THEN System SHALL 文档描述 CAS 架构的接口
4. WHEN 使用 WebSocket 时 THEN System SHALL 事件数据基于 CAS 架构
5. WHEN 导出数据时 THEN System SHALL 使用 CAS 格式

## Testing Strategy

本规范将采用以下测试策略：

1. **代码搜索验证**: 使用 grep 搜索确保没有旧代码残留
2. **编译验证**: 确保移除旧代码后系统仍能编译
3. **单元测试**: 验证所有模块使用 CAS 架构
4. **集成测试**: 验证完整的导入-搜索-删除流程
5. **性能测试**: 确保新架构性能优于旧系统
6. **回归测试**: 确保所有功能正常工作

## Success Criteria

迁移成功的标准：

1. ✅ 代码库中没有 `path_map` 相关代码（除文档）
2. ✅ 所有测试通过
3. ✅ 性能测试达标
4. ✅ 代码 linter 无警告
5. ✅ 文档更新完成
6. ✅ 用户可以正常使用所有功能

## References

- **Git Object Storage**: Git 的 CAS 实现是业内标准
- **SQLite FTS5**: 成熟的全文搜索引擎
- **Rust std::fs**: 标准文件系统操作
- **SHA-256**: 业内标准的哈希算法
