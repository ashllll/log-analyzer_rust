# Phase 7: 后端 API 集成 - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Flutter 应用通过 flutter_rust_bridge FFI 调用 Rust 后端的搜索历史和虚拟文件树 API。支持搜索历史的增删改查、虚拟文件树获取、正则表达式搜索、多关键词组合搜索。

</domain>

<decisions>
## Implementation Decisions

### API 接口粒度
- 使用细粒度方法设计，而非粗粒度
- 方法命名风格：按操作命名 (如 `searchHistory.add`, `searchHistory.get`, `searchHistory.delete`)
- 虚拟文件树使用懒加载方式，按需加载子节点
- 支持批量删除操作

### 错误传播机制
- FFI 调用失败时抛出异常
- Flutter 端使用分类异常 (网络错误、权限错误、未找到等)
- 错误消息包含详细错误信息和调试数据
- 后端自动记录错误日志
- 后端重试策略：3次重试
- FFI 调用超时时间：10秒

### 流式数据处理
- 虚拟文件树使用 Stream 流式传输
- 分批大小：每批100条
- 启用客户端缓存
- 文件树刷新采用增量更新

### 状态同步策略
- Flutter 端通过手动刷新获取后端最新数据
- 刷新触发方式：按钮刷新
- 搜索历史采用自动同步 (每次搜索后自动添加)
- 前后端数据一致性要求：强一致性
- 离线数据处理：网络恢复后自动同步
- 无需版本控制
- 启用乐观更新 (先更新 UI，后同步后端)
- 数据加载时显示骨架屏
- 加载失败时自动重试
- 允许并发请求
- 前后端数据冲突时后端数据优先
- 启用 debounce (快速连续刷新只触发一次)

</decisions>

<specifics>
## Specific Ideas

- 使用 flutter_rust_bridge 作为 FFI 框架 (现有)
- 参考现有 API 设计模式进行扩展

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-api*
*Context gathered: 2026-03-04*
