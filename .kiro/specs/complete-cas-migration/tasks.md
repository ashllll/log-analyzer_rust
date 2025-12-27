# Implementation Plan - Complete CAS Migration

## Phase 1: 代码审计和准备

- [x] 1. 完整代码审计





  - 列出所有使用 `path_map` 的文件
  - 列出所有使用 `load_index` / `save_index` 的文件
  - 列出所有使用 `index_store` 的文件
  - 列出所有使用 `migration` 的文件
  - 创建详细的修改清单
  - _Requirements: 1.1, 6.1_


- [x] 2. 创建备份分支




  - 创建 `backup/pre-cas-migration` 分支
  - 确保所有现有测试通过
  - 记录当前性能基线
  - _Requirements: 6.1_


- [x] 3. 运行基线测试




  - 运行所有单元测试
  - 运行所有集成测试
  - 记录测试结果
  - 记录性能指标
  - _Requirements: 4.4_

## Phase 2: 移除旧代码文件


- [x] 4. 删除旧的索引系统文件




  - 删除 `src-tauri/src/services/index_store.rs`
  - 删除 `src-tauri/src/services/metadata_db.rs`
  - 更新 `src-tauri/src/services/mod.rs` 移除导出
  - _Requirements: 1.2, 1.3_

- [x] 5. 删除迁移相关文件





  - 删除 `src-tauri/src/migration/mod.rs`
  - 删除 `src-tauri/src/commands/migration.rs`
  - 删除 `src-tauri/tests/migration_tests.rs`
  - 更新 `src-tauri/src/lib.rs` 移除 migration 模块
  - _Requirements: 3.1, 3.2, 3.3_


- [x] 6. 删除临时文件




  - 删除根目录的 `temp_lib.rs`
  - 验证没有其他临时文件
  - _Requirements: 1.1_


- [x] 7. 删除前端迁移组件




  - 删除 `src/components/MigrationDialog.tsx`
  - 删除 `src/hooks/useMigration.ts`
  - _Requirements: 3.4_

## Phase 3: 更新数据模型

- [x] 8. 清理 models/config.rs





  - 删除 `IndexData` 结构体
  - 删除旧的 `FileMetadata` 结构体（如果只用于旧系统）
  - 验证没有其他代码引用这些结构体
  - _Requirements: 1.1, 6.2_


- [x] 9. 清理 models/state.rs




  - 删除 `PathMapType` 类型别名
  - 删除 `MetadataMapType` 类型别名
  - 删除 `IndexResult` 类型别名
  - 验证 AppState 只包含 CAS 相关字段
  - _Requirements: 1.1, 6.2_


- [x] 10. 更新前端类型定义




  - 从 `workspaceStore.ts` 移除 `format` 字段
  - 从 `workspaceStore.ts` 移除 `needsMigration` 字段
  - 更新相关类型定义
  - _Requirements: 8.2_

## Phase 4: 修复编译错误（核心迁移）

- [x] 11. 更新 commands/import.rs





  - 移除 `use crate::services::save_index;`
  - 移除对 `state.path_map` 的访问
  - 移除对 `state.file_metadata` 的访问
  - 移除 `save_index()` 调用
  - 确保使用 `MetadataStore::insert_file()`
  - 添加导入完成后的验证逻辑
  - _Requirements: 2.1, 2.2, 8.1_


- [x] 12. 更新 commands/workspace.rs




  - 移除 `use crate::services::{load_index, save_index};`
  - 替换 `load_index()` 为 `MetadataStore::get_all_files()`
  - 移除所有 `save_index()` 调用
  - 更新所有使用 path_map 的逻辑
  - 使用 CAS 读取文件内容
  - _Requirements: 2.1, 2.3, 8.1_


- [x] 13. 更新 commands/async_search.rs




  - 移除 `path_map` 参数
  - 添加 `workspace_id` 参数
  - 使用 `MetadataStore` 获取文件列表
  - 使用 `CAS` 读取文件内容
  - 更新所有调用此函数的地方
  - _Requirements: 2.3, 8.1_


- [x] 14. 验证 commands/search.rs




  - 确认使用 `MetadataStore` 查询文件
  - 确认使用 `CAS` 读取内容
  - 确认没有使用 path_map
  - _Requirements: 2.3_


- [x] 15. 验证 archive/processor.rs




  - 确认使用 `CAS::store_file_streaming()`
  - 确认使用 `MetadataStore::insert_file()`
  - 确认没有使用 path_map
  - _Requirements: 2.1, 2.2_


- [x] 16. 编译验证




  - 运行 `cargo check`
  - 修复所有编译错误
  - 运行 `cargo build --release`
  - 确保编译成功
  - _Requirements: 6.4_

## Phase 5: 更新测试代码

- [x] 17. 移除旧测试辅助函数





  - 删除 `create_traditional_workspace_with_index`
  - 删除所有迁移相关测试辅助函数
  - _Requirements: 4.2_


- [x] 18. 创建新的 CAS 测试辅助函数




  - 创建 `create_cas_workspace` 函数
  - 创建 `populate_cas_workspace` 函数
  - 创建 `verify_cas_workspace` 函数
  - _Requirements: 4.1, 4.2_


- [x] 19. 更新单元测试




  - 更新所有使用旧测试辅助函数的测试
  - 确保测试使用 CAS + MetadataStore
  - 运行所有单元测试
  - _Requirements: 4.1, 4.3_



- [x] 20. 更新集成测试



  - 更新导入测试使用 CAS
  - 更新搜索测试使用 CAS
  - 更新工作区管理测试使用 CAS
  - 运行所有集成测试
  - _Requirements: 4.1, 4.3_

- [x] 20.1 编写属性测试：无旧代码引用








  - **Property 1: No Legacy Code References**
  - **Validates: Requirements 1.1**
  - 搜索源代码中的旧引用
  - 确保只在文档和注释中出现
  - _Requirements: 1.1_

- [x] 20.2 编写属性测试：CAS 存储一致性





















  - **Property 2: CAS Storage Consistency**
  - **Validates: Requirements 2.1, 2.2**
  - 验证所有导入的文件都在 CAS 中
  - 验证所有文件都有 MetadataStore 记录
  - _Requirements: 2.1, 2.2_




- [x] 20.3 编写属性测试：搜索使用 CAS







  - **Property 3: Search Uses CAS**
  - **Validates: Requirements 2.3**
  - 验证搜索通过 MetadataStore 查询
  - 验证搜索通过 CAS 读取内容
  - _Requirements: 2.3_

## Phase 6: 前端更新


- [x] 21. 更新 WorkspacesPage.tsx




  - 移除 `import { MigrationDialog }`
  - 移除 `import { useMigration }`
  - 移除所有迁移相关状态
  - 移除迁移横幅 UI
  - 移除迁移按钮和处理函数
  - _Requirements: 3.4, 8.2_

- [x] 22. 更新工作区类型定义










  - 从 Workspace 类型移除 `format` 字段
  - 从 Workspace 类型移除 `needsMigration` 字段
  - 更新所有使用这些字段的代码
  - _Requirements: 8.2_

- [x] 23. 前端编译验证





  - 运行 `npm run build`
  - 修复所有编译错误
  - 确保前端编译成功
  - _Requirements: 6.4_


- [x] 23.1 编写前端 E2E 测试







  - 测试导入工作流
  - 测试搜索工作流
  - 测试工作区管理
  - _Requirements: 4.4_

## Phase 7: 数据库和依赖清理

- [x] 24. 清理数据库迁移文件





  - 检查 `migrations/` 目录
  - 移除创建 `path_mappings` 表的迁移文件
  - 验证只保留 CAS 相关的迁移
  - _Requirements: 7.1_

- [x] 25. 清理依赖





  - 检查 `bincode` 是否只用于旧系统
  - 检查 `flate2` 是否只用于旧系统
  - 如果是，从 `Cargo.toml` 移除
  - 运行 `cargo build` 验证
  - _Requirements: 1.4_

- [x] 26. 添加旧格式检测和提示





  - 在应用启动时检测旧的 `.idx.gz` 文件
  - 提示用户旧格式不再支持
  - 提供创建新工作区的指引
  - _Requirements: 3.4, 7.2_

## Phase 8: 代码质量和文档

- [x] 27. 运行 linter 清理





  - 运行 `cargo clippy`
  - 修复所有警告
  - 运行 `cargo fmt`
  - _Requirements: 6.4_


- [x] 28. 移除注释掉的代码




  - 搜索并移除所有注释掉的旧代码
  - 搜索并移除所有 TODO 注释（如果已完成）
  - _Requirements: 6.3_


- [x] 29. 更新文档




  - 更新 README.md 反映 CAS 架构
  - 更新 API 文档
  - 更新架构文档
  - 移除旧系统的描述
  - _Requirements: 6.5, 8.3_


- [x] 30. 添加迁移指南




  - 创建用户指南说明旧格式不再支持
  - 提供创建新工作区的步骤
  - 说明 CAS 架构的优势
  - _Requirements: 3.4_

## Phase 9: 最终验证和性能测试

- [x] 31. 运行完整测试套件





  - 运行所有单元测试
  - 运行所有集成测试
  - 运行所有属性测试
  - 运行所有 E2E 测试
  - 确保所有测试通过
  - _Requirements: 4.4, 4.5_

- [x] 32. 性能回归测试





  - 测试导入性能
  - 测试搜索性能
  - 测试内存使用
  - 对比基线性能
  - 确保性能不退化
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 33. 手动功能测试





  - 测试导入文件夹
  - 测试导入压缩包
  - 测试嵌套压缩包
  - 测试搜索功能
  - 测试工作区管理
  - 测试文件树显示
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 34. 代码审查
  - 审查所有修改的文件
  - 确保代码质量
  - 确保没有遗漏的旧代码
  - 确保注释和文档准确
  - _Requirements: 6.1, 6.2_

## Phase 10: 发布准备

- [ ] 35. 生成变更日志
  - 列出所有移除的文件
  - 列出所有修改的文件
  - 说明架构变更
  - 说明用户影响
  - _Requirements: 6.5_

- [ ] 36. 更新版本号
  - 更新 `Cargo.toml` 版本号
  - 更新 `package.json` 版本号
  - 更新 `tauri.conf.json` 版本号
  - _Requirements: 6.5_

- [ ] 37. 最终构建
  - 运行 `cargo build --release`
  - 运行 `npm run build`
  - 测试发布版本
  - _Requirements: 6.4_

- [ ] 38. 准备发布说明
  - 说明 CAS 架构的优势
  - 说明不再支持旧格式
  - 提供迁移指引
  - 列出破坏性变更
  - _Requirements: 6.5, 8.3_

## Checkpoint Tasks

- [ ] Checkpoint 1: After Phase 2
  - 确保所有旧文件已删除
  - 记录编译错误列表
  - 询问用户是否继续

- [ ] Checkpoint 2: After Phase 4
  - 确保编译成功
  - 运行基本功能测试
  - 询问用户是否继续

- [ ] Checkpoint 3: After Phase 5
  - 确保所有测试通过
  - 验证测试覆盖率
  - 询问用户是否继续

- [ ] Checkpoint 4: After Phase 9
  - 确保所有验证通过
  - 性能达标
  - 准备发布

## Notes

**关键原则**:
1. 每个 Phase 完成后运行测试
2. 遇到问题立即停止并询问用户
3. 保持代码质量，不引入技术债
4. 确保每个修改都有测试覆盖

**风险管理**:
- 备份分支已创建，可随时回滚
- 分阶段进行，每阶段都有验证
- 保持与用户沟通，及时反馈问题

**成功标准**:
- ✅ 所有旧代码文件已删除
- ✅ 所有命令使用 CAS 架构
- ✅ 所有测试通过
- ✅ 性能不退化
- ✅ 代码质量高
- ✅ 文档完整准确
