## 步骤4: 修复 websocketClient.ts flushPendingMessages 无限循环风险

**任务**: 修复 line 397-409 `flushPendingMessages` 方法可能的无限循环

**修改内容**:
- 添加消息数量上限 (MAX_PENDING_MESSAGES)
- 添加超时保护
- 确保在连接断开时正确退出循环

**验证方法**:
- 运行前端 lint 检查
- 运行前端类型检查

**影响范围**:
- 仅影响 WebSocket 消息队列处理