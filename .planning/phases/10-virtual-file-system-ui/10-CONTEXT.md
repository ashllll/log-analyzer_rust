# Phase 10: 虚拟文件系统 UI - Context

**Gathered:** 2026-03-07
**Status:** Ready for planning

<domain>
## Phase Boundary

用户可以浏览工作区的虚拟文件树、展开/折叠目录、预览文件内容。文件树作为侧边栏组件集成到现有搜索页面。创建、删除、重命名文件等操作属于其他阶段。

Depends on: Phase 8 (VirtualFileTreeProvider 已就绪)

</domain>

<decisions>
## Implementation Decisions

### 布局与结构

- **位置**: 左侧边栏 + 主预览区，类似 VS Code 的经典 IDE 布局
- **行高密度**: 紧凑模式，每行 24-28px，适合大型工作区
- **侧边栏宽度**: 可拖动调整，最小 200px，最大 500px
- **预览面板**: 标签页切换模式，保留原有日志列表，用户通过标签切换

### 交互行为

- **点击行为**: 单击选中并预览，右键显示上下文菜单
- **键盘导航**: 完整支持
  - 上下箭头：导航节点
  - 左右箭头：折叠/展开目录
  - 回车：打开预览
- **多选支持**: 支持 Ctrl+点击 和 Shift+点击 多选文件/目录
- **目录展开**: 点击箭头图标展开，点击名称选中/预览

### 内容展示

- **节点信息**: 仅显示文件名（鼠标悬停显示完整路径 tooltip）
- **图标风格**: 文件类型图标，根据扩展名区分（.log, .txt, .json, .zip 等）
- **预览内容**: 显示文件内容（纯文本），支持滚动查看
- **语法高亮**: 纯文本显示，无语法高亮（适合日志文件）

### 状态处理

- **空状态**: 友好空状态，显示图标 + 文案 "工作区为空，导入文件开始分析"
- **加载状态**: 骨架屏（Skeleton）动画效果
- **错误状态**: 使用现有的 ErrorView 组件，显示错误信息和重试按钮
- **预览加载**: 显示加载状态指示器，加载完成后显示内容

### Claude's Discretion

- 具体的骨架屏样式和动画
- 文件类型图标的具体设计
- 错误信息的具体文案
- 加载指示器的样式

</decisions>

<specifics>
## Specific Ideas

- "类似 VS Code 的侧边栏文件树体验"
- "与现有搜索页面通过标签页整合"

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope

</deferred>

---

*Phase: 10-virtual-file-system-ui*
*Context gathered: 2026-03-07*
