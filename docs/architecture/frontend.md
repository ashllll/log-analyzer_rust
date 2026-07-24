# 前端设计系统

前端采用 Apple 风格的双主题设计系统：CSS 变量承载全部色彩与材质语义，Tailwind 负责响应式与布局，`AppearanceProvider` 管理亮 / 暗 / 跟随系统三态切换。所有页面与组件共享同一套变量，主题切换无需重载页面。

## CSS 变量体系

变量定义在 `src/index.css`，分两套：

- `:root` — 暗色（默认），`color-scheme: dark`
- `:root[data-theme="light"]` — 亮色覆盖

变量按语义分六大类，值统一为 **RGB 三通道格式**（如 `--color-bg-main: 28 28 30`），配合 Tailwind 的 `<alpha-value>` 占位符支持任意透明度：

| 类别 | 代表变量 | 用途 |
| --- | --- | --- |
| 背景 | `--color-bg-main / sidebar / card / hover / elevated / popover / subtle / surface` | 页面与各层级表面 |
| 边框 | `--color-border-base / light / subtle` | 分隔线与描边 |
| 文本 | `--color-text-main / muted / dim` | 三级文字层次 |
| 强调 | `--color-accent / accent-hover / accent-muted` | 主色（macOS 蓝） |
| 状态 | `--color-success / warning / danger / info` | 操作反馈 |
| 日志级别 | `--color-log-error / warn / info / debug` | 日志行着色 |

### Material 半透明变量

毛玻璃材质的颜色也随主题变化：

```css
--material-sidebar: rgb(38 38 40 / .78);   /* 暗色 */
--material-toolbar: rgb(28 28 30 / .76);
--material-overlay: rgb(46 46 50 / .84);
```

配套 `.apple-material` 工具类统一模糊参数：

```css
.apple-material {
  backdrop-filter: saturate(165%) blur(24px);
  -webkit-backdrop-filter: saturate(165%) blur(24px);
}
```

侧栏、工具栏、对话框等叠加层组合使用 `background: var(--material-*)` + `.apple-material`。

## Tailwind 集成

`tailwind.config.js` 将变量映射为命名颜色，透明度由 `<alpha-value>` 动态注入：

```js
colors: {
  bg:     { main: "rgb(var(--color-bg-main) / <alpha-value>)", /* … */ },
  border: { base: "rgb(var(--color-border-base) / <alpha-value>)", /* … */ },
  text:   { main: "rgb(var(--color-text-main) / <alpha-value>)", /* … */ },
  status: { error: "rgb(var(--color-danger) / <alpha-value>)", /* … */ },
  log:    { error: "rgb(var(--color-log-error) / <alpha-value>)", /* … */ },
}
```

组件写作 `bg-bg-card`、`text-text-muted`、`border-border-subtle`，语义与主题自动绑定。字体栈优先 SF Pro（`-apple-system`），等宽使用 SF Mono。

## 主题切换

`src/theme/AppearanceProvider.tsx`：

- **三态模式**：`'system' | 'light' | 'dark'`，默认 `system`
- **持久化**：`localStorage["log-analyzer-appearance"]`
- **系统跟随**：监听 `(prefers-color-scheme: dark)` 媒体查询变化
- **应用方式**：写 `document.documentElement.dataset.theme` + `style.colorScheme`，CSS 变量随之切换

`AppearanceControl` 组件在顶栏提供 cycle 切换按钮（system → light → dark → system）。

## 组件分层

```
components/
├── ui/          # 设计系统原语
│   ├── Button / Input / Card / NavItem / FormField
│   ├── Skeleton / EmptyState
│   └── DialogSurface / PopoverSurface   # Portal 渲染、焦点陷阱、ESC 关闭
├── search/      # 搜索业务组件（SearchControls / SearchResults / LogRow / LogDetailPanel …）
├── renderers/   # HybridLogRenderer — 关键词高亮 + 智能截断
├── modals/      # FileFilterSettings / FilterPalette / KeywordModal
└── 布局组件      # Sidebar / WorkspaceHeader / ErrorBoundary / PageTransition
```

原语层不含业务逻辑，通过 props 接受变体；业务组件组合原语；`DialogSurface` / `PopoverSurface` 处理焦点管理与退出动画。

## 动画系统

三条统一的缓动曲线（`index.css`）：

| 变量 | 值 | 场景 |
| --- | --- | --- |
| `--ease-out-ui` | `cubic-bezier(.23, 1, .32, 1)` | 常规 UI 过渡 |
| `--ease-in-out-ui` | `cubic-bezier(.77, 0, .175, 1)` | 强调型过渡 |
| `--ease-drawer` | `cubic-bezier(.32, .72, 0, 1)` | 抽屉 / 面板滑入 |

关键模式：

- **`.ui-pressable`**：可按压元素，`active` 时 `scale(.97)`
- **`@starting-style`**：Popover / Inspector 的纯 CSS 入场动画，无需 JS 挂载类
- **`.skeleton-shimmer`**：骨架屏扫光动画（1.8s 线性循环）
- **对话框联动**：`body:has(> .dialog-host) .app-shell` 在对话框打开时将背景缩放 `.995` 并压暗至 `.88`，模拟 macOS  sheet 效果

## 无障碍策略

三个系统级媒体查询在 `index.css` 末尾统一降级：

| 媒体查询 | 降级行为 |
| --- | --- |
| `prefers-reduced-motion: reduce` | 关闭过渡与 shimmer 动画 |
| `prefers-reduced-transparency: reduce` | 材质退化为不透明背景 |
| `prefers-contrast: more` | 提高边框与文本对比度 |

另有两条全局规则：`:focus-visible` 统一 2px accent 描边；`::selection` 使用 30% 透明度的 accent 色。

## 相关文件

| 文件 | 职责 |
| --- | --- |
| `src/index.css` | 变量、材质、缓动、无障碍降级 |
| `tailwind.config.js` | 变量 → 命名颜色映射、字体栈、阴影 |
| `src/theme/AppearanceProvider.tsx` | 主题状态与系统跟随 |
| `src/theme/AppearanceControl.tsx` | 顶栏三态切换按钮 |
| `src/components/ui/` | 设计系统原语实现 |
