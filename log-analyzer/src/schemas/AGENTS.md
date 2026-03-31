<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# schemas (Zod验证Schema)

## Purpose
Zod运行时验证Schema，用于API参数和表单验证。

## Key Files

| File | Description |
|------|-------------|
| `search.ts` | 搜索参数验证 |
| `workspace.ts` | 工作区验证 |
| `config.ts` | 配置验证 |

## For AI Agents

### Working In This Directory
- Schema与TypeScript类型对应
- 提供清晰的错误消息
- 支持i18n错误码

### Common Patterns
- 使用 .min(), .max() 限制范围
- 使用 .email(), .url() 验证格式
- 复杂对象使用 .shape()

## Dependencies

### External
- **zod** - 运行时验证

<!-- MANUAL: -->
