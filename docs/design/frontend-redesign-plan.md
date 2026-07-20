# Log Analyzer 前端改版规划

状态：已定稿（2026-07-20）  
范围：`log-analyzer/src` 视觉风格、组件状态、动画、动效与交互  
设计基线：Apple Design Language × Emil Kowalski design engineering — 清晰层级、适度材质、直接操控、高频操作瞬时、动效必须解释状态

生产迁移目标：HTML 原型 `Variant A — Calm Instrument Panel`。Variant B/C 仅保留为比较记录，不进入生产实现。实施拆分见 [`frontend-redesign-migration-plan.md`](./frontend-redesign-migration-plan.md)。

## 1. 目标体验

将当前界面收束为一套具有 Apple 桌面应用气质的“冷静分析工作台”：专业、紧凑、低干扰，使用系统字体、语义色和结构性材质建立清晰层级，同时让搜索、筛选、任务状态和错误反馈足够清楚。

三个体验目标：

1. 用户打开应用后 3 秒内理解当前位置、当前工作区和首要操作。
2. 导航、搜索、选择日志等高频操作即时响应，不用展示型动画阻塞操作。
3. 所有进入、退出和状态变化使用同一套颜色、间距、圆角、阴影与动效语言。

不做：无层级意义的玻璃拟态堆叠、大面积装饰渐变、弹跳式仪表盘动画、日志行进场动画、为装饰引入新动效依赖。

## 2. 现状审计

视觉核对覆盖 Workspaces、Search 和 Filter Palette；代码审计覆盖应用壳、导航、页面切换、按钮、卡片、弹窗、搜索控制、结果列表和详情面板。

| Before | After | Why |
| --- | --- | --- |
| 侧栏 Logo 使用渐变图标、渐变文字和发光阴影 | 单色品牌标记 + 清晰文字，强调色只保留一处 | 当前品牌元素比任务内容更抢眼，专业工具应优先建立信息层级 |
| `NavItem` 用 Framer Motion 共享布局弹簧移动激活背景 | 导航激活状态即时切换；仅指针 hover 使用 120ms 颜色过渡 | 导航是高频操作，不应等待弹簧；键盘触发必须无动画 |
| 页面通过 `AnimatePresence mode="wait"` 先退场再进场 | 页面内容即时替换，不做整页位移动画 | `mode="wait"` 串行延迟会放大“应用很慢”的感受 |
| 多处 `transition-all duration-200` | 仅过渡 `color`、`background-color`、`border-color`、`box-shadow` 或 `transform` | 避免意外布局动画，并明确每个动效的目的 |
| 工作区卡片是大块中灰表面，标题区与正文割裂 | 低对比主表面 + 一致内边距 + 底部状态行 | 降低视觉重量，让名称、操作和状态形成单一阅读路径 |
| Search 顶部同时出现 Filters 和 Advanced Filters，但语义不同 | 将 Filters 明确命名为“Keyword Groups”；Advanced Filters 收束为紧凑二级工具栏 | 消除“两个筛选入口”的认知冲突 |
| Level 筛选只显示 `E/W/I/D` | 显示 `Error/Warn/Info/Debug`，窄窗口再降级为首字母 | 单字母映射依赖记忆，节省的空间不值得牺牲可理解性 |
| Filter Palette 固定 `600px`，无数据时仍形成巨大空盒 | `width: min(34rem, calc(100vw - 2rem))`，空状态紧凑，内容超长再滚动 | 浮层大小应由任务内容决定，并保持与触发器的空间关系 |
| 搜索中按钮使用 `animate-pulse` | 保持按钮稳定，图标使用线性旋转，按钮文案显示明确进度 | 整个按钮呼吸会削弱可读性，旋转图标已经足够表达进行中 |
| Log detail panel 使用不可中断的 `animate-slide-in` | 220ms `transform` + `opacity` ease-out；关闭 160ms；可在当前值反向 | 侧面板需要空间连续性，也应允许快速关闭和重新打开 |
| 弹窗只定义内容 `zoom-in-95`，遮罩与内容缺少统一时序 | 遮罩 160ms 淡入，内容 200ms 从 `scale(.97)` + `opacity(0)` 进入；退出 140ms | 模态层需要聚焦，进入与退出应协调且退出更快 |
| 所有 Skeleton 长时间统一 pulse | 首屏/换页使用低幅 shimmer；虚拟列表占位保持静态或低幅 pulse | 大面积持续闪烁会造成视觉噪声，日志工具尤其明显 |
| 全局 reduced-motion 将动画压到 `0.01ms` | 用组件级降级：移除位移/缩放，保留 120–160ms 颜色或透明度反馈 | Reduced motion 不是移除全部反馈，而是避免前庭刺激 |

## 3. 视觉方向：Apple Calm Instrument Panel

### 3.1 色彩角色

- 背景分三层：应用底色、工作表面、浮层表面。侧栏、工具栏和浮层使用结构性半透明材质，内容卡片保持稳定实色；不在同一层级连续堆叠多个透明面板。
- Apple System Blue 只用于主操作、当前焦点、选中态和可行动链接，不用于普通装饰文字。
- 状态色只表达状态：Error、Warning、Success、Info；不与品牌色混用。
- 边框承担分区，阴影仅用于浮层、详情面板和模态框。
- 默认支持暗色；浅色作为同一语义 token 的映射，不在组件内写死颜色。

建议 token：

```css
--surface-canvas-dark: #1c1c1e;
--surface-panel-dark: #242426;
--surface-raised-dark: #2c2c2e;
--surface-canvas-light: #f5f5f7;
--surface-panel-light: rgba(255, 255, 255, 0.86);
--material-sidebar: rgba(38, 38, 40, 0.78);
--material-toolbar: rgba(28, 28, 30, 0.76);
--separator: rgba(255, 255, 255, 0.095);
--text-primary: rgba(255, 255, 255, 0.94);
--text-secondary: rgba(235, 235, 245, 0.62);
--accent-dark: #0a84ff;
--accent-light: #007aff;
--success: #30d158;
--warning: #ff9f0a;
--danger: #ff453a;
```

最终值需用实际窗口和日志内容做 WCAG 对比度复核；组件只能消费语义 token。

### 3.2 字体与密度

- 界面字体优先 `-apple-system` / `BlinkMacSystemFont` 并启用 optical sizing；日志内容、查询表达式、路径和时间戳使用 SF Mono 等宽字体。
- 页面标题 26/32、区块标题 15/20、正文 13/20、辅助信息 12/16。
- 去掉无必要的全大写标题；仅表头和极短状态标签使用大写，并保持有限字距。
- 控件基准高度 36px，主搜索框 40px，图标按钮可点击区域至少 40×40px。
- 间距使用 4px 基线：4、8、12、16、24、32。

### 3.3 形状与层级

- 普通输入和按钮圆角 10px；卡片 14px；模态框 16px；状态胶囊使用全圆角。
- 普通卡片无发光阴影，hover 只提升边框和背景亮度。
- 主按钮保持实色；Secondary 为中性表面；Ghost 只在工具栏和行内操作使用。
- 材质只用于侧栏、工具栏、检查器和浮层，以 `backdrop-filter: saturate(...) blur(...)` 表达窗口层级；`prefers-reduced-transparency` 下退化为不透明表面。

## 4. 信息架构与页面布局

### 应用壳

- 保留左侧栏 + 顶部工作区栏结构。
- 侧栏宽度从 240px 调整为 224px；减少品牌区高度与底部空置感。
- 顶部栏左侧显示当前位置，右侧预留任务进度、全局状态与轻量操作。
- 活动导航使用 Apple 式圆角选中面，不移动、不使用共享布局动画。
- 顶部状态区提供浅色/深色外观切换；语义 token 随外观映射，组件不写死颜色。

### Workspaces

- 页面标题与主要导入操作保持同一水平基线。
- 工作区卡片使用一致的 320–360px 宽度；名称为主信息，路径/哈希为次信息，状态在底部。
- 常用动作直接显示；删除放入 overflow menu，并在确认对话框中说明影响。
- Processing 用稳定状态点 + 旋转图标，不让整张卡片 pulse。

### Search

- 第一行：查询输入、Keyword Groups、Search；CSV/JSON 合并为 Export 菜单。
- 第二行：Level、Time range、File pattern、Reset，整体控制在 44px 左右。
- Active filters 只在有筛选时出现为可移除 chips，不显示 `Active: None`。
- 结果表头保持 sticky，但降低字距；Level、Time、File 列允许用户调整宽度或采用更合理默认值。
- 空状态靠近结果区域视觉中心，并提供明确行动：选择工作区或前往 Workspaces。
- 详情面板默认宽度 420px，允许拖动调整到窗口的 30–55%，宽度变化不使用 transition。

### Keywords / Tasks / Settings

- Keywords 使用“组 → 规则 → 匹配模式”的稳定层级；编辑为模态任务，删除为小型确认。
- Tasks 按 Running、Failed、Completed 排序；进行中只动画状态图标。
- Settings 使用分组表单和粘性保存栏；成功保存以内联状态确认，不用庆祝性动画。

## 5. 动效规范

### 5.1 Token

```css
--ease-out-ui: cubic-bezier(0.23, 1, 0.32, 1);
--ease-in-out-ui: cubic-bezier(0.77, 0, 0.175, 1);
--duration-press: 120ms;
--duration-hover: 120ms;
--spring-ui: cubic-bezier(0.32, 0.72, 0, 1);
--duration-popover-enter: 180ms;
--duration-popover-exit: 130ms;
--duration-panel-enter: 260ms;
--duration-panel-exit: 180ms;
--duration-modal-enter: 240ms;
--duration-modal-exit: 180ms;
```

### 5.2 交互矩阵

| 场景 | 频率 | 目的 | 方案 |
| --- | --- | --- | --- |
| 侧栏导航、键盘切页 | 高 | 即时响应 | 无页面进出动画；激活态即时替换 |
| 按钮按下 | 高 | 输入反馈 | 指针按压 `scale(.97)`，120ms ease-out；键盘保留 focus ring，不缩放 |
| Hover | 高 | 可点击提示 | 仅细指针设备，120ms 颜色/边框变化 |
| Filter/Export popover | 中 | 空间来源 | 从触发器方向以 `translateY(-4px) scale(.96)` + opacity + 轻微 blur 进入，180ms ease-out |
| Modal | 低 | 聚焦任务 | 内容中心弹簧进入；遮罩淡入，背景轻微缩小并降亮度，退出更快 |
| Log detail panel | 中 | 空间连续性 | 260ms 抽屉曲线进入；分隔线支持 pointer capture 直接拖拽宽度，拖拽期间无 transition |
| Toast | 低 | 完成/错误反馈 | 从右下同一路径进入退出；动态堆栈使用可中断 transition |
| 搜索/导入进度 | 中 | 持续状态 | 线性旋转图标或 determinate progress；不动画容器 |
| 日志行选择 | 极高 | 当前上下文 | 背景与左侧标记即时切换，不移动、不缩放 |

实现规则：

- 不使用 `transition: all`。
- UI 动画原则上不超过 300ms。
- 进入用 ease-out；屏幕内位置变化用 ease-in-out；持续旋转用 linear。
- 只动画 `transform` 和 `opacity`；颜色与边框可做短过渡。
- Popover 的 `transform-origin` 必须与触发器一致；Modal 保持中心原点。
- 不给虚拟日志行添加 enter/stagger 动画。

## 6. 组件改造边界

| 层级 | 主要文件 | 责任 |
| --- | --- | --- |
| Token/全局 | `tailwind.config.*`, `src/index.css` | 颜色、字号、圆角、阴影、动效与无障碍 token |
| 基础组件 | `components/ui/Button.tsx`, `Input.tsx`, `Card.tsx`, `NavItem.tsx`, `Skeleton.tsx` | 所有状态成为默认能力，移除散落的 `transition-all` |
| 应用壳 | `App.tsx`, `Sidebar.tsx`, `WorkspaceHeader.tsx`, `PageTransition.tsx` | 清晰层级、即时导航、统一焦点和状态 |
| 浮层 | `components/modals/*` | 统一遮罩、焦点管理、origin、进出时序和 Escape 行为 |
| 搜索工作台 | `pages/SearchPage.tsx`, `pages/SearchPage/components/*` | 工具栏重排、筛选命名、结果密度、详情面板 |
| 其他页面 | `WorkspacesPage.tsx`, `KeywordsPage.tsx`, `TasksPage.tsx`, `SettingsPage.tsx` | 使用统一页面模板与组件，不局部发明样式 |

不在本次改版中修改 Rust 后端、IPC 合约、查询语义、导入逻辑或虚拟列表数据模型。

## 7. 分阶段实施

### Phase 1 — Design foundation

- 建立语义色、排版、间距、圆角、阴影和 motion token。
- 改造 Button、Input、Card、NavItem、Skeleton。
- 清理 `transition-all` 和没有目的的动画。
- 建立基础组件状态清单：default、hover、active、focus-visible、disabled、loading、error。

验收：任一页面不再自行定义主按钮、焦点环或卡片 hover 规则。

### Phase 2 — Shell and navigation

- 重做 Sidebar 与 WorkspaceHeader 的层级。
- 移除共享布局弹簧和整页等待式过渡。
- 验证鼠标、键盘和屏幕阅读器导航。

验收：连续切换 20 次页面无等待感、无布局跳动、焦点位置可预测。

### Phase 3 — Search cockpit

- 重排搜索主工具栏和高级筛选栏。
- 重命名 Keyword Groups 入口，合并 Export 行为。
- 改造结果表头、空状态、加载状态、选中态和详情面板。

验收：在 1280×720、1440×900、1920×1080 下无横向溢出；虚拟滚动性能不下降。

### Phase 4 — Supporting pages and overlays

- Workspaces、Keywords、Tasks、Settings 套用统一页面模板。
- 统一 Filter Palette、Keyword Modal、File Filter Settings 和确认对话框。
- 统一 toast、错误、空状态和保存状态。

验收：所有浮层的进入来源、退出路径、Escape、遮罩点击和焦点恢复一致。

### Phase 5 — Accessibility and polish

- 组件级实现 reduced motion；hover 仅在 `(hover: hover) and (pointer: fine)` 生效。
- 验证对比度、200% 字体缩放、键盘顺序和 40px 最小点击区。
- 以慢放和逐帧方式检查 modal、popover、详情面板；真实桌面构建再做最终 feel-check。

验收：无动画依赖才能理解的状态；reduced-motion 下无大幅位移或缩放。

## 8. 验证清单

每个 Phase 至少运行：

```bash
cd log-analyzer
npm run type-check
npm run lint
npm test -- --runInBand
npm run build
```

视觉验收：

- 比较 Workspaces、Search、Keywords、Tasks、Settings 五个页面的统一性。
- 检查 1280×720 最小目标窗口和 200% 文本缩放。
- 检查默认、hover、pointer active、keyboard focus、disabled、loading、error。
- DevTools 4× 慢放检查 transform origin、进入/退出方向和属性同步。
- 在 Tauri 桌面环境验证加载时动画是否掉帧，尤其是搜索和虚拟滚动期间。

## 9. 推荐实施顺序

严格按 Phase 1 → 2 → 3 → 4 → 5 推进。Phase 1 是其余页面的依赖；Search 是业务核心，应在设计基础稳定后优先落地。每个 Phase 独立提交和验收，避免视觉改版与业务逻辑重构混在同一个 diff 中。
