# Log Analyzer v0.1.0 发布说明

## 🎉 重大更新：完成 CAS 架构迁移

Log Analyzer v0.1.0 标志着从传统 `path_map` 索引系统到 Content-Addressable Storage (CAS) 架构的重大升级。

## ✨ 新功能

### CAS 架构优势
- **更快的文件检索**: 使用内容寻址存储，文件读取性能显著提升
- **更好的内存效率**: 流式处理减少内存占用
- **数据完整性**: 内容哈希确保数据完整性
- **简化维护**: 统一的存储层简化了代码维护

### 搜索增强
- 搜索现在通过 MetadataStore 查询文件列表
- 通过 CAS 直接读取文件内容
- 改进的并行搜索性能

## 🔧 技术变更

### 移除的组件
- 旧索引系统 (`src-tauri/src/services/index_store.rs`)
- 迁移模块 (`src-tauri/src/migration/`)
- 前端迁移 UI 组件
- 临时库文件

### 更新模块
- `commands/import.rs` - 使用 MetadataStore::insert_file()
- `commands/workspace.rs` - 使用 MetadataStore::get_all_files()
- `commands/async_search.rs` - 支持 workspace_id 参数

### 数据库变更
- 移除 `path_mappings` 表
- 新增 `files` 和 `archives` 表

## ⚠️ 破坏性变更

### 不再支持旧格式
- **`.idx.gz` 索引文件**: 旧版索引格式不再支持
- **无迁移路径**: 使用旧格式创建的工作区需要重新导入

### 用户影响
1. 如果你有旧格式的工作区，需要重新导入文件
2. 旧版配置文件可能不兼容
3. 建议创建新的工作区来使用新功能

## 📦 安装更新

```bash
# 重新构建
cd log-analyzer/src-tauri
cargo build --release

cd log-analyzer
npm run build
```

## 🧪 测试

所有测试已更新以使用 CAS 架构：
- ✅ 单元测试
- ✅ 集成测试
- ✅ 属性测试 (CAS 存储一致性)
- ✅ E2E 测试

## 📚 文档

- [架构文档](docs/architecture/CAS_ARCHITECTURE.md)
- [迁移指南](docs/MIGRATION_GUIDE.md)
- [API 文档](docs/architecture/API.md)

## 🐛 已知问题

- 旧格式工作区无法打开（需要重新导入）
- 部分未使用的代码可能产生警告

## 🙏 感谢

感谢所有参与测试和反馈的用户！

## 📄 完整变更日志

请参阅 [CHANGELOG.md](../../CHANGELOG.md) 查看完整变更历史。
