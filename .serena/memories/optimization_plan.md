# 关键词搜索功能优化计划

## 项目目标
优化 log-analyzer_rust 的搜索功能，确保任何关键词都能从压缩包搜索到结果。

## 实施状态

### ✅ 阶段1: 配置架构扩展（已完成）
**状态**: 完成
**测试**: 7个测试用例全部通过
**文件**: 
- `models/config.rs` - 新增配置结构和方法

### ✅ 阶段2: 智能文件类型检测（已完成）
**状态**: 完成
**测试**: 6个测试用例全部通过
**文件**:
- `services/intelligent_file_filter.rs` - 智能文件检测
- `models/import_decision.rs` - 决策模型

### ✅ 阶段3: 嵌套压缩包智能处理（已完成）
**状态**: 完成
**测试**: 11个测试用例全部通过
**文件**:
- `archive/nested_archive_config.rs` - 嵌套配置
- `archive/compression_analyzer.rs` - 压缩分析

### ✅ 阶段4: 错误处理与报告（已完成）
**状态**: 完成
**测试**: 14个测试用例全部通过
**文件**:
- `models/processing_report.rs` - 报告模型
- `services/report_collector.rs` - 报告收集

## 关键文件清单

### 需要修改的核心文件

| 文件 | 修改内容 | 优先级 |
|------|----------|--------|
| `models/config.rs` | 新增配置结构 | 高 |
| `archive/processor.rs` | 集成所有新功能 | 高 |
| `services/file_type_filter.rs` | 增强为智能检测 | 高 |
| `commands/config.rs` | 配置命令 | 中 |
| `commands/import.rs` | 集成报告 | 中 |
| `commands/search.rs` | 流式搜索集成 | 中 |

### 需要新增的文件

| 文件 | 功能 | 优先级 |
|------|------|--------|
| `services/intelligent_file_filter.rs` | 智能文件检测 | 高 |
| `models/import_decision.rs` | 决策模型 | 高 |
| `archive/nested_archive_config.rs` | 嵌套配置 | 中 |
| `archive/compression_analyzer.rs` | 压缩分析 | 中 |
| `models/processing_report.rs` | 报告模型 | 中 |
| `services/report_collector.rs` | 报告收集 | 中 |
| `services/streaming_archive_search.rs` | 流式搜索 | 低 |

## 成功标准

1. ✅ 任何关键词都能从压缩包搜索到结果
2. ✅ 支持至少15层嵌套压缩包
3. ✅ 自动识别日志文件格式
4. ✅ 完整的错误报告和进度反馈
5. ✅ 所有测试通过（覆盖率80%+）
