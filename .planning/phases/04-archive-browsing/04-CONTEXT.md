# Phase 4: 压缩包浏览 - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

用户可以浏览压缩包内的文件列表、预览文本文件内容、在压缩包内搜索关键词。实现依赖 Rust 后端已有的压缩包处理能力(ARCH-01, ARCH-02, ARCH-03)。

</domain>

<decisions>
## Implementation Decisions

### 文件列表展示
- **树形视图** — 展示嵌套目录结构，类似文件管理器，适合深层次压缩包

### 预览布局
- **Split Pane** — 左侧文件列表，右侧实时预览，便于快速浏览多个文件
- 单击文件立即预览，无需额外操作

### 关键词高亮
- 预览文本文件时**支持关键词高亮显示**
- 压缩包内搜索使用**实时搜索**模式：搜索框输入关键词，实时显示匹配结果列表

### 压缩包格式
- **所有主流格式**: ZIP/TAR/GZ/RAR/7Z

### 文件大小处理
- 大文件(超过阈值)**截断并提示**用户无法完整预览
- 空压缩包或无法预览时显示**友好提示信息**

### Claude's Discretion
- 树形视图的具体展开/折叠交互细节
- 搜索结果排序逻辑
- 预览面板的默认宽度比例
- 大文件阈值具体数值

</decisions>

<specifics>
## Specific Ideas

- "用户明确说要在压缩包内搜索" — 所以预览也支持关键词高亮
- 类似文件管理器的树形结构体验

</specifics>

<deferred>
## Deferred Ideas

- 无 — 讨论保持在 Phase 4 范围内

</deferred>

---

*Phase: 04-archive-browsing*
*Context gathered: 2026-03-02*
