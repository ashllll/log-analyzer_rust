<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# services (服务层)

## Purpose
API调用封装和业务逻辑服务，统一前端与Rust后端的交互。

## Key Files

| File | Description |
|------|-------------|
| `api.ts` | Tauri invoke 统一入口 |
| `errors.ts` | 错误类型定义和处理 |
| `errorService.ts` | i18n错误消息服务 |

## For AI Agents

### Working In This Directory
- API函数返回 Promise，使用 async/await
- 错误统一处理，转换为前端错误类型
- 请求参数使用 Zod 验证

### Testing Requirements
- 使用 jest.mock 模拟 Tauri API
- 测试正常和错误场景

### Common Patterns
- 使用 Zod 进行运行时验证
- 错误码映射到 i18n 键
- 超时控制使用 AbortController

## Dependencies

### Internal
- `types/` - API类型定义
- `schemas/` - Zod验证schema
- `i18n/` - 国际化

### External
- **@tauri-apps/api** - Tauri调用
- **zod** - 运行时验证

<!-- MANUAL: -->
