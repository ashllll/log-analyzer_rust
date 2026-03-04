# Pitfalls Research: v1.1 高级搜索与虚拟文件系统

**Domain:** Flutter Desktop + Rust Backend 高级搜索功能集成
**Researched:** 2026-03-04
**Confidence:** MEDIUM

基于 Rust 后端 API 分析、Flutter 桌面开发经验和现有项目陷阱总结。注意：部分发现未能通过 WebSearch 验证 (工具返回错误)，基于代码分析和经验推断。

---

## Critical Pitfalls

### Pitfall 1: 正则表达式搜索 ReDoS 攻击风险

**What goes wrong:**
用户输入恶意正则表达式（如 `a+*`、`(a+){20}` 等 catastrophic backtracking 模式）导致搜索线程阻塞，CPU 占用 100%，应用无响应。

**Why it happens:**
- Rust 后端使用 `regex-automata` 库，默认未启用 timeout 或回溯限制
- Flutter 前端未验证正则表达式有效性就发送到后端
- 用户不理解正则表达式复杂度与性能的关系

**How to avoid:**
1. **前端验证**: 使用 Dart 的 `RegExp` 构造函数在发送前验证正则语法是否有效
2. **后端超时**: 在 Rust 端为正则搜索设置超时机制（使用 `regex-automata` 的 `MatchAt` 或超时检测）
3. **复杂度警告**: 识别潜在危险模式，提示用户简化

```dart
// Flutter 端预验证
bool isValidRegex(String pattern) {
  try {
    RegExp(pattern);
    return true;
  } catch (e) {
    return false;
  }
}
```

**Warning signs:**
- 搜索响应时间 > 5 秒
- CPU 占用持续 100%
- 日志中出现 "Regex search timeout"

**Phase to address:**
- Phase 搜索 UI 实现阶段 (regex 输入组件)

---

### Pitfall 2: 多关键词 AND/OR/NOT 逻辑解析错误

**What goes wrong:**
用户输入 `error AND warning OR critical` 期望 `(error AND warning) OR critical`，但实际解析为 `error AND (warning OR critical)`，导致搜索结果与预期不符。

**Why it happens:**
- 没有明确的优先级规则文档
- 用户对布尔逻辑理解不一致
- 前端解析与后端解析逻辑不一致

**How to avoid:**
1. **明确优先级**: 实现 `NOT > AND > OR` 标准布尔逻辑
2. **括号支持**: 允许用户使用括号明确分组
3. **预览确认**: 搜索前显示解析后的查询树，让用户确认

```dart
// 推荐的解析结果展示
QueryPreview(
  tokens: [
    Token(type: 'TERM', value: 'error'),
    Token(type: 'AND'),
    Token(type: 'GROUP', children: [...], operator: 'OR'),
  ],
)
```

**Warning signs:**
- 高级用户反馈搜索结果"不对"
- 测试用例 `error AND warning OR critical` 结果异常

**Phase to address:**
- Phase 布尔搜索功能实现阶段

---

### Pitfall 3: 搜索历史数据无限增长

**What goes wrong:**
搜索历史记录不断累积，未实现清理策略，导致：
- 内存占用持续增长
- 下拉列表渲染性能下降
- 存储空间浪费

**Why it happens:**
- 只有 `clear_search_history` 命令，没有自动清理
- 前端可能每次搜索都调用 `add_search_history`
- 没有限制单工作区历史条目数量

**How to avoid:**
1. **自动清理**: 实现 LRU 策略，限制单工作区历史数量（如 100 条）
2. **去重**: 相同查询不重复添加，更新时间戳
3. **过期清理**: 30 天前的历史自动删除

```rust
// Rust 后端 SearchHistoryManager 应实现
impl SearchHistoryManager {
    const MAX_ENTRIES_PER_WORKSPACE: usize = 100;
    const MAX_AGE_DAYS: u32 = 30;
}
```

**Warning signs:**
- 内存分析显示 SearchHistory 持续增长
- UI 渲染搜索历史下拉列表卡顿

**Phase to address:**
- Phase 搜索历史 UI 集成阶段

---

### Pitfall 4: 虚拟文件系统大数据集性能崩溃

**What goes wrong:**
工作区包含数千个文件时，`get_virtual_file_tree` 返回完整树结构，Flutter 端尝试渲染所有节点导致：
- UI 冻结
- 内存暴涨
- 树展开/收起操作卡顿

**Why it happens:**
- 后端返回完整递归树，一次性传输所有数据
- Flutter `TreeView` 组件默认渲染所有节点
- 未实现按需加载（lazy loading）

**How to avoid:**
1. **分层加载**: 先返回根节点，点击展开时再加载子节点
2. **虚拟滚动**: 使用 `ListView.builder` 构建扁平化树
3. **限制深度**: 初始只加载 2-3 层，避免深层嵌套

```dart
// 推荐的按需加载模式
class VirtualTreeNode {
  final String name;
  final bool isExpanded;
  final List<VirtualTreeNode>? children; // null = 未加载

  Future<void> expand() async {
    if (children == null) {
      children = await _loadChildren(); // 按需加载
    }
  }
}
```

**Warning signs:**
- 1000+ 文件时 UI 明显卡顿
- `get_virtual_file_tree` 响应时间 > 1 秒

**Phase to address:**
- Phase 虚拟文件系统 UI 实现阶段

---

### Pitfall 5: 虚拟文件树与实际文件系统状态不同步

**What goes wrong:**
用户通过其他方式（系统文件管理器）修改了工作区文件，但虚拟文件树显示的是旧数据，导致：
- 点击文件显示内容不匹配
- 用户困惑哪个是"正确"状态

**Why it happens:**
- 虚拟文件树从元数据数据库构建，不是实时读取文件系统
- 只有显式刷新才更新
- 缺少"最后更新时间"提示

**How to address:**
1. **显示时间戳**: 树节点显示"最后更新: X 分钟前"
2. **手动刷新按钮**: 提供刷新功能
3. **自动刷新**: 结合文件监听事件，变化时提示刷新

**Warning signs:**
- 用户报告"文件内容不对"
- 差异检测显示数据库与实际不一致

**Phase to address:**
- Phase 虚拟文件系统 UI 完善阶段

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|------------------|
| 不实现正则超时 | 简单快速 | ReDoS 风险，应用卡死 | 永不 (安全风险) |
| 搜索历史不做去重 | 简单快速 | 列表重复，数据冗余 | MVP 阶段 |
| 虚拟树全量加载 | UI 简单 | 性能崩溃 | 100 文件以下 |
| 忽略括号支持 | 快速实现 | 用户无法明确意图 | MVP 阶段 |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **正则搜索** | 前端不做验证直接发后端 | 先 Dart RegExp 验证用语法 |
| **布尔搜索** | 假设 AND 优先级高于 OR | 实现标准 NOT>AND>OR 或支持括号 |
| **搜索历史** | 每次搜索都添加 | 去重 + LRU 限制 |
| **虚拟文件树** | 一次性加载全部 | 按需展开 + 虚拟滚动 |
| **文件读取** | 读取大文件到内存 | 流式读取 + 分页 |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| 复杂正则搜索 | UI 冻结 10s+ | 前端复杂度检查 + 后端超时 | 恶意输入或复杂模式 |
| 虚拟树全量渲染 | 帧率 < 10fps | 虚拟滚动 + 按需加载 | 1000+ 文件 |
| 搜索历史全量传输 | 内存暴涨 | 分页加载 + LRU 缓存 | 10000+ 历史条目 |
| 嵌套归档递归加载 | 响应时间 > 5s | 限制展开深度 | 5+ 层嵌套 |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| 正则 ReDoS | 服务拒绝 (DoS) | 超时 + 复杂度限制 |
| 路径遍历读取 | 读取任意文件 | 验证 hash 属于工作区 |
| 搜索历史泄露 | 敏感搜索暴露 | 敏感词过滤 |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|------------------|
| 正则语法错误无提示 | 用户不知道搜索失败 | 友好错误消息 + 修正建议 |
| 布尔逻辑不明确 | 结果与预期不符 | 显示解析后的查询树 |
| 虚拟树无反馈 | 加载时用户等待困惑 | 骨架屏/加载指示器 |
| 历史列表过长 | 难以找到常用搜索 | 按使用频率排序 + 搜索过滤 |

---

## "Looks Done But Isn't" Checklist

- [ ] **正则搜索**: 验证 ReDoS 防护已实现了吗？测试恶意输入
- [ ] **布尔逻辑**: 测试 `A AND B OR C AND D` 优先级正确吗？
- [ ] **搜索历史**: 实现去重和 LRU 限制了吗？1000+ 条历史测试
- [ ] **虚拟树**: 1000 文件性能测试了吗？展开/收起流畅吗？
- [ ] **文件读取**: 大文件 (100MB+) 流式读取测试了吗？
- [ ] **刷新机制**: 虚拟树显示最后更新时间了吗？

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| ReDoS 卡死 | MEDIUM | 添加超时，强制终止搜索线程 |
| 布尔逻辑错误 | LOW | 修复解析器，数据库无状态 |
| 历史数据膨胀 | LOW | 添加清理 job，重启应用 |
| 虚拟树性能 | HIGH | 重构为按需加载，需 UI 调整 |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| ReDoS 风险 | Phase: 正则搜索 UI 实现 | 恶意输入压力测试 |
| 布尔逻辑错误 | Phase: 多关键词搜索实现 | 单元测试覆盖边界情况 |
| 历史无限增长 | Phase: 搜索历史集成 | 内存长期运行测试 |
| 虚拟树性能 | Phase: 虚拟文件系统 UI | 1000+ 文件性能测试 |
| 文件状态不同步 | Phase: 虚拟文件系统完善 | UI 测试刷新机制 |

---

## Sources

### Primary (HIGH confidence)
- Rust 后端代码分析: `commands/search_history.rs`, `commands/virtual_tree.rs`, `search_engine/boolean_query_processor.rs`
- 项目现有陷阱: `.planning/research/PITFALLS.md` (FFI 集成陷阱)
- Flutter 官方文档: TreeView,虚拟滚动最佳实践

### Secondary (MEDIUM confidence)
- Flutter 桌面开发经验: 虚拟滚动性能优化
- Rust regex-automata 文档: 超时和复杂度控制

### Tertiary (LOW confidence - 未能验证)
- WebSearch 返回错误，部分发现基于经验推断
- 建议在实际实现中验证这些陷阱的准确性

---

*Pitfalls research for: Flutter Desktop 高级搜索与虚拟文件系统 (v1.1)*
*Researched: 2026-03-04*
