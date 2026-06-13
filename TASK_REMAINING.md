# 未完成任务清单

> 生成日期：2026-05-20 | 更新日期：2026-05-23 | 基于 Phase 1-3 完整重构后的剩余工作
>
> **5/23 更新**: commit `19709fa` 完成了 RF-01（config 模块拆分）和 IF-04/07/08/09（命令注册迁移），共 5 项由 `[ ]` → `[x]`。

---

## 一、CI/CD

- [x] **CI-01** `cargo fmt --check` CI 持续失败 — 已修复 Rust 格式差异，CI 恢复为严格 `cargo fmt -- --check`；补充 `.gitattributes` 约束 YAML/脚本 LF 行尾
- [x] **CI-02** 测试覆盖率追踪 — 已配置 cargo-tarpaulin + Codecov workflow；`tarpaulin.toml` 启用 `--all-features` 与 CI 测试对齐；`scripts/validate-ci.sh` 增加本地覆盖率检查（可选）

---

## 二、Clean Architecture — 用例接入（骨架已建）

- [x] **UC-01** SearchUseCase 完整接入 — `interfaces/search.rs` 构造 SearchUseCase + 适配器直接执行搜索；移除 search_logs_impl 委托；搜索测试全通过
- [x] **UC-02** ImportUseCase 接入 — 已接入 `ArchiveExtractor` + `TaskScheduler` trait，实现 `execute()` 编排逻辑（创建→验证→扫描→提取→完成/失败），含 3 个单元测试。`interfaces/import.rs` 标记 TODO(UC-02) 待 CAS/MetadataStore trait 化后完整接线。
- [x] **UC-03** ExportUseCase 接入 — 已接入 `commands/export.rs`
- [x] **UC-04** WorkspaceUseCase 创建并接入基础查询能力 — 已新增 `WorkspaceUseCase`，覆盖 list/get/delete 基础编排并通过单元测试
- [x] **UC-05** ConfigUseCase 创建并接入基础配置能力 — 已新增 `ConfigUseCase`，覆盖 load/save 与配置验证，并通过单元测试
- [x] **UC-06** WatchUseCase 创建并接入基础监听能力 — `WatchUseCase<E,C,M>` 完整实现：notify watcher + 增量读取 + CAS 写入 + metadata 更新 + EventPublisher 事件；`commands/watch.rs` 精简为 ~180 行胶水层（含 `WatchEventAdapter` 处理搜索索引）；4 个单元测试

### Domain Trait / Adapter 实现进度

- [x] **AD-01** `EventPublisher` — 有 `TauriEventPublisher` 适配器
- [x] **AD-02** `LogFileRepository` — 有 `CasLogFileRepository` 适配器
- [x] **AD-03** `SearchResultRepository` — 有 `DiskResultStoreRepo` 适配器
- [x] **AD-04** `LogSearcher` — 有 `QueryEngineLogSearcher` 适配器，复用现有 QueryPlanBuilder/RegexEngine
- [x] **AD-05** `WorkspaceRepository` — 已新增 `RuntimeWorkspaceRepository` 适配器，覆盖 list/get/delete 运行态工作区能力
- [x] **AD-06** `ArchiveExtractor` — 已创建 `domain/extract.rs`，含 extract/list/supported_formats/validate；适配器 `ArchiveManagerAdapter`
- [x] **AD-07** `TaskScheduler` — 已创建 `domain/task.rs`，含 create/update/complete/fail/cancel；适配器 `TaskManagerAdapter`

### interfaces/ — 命令迁移

- [x] **IF-01** `search_logs` 迁到 `interfaces/search.rs` — Tauri 入口已迁移；内部暂委托 legacy 执行引擎
- [x] **IF-02** `import_folder` 迁到 `interfaces/import.rs` — Tauri 入口已迁移；内部暂委托 legacy 导入流程
- [x] **IF-03** `export_results` 已接入导出用例
- [x] **IF-04** workspace 命令迁移到 `interfaces/workspace.rs` — Tauri 入口已迁移；内部暂委托 legacy workspace 流程
- [x] **IF-05** config 命令迁移到 `interfaces/config.rs` — 已接入 `ConfigUseCase`，legacy 配置命令取消 Tauri 暴露
- [x] **IF-06** watch 命令迁移到 `interfaces/watch.rs` — Tauri 入口已迁移；内部暂委托 legacy 监听流程
- [x] **IF-07** virtual_tree 命令迁移到 `interfaces/virtual_tree.rs` — Tauri 入口已迁移；内部暂委托 legacy 虚拟文件树流程
- [x] **IF-08** log_config 命令迁移到 `interfaces/log_config.rs` — Tauri 入口已迁移；内部暂委托 legacy 日志配置流程
- [x] **IF-09** validation 命令迁移到 `interfaces/validation.rs` — Tauri 入口已迁移；内部暂委托 legacy 校验流程
- [x] **IF-10** state_sync 命令迁移到 `interfaces/state_sync.rs` — Tauri 入口已迁移；内部暂委托 legacy 状态同步初始化流程

---

## 三、大文件拆分

- [x] **RF-01** 拆分 `la-core/src/models/config.rs` — 已拆为 `config/` 子模块目录 (models.rs / loader.rs / validator.rs / mod.rs)，原文件 2620 行已删除
- [x] **RF-02** 拆分 `la-storage/src/metadata_store.rs` — 拆为 `metadata_store/` 目录 6 个子模块：`types.rs` (类型)、`schema.rs` (表初始化+迁移)、`file_ops.rs` (文件 CRUD+FTS+批处理)、`archive_ops.rs` (归档 CRUD)、`index_ops.rs` (增量索引)、`mod.rs` (facade+委托)。公开 API 不变，185 测试全通过

---

## 四、Phase 3

- [x] **P3-01** 插件化归档格式 — `HandlersConfig` 建模 + TOML 反序列化 + `ArchiveManager::with_handlers_config()` + `ArchiveManagerAdapter::with_handlers_config()` 完整接线；`[handlers]` 段可运行时关闭任意格式处理器
- [x] **P3-02** 分布式/远程工作区 — 评估完成，决策 DEFER。`docs/architecture/DISTRIBUTED_WORKSPACE_ASSESSMENT.md` 详细分析了三种实现路径（共享注册表/远程访问/全分布式同步），结论：当前无用户需求，架构适配代价高，建议在有明确需求时基于现有 trait 抽象增量构建而不修改 domain 层
- [x] **P3-03** LogSearcher 完整实现 — 已有 `QueryEngineLogSearcher` 生产适配器，替换 `match_content()` 空实现

---

## 五、统计

| 类别 | 完成 | 未完成 | 备注 |
|------|------|--------|------|
| Domain trait | 7 | 0 | 全部完成 ✅ |
| Adapter | 7 | 0 | 全部有实现 ✅ |
| UseCase | 6 | 0 | UC-01~06 全部完成 ✅ |
| 命令接入 (interfaces/) | 10 | 0 | 全部迁移至 interfaces/ |
| CI | 2 | 0 | CI-01 修复；CI-02 已配置 tarpaulin + Codecov workflow ✅ |
| 大文件拆分 | 2 | 0 | RF-01 + RF-02 全部完成 ✅ |
| Phase 3 | 3 | 0 | P3-01/02/03 全部完成 ✅ |

**建议优先级**：① AD-06/07 (ArchiveExtractor + TaskScheduler) → ② UC-02 (ImportUseCase) → ③ UC-01 (SearchUseCase) → ④ RF-02 (metadata_store 拆分)
