# Apple Calm Instrument Panel 迁移计划

状态：已实施并通过二次验证  
设计定稿：2026-07-20  
目标：将 `frontend-redesign-prototype.html` 的 Variant A 迁移到现有 React 19 + Tailwind CSS 3 + Tauri 2 前端，不改变 Rust 后端、IPC 合约和业务语义。

## 1. 已锁定决策

- 生产目标为 Variant A：224px 左侧栏、52px 顶部工作区栏、内容工作区。
- Apple System Blue 为唯一品牌/操作强调色；成功、警告、错误、信息使用独立系统语义色。
- 默认跟随系统外观，并提供 Light / Dark / System 三态选择。
- 结构性材质只用于侧栏、顶部栏、检查器、Popover 和 Modal；普通内容卡片保持稳定表面。
- 主导航、键盘切页、日志行选择即时切换，不做整页进出动画。
- Button 指针按压为 `scale(.97)` / 120ms；Popover 进入 180ms、退出 130ms；Modal 进入 240ms、退出 180ms；详情面板进入 260ms、退出 180ms。
- Search 详情面板默认 420px，可直接拖拽至 320–640px；拖拽使用 pointer capture，宽度变化不加 transition。
- 支持 `prefers-reduced-motion`、`prefers-reduced-transparency` 和 `prefers-contrast`。

## 2. 现有代码影响面

代码知识图谱显示前端范围包含 99 个 TypeScript 文件和 2 个 CSS 文件。迁移主接缝如下：

| Seam | 当前实现 | 迁移策略 |
| --- | --- | --- |
| 应用壳 | `AppContent` 组合 `Sidebar`、`WorkspaceHeader`、`PageTransition` | 保持路由、后端同步和 Store 不动，只替换壳层实现 |
| 页面切换 | `PageTransition` 使用 `AnimatePresence mode="wait"` | 保留 Suspense / ErrorBoundary / Routes，删除整页 motion 包装 |
| 导航 | `Sidebar` + `NavItem` 使用渐变和共享布局弹簧 | 改为即时 Apple 选中面；保持现有路由与测试 ID |
| 基础 UI | `Button`、`Card`、`Input`、`Skeleton` 各自携带样式与动效 | 先迁移语义 token 和状态，再让页面统一消费 |
| 浮层 | `FilterPalette`、`KeywordModal`、`FileFilterSettings` 分别实现遮罩、焦点和动画 | 抽取共用 Overlay/Dialog 实现，业务表单仍留在原模块 |
| Search | `SearchPage` 负责业务编排，子模块负责渲染 | 不改查询、分页、事件、虚拟列表；只重排 `SearchControls`、`SearchFilters`、`SearchResults`、`LogDetailPanel` |
| 外观 | 当前仅深色，无主题状态 | 新增小接口的 Appearance 模块，内部处理系统偏好与本地选择 |

七个文件直接依赖 Framer Motion：`PageTransition`、`Sidebar`、`Button`、`NavItem`、Workspaces、Keywords、Tasks。迁移完成且引用归零后再删除依赖。

## 3. 模块与接口设计

### 3.1 Theme module

在 `src/theme/` 建立一个深模块，外部接口只暴露：

```ts
type AppearanceMode = 'system' | 'light' | 'dark';

interface AppearanceValue {
  mode: AppearanceMode;
  resolvedMode: 'light' | 'dark';
  setMode(mode: AppearanceMode): void;
}
```

系统媒体查询、本地保存、`data-theme` 写入和监听清理均留在实现内部。调用页面不直接访问 `matchMedia` 或 `localStorage`。

### 3.2 Visual foundation

- `index.css`：定义 light/dark CSS variables、字体、焦点环、选区、滚动条和三类无障碍媒体查询。
- `tailwind.config.js`：现有 `bg-*`、`text-*`、`border-*` 类继续作为兼容接口，但值改为 CSS variables；迁移期间避免全仓类名大爆炸。
- 新增 motion token：`--ease-out-ui`、`--spring-ui` 和各组件 duration。
- 禁止新增 `transition-all`、高频列表 stagger、页面级位移过场和无语义 glow。

### 3.3 Overlay module

建立共享 `DialogSurface` / `PopoverSurface`。模块实现负责遮罩、Escape、焦点圈闭、焦点恢复、进入/退出状态和 reduced-motion；调用方只提供 open、title、onClose、内容和 footer。`KeywordModal` 与 `FileFilterSettings` 是两个真实使用者，足以形成稳定 seam。

### 3.4 Page modules

不建立一个承载所有页面差异的巨型 Layout。只共享稳定、低参数量的 `PageHeader`、`SectionSurface` 和 `StatusBadge`；页面的信息层级仍留在各自模块，避免为了复用把原型结构压平。

## 4. 分阶段迁移

### Phase 0 — 基线与护栏

改动：

- 将 HTML 原型和设计文档标记为视觉验收的 primary source。
- 记录现有 Workspaces、Search、Keywords、Tasks、Settings 的关键 DOM 行为和测试 ID。
- 为 Button、Appearance、DialogSurface、PopoverSurface 和 inspector resize 先建立行为测试骨架。

验收：现有测试保持绿色；业务测试不因纯视觉改动重写断言。

### Phase 1 — Token、外观与基础 UI

主要文件：

- `tailwind.config.js`
- `src/index.css`
- `src/theme/*`（新增）
- `src/components/ui/Button.tsx`
- `src/components/ui/Input.tsx`
- `src/components/ui/Card.tsx`
- `src/components/ui/NavItem.tsx`
- `src/components/ui/Skeleton.tsx`
- `src/components/ui/EmptyState.tsx`

实施：

- 用 CSS variables 替换 Zinc/Teal/Emerald 硬编码，并添加 Apple light/dark 映射。
- Button 改回原生 `button`，用 CSS active 反馈取代 `motion.button`，只过渡明确属性。
- Card 删除 glow 和 `transition-all`；Input、focus ring、disabled、loading、error 统一。
- Skeleton 从大面积持续 pulse 改为低幅 shimmer；虚拟结果占位保持静态。

验收：基础模块的 default、hover、active、focus-visible、disabled、loading、error 均有测试；Light/Dark/System 可切换并正确恢复。

### Phase 2 — 应用壳与即时导航

主要文件：

- `src/App.tsx`
- `src/components/Sidebar.tsx`
- `src/components/WorkspaceHeader.tsx`
- `src/components/PageTransition.tsx`

实施：

- 落地 224px 材质侧栏、52px 材质顶部栏和状态/外观控制。
- 删除 Sidebar 渐变、LayoutGroup 和 NavItem 共享布局弹簧。
- `PageTransition` 只保留 ErrorBoundary、Suspense 和 Routes；路由内容即时替换。
- 保留 skip link、路由路径、现有测试 ID、工作区状态逻辑和懒加载。

验收：连续切页无等待、无整页位移；键盘焦点与当前路由一致；初始化、错误和 Suspense 状态仍工作。

### Phase 3 — Search cockpit

主要文件：

- `src/pages/SearchPage.tsx`
- `src/pages/SearchPage/components/SearchControls.tsx`
- `src/pages/SearchPage/components/SearchFilters.tsx`
- `src/pages/SearchPage/components/ActiveKeywords.tsx`
- `src/pages/SearchPage/components/SearchResults.tsx`
- `src/pages/SearchPage/components/LogDetailPanel.tsx`
- `src/components/modals/FilterPalette.tsx`

实施：

- 第一行统一查询、Keyword Groups、Export、Search；第二行统一 Level、Time、File、Reset。
- FilterPalette 迁移为 origin-aware PopoverSurface，宽度改为响应式内容宽度。
- 保持 `useSearchState`、`useSearchQuery`、`useInfiniteSearch`、`useSearchEvents`、虚拟滚动和导出逻辑原样。
- SearchResults 只调整表头、密度、选中状态和语义色，不给日志行添加 enter/stagger。
- LogDetailPanel 增加 pointer-capture resize、420px 默认宽度和可中断抽屉动效；关闭后焦点回到被选日志行。

验收：现有 SearchControls/SearchFilters/ActiveKeywords 测试通过；补充 Popover Escape/焦点恢复、检查器拖拽边界与 reduced-motion 测试；大数据虚拟滚动无明显性能回退。

### Phase 4 — Workspaces、Keywords、Tasks

主要文件：

- `src/pages/WorkspacesPage.tsx`
- `src/pages/KeywordsPage.tsx`
- `src/pages/TasksPage.tsx`

实施：

- Workspaces 迁移为定稿卡片结构，保留导入、刷新、监听、切换和删除行为。
- Keywords 迁移为“组列表 + 规则详情”双栏结构；移除列表 stagger 和发光色点。
- Tasks 迁移为紧凑表格/列表；只让运行中状态图标旋转，进度宽度使用明确的 width transition。
- 删除三个页面的 Framer Motion 容器。

验收：现有 Workspace workflow、KeywordsPage 和 task store 测试通过；关键动作测试 ID 不变。

### Phase 5 — Settings、Modal、Toast 与状态面

主要文件：

- `src/pages/SettingsPage.tsx`
- `src/components/modals/KeywordModal.tsx`
- `src/components/modals/FileFilterSettings.tsx`
- `src/components/ErrorBoundary.tsx`
- `src/hooks/useToast.ts`

实施：

- Settings 迁移为左侧分组导航、右侧表单和粘性保存栏，不改配置读写与校验。
- KeywordModal/FileFilterSettings 改用 DialogSurface，删除重复焦点和 Escape 实现。
- Modal 打开时背景轻微缩小和降亮度；退出比进入快；reduced-motion 只保留 opacity。
- Toast 统一 Apple material、系统语义色和非对称进出时序。
- 错误、空状态、保存成功使用同一组 StatusBadge / inline feedback。

验收：Modal 的 tab loop、Escape、遮罩关闭、焦点恢复测试通过；配置保存和错误处理行为不变。

### Phase 6 — 清理与整体验收

- 全仓清理 `transition-all`、无目的 glow、页面/list stagger 和旧 Teal/Emerald 品牌 token。
- Framer Motion 引用为零后从 `package.json` 删除；若仍有不可替代的低频交互，记录保留原因，不为“删依赖”重写稳定代码。
- 删除不再使用的 `App.css` Vite 模板样式。
- 原型保留为定稿记录，但不接入生产路由和构建入口。

## 5. 提交与回滚策略

每个 Phase 独立 PR/提交，不混入 Rust、IPC 或业务重构。Phase 1 的 token 迁移采用兼容类名，使单个页面可以独立回滚。Phase 3 单独提交 Search，因为它包含 406 行编排模块和虚拟列表，是最高风险区域。任何阶段若功能测试失败，回滚该阶段渲染实现，不回滚已验证的基础 token。

建议提交顺序：

1. `ui: add apple semantic tokens and appearance module`
2. `ui: migrate primitives to apple interaction states`
3. `ui: migrate application shell and remove page transitions`
4. `ui: migrate search cockpit and resizable inspector`
5. `ui: migrate workspaces keywords and tasks`
6. `ui: migrate settings overlays and feedback`
7. `ui: remove obsolete motion and legacy styles`

## 6. 每阶段验证

```bash
cd log-analyzer
npm run type-check
npm run lint
npm test -- --runInBand
npm run build
```

桌面验收至少覆盖：

- 1280×720、1440×900、1920×1080。
- Light、Dark、System；系统主题运行中切换。
- 鼠标、键盘、200% 文本缩放。
- reduced-motion、reduced-transparency、increased-contrast。
- Workspaces 导入/切换、Search 搜索/分页/选择/导出、Keywords 编辑、Tasks 取消、Settings 保存。
- Tauri 开发构建下搜索进行中与虚拟滚动期间的动画帧稳定性。

## 7. 完成定义

- 五个生产页面与定稿 Variant A 的信息层级、配色、材质和关键交互一致。
- 业务 Store、hooks、Tauri Events、IPC 参数和 Rust 代码没有因视觉迁移发生语义变化。
- 不存在 `transition-all`、高频导航动画、日志行 enter/stagger 或 `scale(0)` 入场。
- 所有浮层具备 Escape、焦点圈闭、焦点恢复、遮罩行为和 reduced-motion 降级。
- 三种外观模式、三类无障碍媒体查询和检查器拖拽均通过自动测试与桌面人工验收。
- `type-check`、`lint`、Jest、Vite build 全部通过。
