<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# i18n (国际化)

## Purpose
i18next国际化配置，支持中英文切换。

## Key Files

| File | Description |
|------|-------------|
| `index.ts` | i18n配置初始化 |
| `config.ts` | 语言配置 |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `locales/` | 翻译文件 (see `locales/AGENTS.md`) |

## For AI Agents

### Working In This Directory
- 所有UI文本必须走i18n
- 新增文案同步添加到en.json和zh.json
- 使用命名空间组织翻译键

### Common Patterns
- 错误码映射到i18n键
- 支持动态参数插值
- 语言切换实时生效

## Dependencies

### External
- **i18next** - 国际化核心
- **react-i18next** - React集成

<!-- MANUAL: -->
