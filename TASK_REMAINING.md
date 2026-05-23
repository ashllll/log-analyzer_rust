# 未完成任务清单

> 生成日期：2026-05-20 | 基于 Phase 1-3 完整重构后的剩余工作

---

## 一、CI/CD

- [x] **CI-01** `cargo fmt --check` CI 持续失败 — 已修复 Rust 格式差异，CI 恢复为严格 `cargo fmt -- --check`；补充 `.gitattributes` 约束 YAML/脚本 LF 行尾
- [ ] **CI-02** 测试覆盖率追踪 — 未接入 codecov / cargo-tarpaulin

---

## 二、Clean Architecture — 用例接入（骨架已建）

- [ ] **UC-01** SearchUseCase 完整接入 — 已写 + 集成测试通过；已依赖 `LogSearcher` domain trait 和 `QueryEngineLogSearcher` 生产适配器；Tauri 入口已迁到 `interfaces/search.rs`，当前仍委托 `commands/search/mod.rs::search_logs_impl`，待补齐事件协议后切换到 UseCase
- [ ] **UC-02** ImportUseCase 完整接入 — 骨架已写，标注 TODO(p3)；Tauri 入口已迁到 `interfaces/import.rs`，当前仍委托 `commands/import.rs::import_folder_impl`
- [x] **UC-03** ExportUseCase 接入 — 已接入 `commands/export.rs`
- [x] **UC-04** WorkspaceUseCase 创建并接入基础查询能力 — 已新增 `WorkspaceUseCase`，覆盖 list/get/delete 基础编排并通过单元测试
- [x] **UC-05** ConfigUseCase 创建并接入基础配置能力 — 已新增 `ConfigUseCase`，覆盖 load/save 与配置验证，并通过单元测试
- [ ] **UC-06** WatchUseCase 创建并接入基础监听能力

### Domain Trait / Adapter 实现进度

- [x] **AD-01** `EventPublisher` — 有 `TauriEventPublisher` 适配器
- [x] **AD-02** `LogFileRepository` — 有 `CasLogFileRepository` 适配器
- [x] **AD-03** `SearchResultRepository` — 有 `DiskResultStoreRepo` 适配器
- [x] **AD-04** `LogSearcher` — 有 `QueryEngineLogSearcher` 适配器，复用现有 QueryPlanBuilder/RegexEngine
- [x] **AD-05** `WorkspaceRepository` — 已新增 `RuntimeWorkspaceRepository` 适配器，覆盖 list/get/delete 运行态工作区能力
- [ ] **AD-06** `ArchiveExtractor` — 未创建，ImportUseCase 阻塞项
- [ ] **AD-07** `TaskScheduler` — 未创建，ImportUseCase 阻塞项

### interfaces/ — 命令迁移

- [x] **IF-01** `search_logs` 迁到 `interfaces/search.rs` — Tauri 入口已迁移；内部暂委托 legacy 执行引擎
- [x] **IF-02** `import_folder` 迁到 `interfaces/import.rs` — Tauri 入口已迁移；内部暂委托 legacy 导入流程
- [x] **IF-03** `export_results` 已接入导出用例
- [ ] **IF-04** workspace 命令迁移到 `interfaces/workspace.rs`
- [x] **IF-05** config 命令迁移到 `interfaces/config.rs` — 已接入 `ConfigUseCase`，legacy 配置命令取消 Tauri 暴露
- [x] **IF-06** watch 命令迁移到 `interfaces/watch.rs` — Tauri 入口已迁移；内部暂委托 legacy 监听流程
- [ ] **IF-07** virtual_tree 命令迁移到 `interfaces/virtual_tree.rs`
- [ ] **IF-08** log_config 命令迁移到 `interfaces/log_config.rs`
- [ ] **IF-09** validation 命令迁移到 `interfaces/validation.rs`
- [x] **IF-10** state_sync 命令迁移到 `interfaces/state_sync.rs` — Tauri 入口已迁移；内部暂委托 legacy 状态同步初始化流程

---

## 三、大文件拆分

- [ ] **RF-01** 拆分 `la-core/src/models/config.rs` — 按子领域拆到 `config/` 目录下 models / loader / validator
- [ ] **RF-02** 拆分 `la-storage/src/metadata_store.rs` — 按查询 / 迁移 / FTS 拆

---

## 四、Phase 3

- [ ] **P3-01** 插件化归档格式 — `extraction_policy.toml` `[handlers]` 段已加，但 `ArchiveManager` 未读取该配置
- [ ] **P3-02** 分布式/远程工作区 — 已出评估文档 `docs/architecture/DISTRIBUTED_WORKSPACE_ASSESSMENT.md`，决策暂缓
- [x] **P3-03** LogSearcher 完整实现 — 已有 `QueryEngineLogSearcher` 生产适配器，替换 `match_content()` 空实现

---

## 五、统计

| 类别 | 完成 | 未完成 |
|------|------|--------|
| Domain trait | 7 | 0 |
| Adapter | 5 | 0 |
| UseCase | 5 (Export已接入) | 3 |
| 命令接入 | 6 | 23 |
| CI 修复 | 5 | 0 |
| 大文件拆分 | 2 (测试提取) | 2 |
| Phase 3 功能 | 3 (搜索拆分/混合/流式) | 3 |

**建议优先级**：① SearchUseCase 接入 → ② ImportUseCase 接入 → ③ 大文件剩余拆分 → ④ 覆盖率追踪
