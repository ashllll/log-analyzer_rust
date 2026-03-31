<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-31 | Updated: 2026-03-31 -->

# utils (工具函数)

## Purpose
提供 la-core crate 使用的通用工具函数和辅助类型。

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | 工具模块入口 |
| `path_security.rs` | 路径安全验证（18KB），防止目录遍历攻击 |
| `path.rs` | 路径处理辅助函数 |
| `validation.rs` | 通用验证函数 |

## For AI Agents

### Working In This Directory
- 路径安全验证是核心安全机制
- 工具函数需有完整单元测试
- 保持函数纯度和无副作用

### Common Patterns
- 防御性编程
- 输入验证优先
- 安全错误处理

## Dependencies

### External
- `regex` - 正则验证

<!-- MANUAL: -->
