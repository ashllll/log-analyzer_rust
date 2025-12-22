# 搜索逻辑快速参考

## 核心规则

```
| 是分隔符，不是逻辑操作符
多个关键词 = AND 逻辑（全部必须匹配）
```

---

## 搜索示例

### 单个关键词
```
搜索: error
结果: 包含 "error" 的日志
```

### 两个关键词（AND）
```
搜索: error | timeout
结果: 同时包含 "error" 和 "timeout" 的日志
     ✓ "ERROR: timeout occurred"
     ✗ "ERROR: invalid input"
     ✗ "timeout in cache"
```

### 三个关键词（AND）
```
搜索: error | database | constraint
结果: 同时包含全部三个的日志
     ✓ "ERROR: database constraint failed"
     ✗ "ERROR: database connected"
     ✗ "constraint validation error"
```

---

## 正则表达式对比

| 搜索 | OR 逻辑 | AND 逻辑 |
|------|--------|---------|
| `error` | `(?i)error` | `(?i)error` |
| `error \| timeout` | `(?i)(error\|timeout)` | `(?i)(?=.*error)(?=.*timeout).*` |
| `a \| b \| c` | `(?i)(a\|b\|c)` | `(?i)(?=.*a)(?=.*b)(?=.*c).*` |

---

## UI 提示

| 元素 | 内容 |
|------|------|
| 搜索框占位符 | "Search keywords separated by \| (AND logic)..." |
| 活跃关键词标签 | "Active (AND):" |
| 分隔符 | `\|` |

---

## 常见场景

### 场景 1：从结果中进一步过滤
```
1. 搜索: error           (100 个结果)
2. 添加: | database      (30 个结果)
3. 添加: | constraint    (5 个结果)
```

### 场景 2：处理异常日志
```
搜索: EXCEPTION | null | pointer
结果: 空指针异常的相关日志
```

### 场景 3：性能分析
```
搜索: slow | query | seconds
结果: 慢查询的日志记录
```

---

## 快捷操作

| 操作 | 方法 |
|------|------|
| 添加关键词 | 搜索框输入 + `\|` + 新关键词 |
| 移除关键词 | 点击 Filters 面板中已激活的关键词 |
| 清空所有 | 清空搜索框 |
| 快速切换 | 使用 Filters 面板中的关键词按钮 |

---

## 性能指标

| 关键词数 | 预期耗时 |
|---------|---------|
| 1 个 | ~50ms |
| 2 个 | ~100ms |
| 3 个 | ~150ms |
| 5+ 个 | ~300-500ms |

（基于 100 万行日志）

---

## 常见问题

**Q: 如何实现 OR 逻辑？**  
A: 分别执行多次搜索

**Q: 关键词顺序重要吗？**  
A: 不重要，结果相同

**Q: 支持正则表达式吗？**  
A: 输入被视为字面量，特殊字符被转义

**Q: 可以排除关键词吗？**  
A: 当前不支持，可在预置关键词中配置

---

## 修改历史

**版本 2.0（当前）：**
- ✨ AND 逻辑搜索
- ✨ 前向断言优化
- ✨ UI 标签更新

**版本 1.0：**
- 基础 OR 逻辑
- 简单关键词搜索
