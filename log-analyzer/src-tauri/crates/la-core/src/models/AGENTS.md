<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# models (数据模型)

## Purpose
定义 la-core crate 的核心数据模型和领域类型，为其他 crates 提供共享的数据结构。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 模型模块入口，统一导出 |
| `config.rs` | 应用配置模型（87KB，主要配置定义） |
| `search.rs` | 搜索相关类型（Query、SearchTerm 等） |
| `extraction_policy.rs` | 解压策略配置 |
| `import_decision.rs` | 导入决策类型 |
| `policy_manager.rs` | 策略管理器模型 |
| `processing_report.rs` | 处理报告模型 |
| `validated.rs` | 验证类型包装器 |
| `filters.rs` | 过滤器类型 |
| `log_entry.rs` | 日志条目模型 |
| `match_detail.rs` | 匹配详情类型 |
| `metrics_state.rs` | 指标状态模型 |
| `search_statistics.rs` | 搜索统计类型 |

## For AI Agents

### Working In This Directory
- 所有模型实现 Serialize/Deserialize 用于 JSON 序列化
- 优先使用 newtype 模式增强类型安全
- 配置变更需考虑向后兼容性

### Common Patterns
- 使用 `validated.rs` 进行运行时验证
- 配置使用 builder 模式构建
- 大型配置拆分为子模块

## Dependencies

### Internal
- `la-core::traits` - 共享 trait 定义
- `la-core::utils` - 工具函数

### External
- `serde` - 序列化/反序列化
- `validator` - 运行时验证

<!-- MANUAL: -->
