# Log Analyzer 全面开发指导文档


## 1. 项目概览 (Project Overview)

* **项目名称**: Log Analyzer
* **定位**: 企业级桌面日志分析工具，专注于**高性能**、**大数据量**和**极佳的用户体验**。
* **核心能力**:

  * **GB 级日志流式加载与虚拟滚动。**

  * **多关键词正则混合搜索（Ad-hoc + 预设配置）。**
  * **智能语义高亮与中文注释 Tag。**
  * **全键盘操作支持与指令中心。**
  * **任务与工作空间管理。**

## 2. 技术栈 (Tech Stack)


| **领域**        | **技术选型**                | **说明**                                        |
| --------------- | --------------------------- | ----------------------------------------------- |
| **Core**        | **Tauri v2 (Rust)**         | **系统底层交互、文件 I/O、高性能计算。**        |
| **Frontend**    | **React 18 + TypeScript**   | **UI 构建与逻辑处理。**                         |
| **Build**       | **Vite**                    | **极速构建工具。**                              |
| **Styling**     | **Tailwind CSS**            | **原子化样式，配合自定义 Zinc/Blue 深色主题。** |
| **Performance** | **@tanstack/react-virtual** | **实现动态高度的虚拟列表（核心依赖）。**        |
| **Icons**       | **Lucide React**            | **统一的矢量图标库。**                          |

---

## 3. 目录结构与文件职责 (File Structure & Responsibilities)

### 根目录配置

* **package.json**: 定义前端依赖。**注意**: 若添加新依赖，需确保与 React 18 和 Vite 兼容。
* **tailwind.config.js**: 定义全局设计系统。

  * **关键**: 自定义了 **colors.bg-main**, **colors.primary** **等变量。所有颜色修改应在此处进行，而非硬编码。**
* **postcss.config.js**: Tailwind 的必需配置，缺失会导致样式失效。
* **src-tauri/tauri.conf.json**: 定义窗口属性、权限列表 (Allowlist) 和打包配置。

### 后端核心 (**src-tauri/**)

* **src/lib.rs**: **Rust 业务逻辑入口**。

  * **LogEntry** **结构体**: 定义前后端数据契约。**修改此结构体必须同步修改前端 TS 类型**。


  * **search\_logs** **命令**: 目前用于生成高性能模拟数据。未来应替换为 **ripgrep** **集成。**
* **src/main.rs**: 二进制入口，仅用于启动 **lib::run()**。

### 前端核心 (**src/**)

* **main.tsx**: 应用挂载点，引入了 **index.css**。
* **index.css**: 全局样式。包含了 **自定义滚动条** **的 CSS 覆写，确保深色模式体验。**
* **App.tsx** **(★ 核心单文件)**: 目前采用单文件架构以集中管理状态。下文将详细解构。

---

## 4. 前端架构深度解析 (**src/App.tsx**)

**前端采用** **“单一数据源 + 状态提升”** **的架构模式。**

### 4.1 状态管理 (State Management)

**所有跨页面共享的状态都提升到了** **App** **根组件，通过 Props 向下传递。**

* **keywordGroups**: 关键词配置（支持热加载）。
* **workspaces**: 工作空间数据。
* **tasks**: 后台任务状态。
* **page**: 简单的路由状态控制。

### 4.2 核心业务组件详解

#### A. **HybridLogRenderer** **(混合日志渲染引擎)**

**这是实现“智能高亮”的核心。**

* **职责**: 将日志文本解析为带颜色和注释的 React 节点。
* **逻辑**:

  * **合并**: 将 **keywordGroups** **(预设规则) 和** **query** **(手动输入) 合并。**


  * **去重与排序**: 按关键词长度倒序排列（防止短词截断长词）。
  * **正则构建**: 动态生成全局正则 **RegExp**。
  * **渲染**: 命中预设规则显示 **配置颜色+中文注释**；命中手动输入显示 **轮询颜色**。

#### B. **FilterPalette** **(过滤器指令中心)**

* **职责**: 搜索框右侧的 Mega Menu，用于快速管理复杂的搜索条件。
* **关键实现**:

  * **右对齐定位**: **right-0 origin-top-right**，防止菜单溢出屏幕右边缘。


  * **增量/减量逻辑**: 点击未选中项 -> **query += |pattern**；点击选中项 -> **query** **移除该 pattern。**

#### C. **SearchPage** **(日志列表页)**

* **职责**: 高性能展示日志。
* **关键技术**:

  * **虚拟滚动**: 仅渲染视口内的 DOM。


  * **动态高度**: 使用 **ref={rowVirtualizer.measureElement}**。这允许日志内容**自动换行** **(**whitespace-pre-wrap**)，而列表不会重叠。**
  * **Grid 布局**: 使用 **grid-cols-[...]** **确保 Level/Time/File 列严格对齐，Content 列自适应。**

#### D. **KeywordModal** **(配置表单)**

* **职责**: 关键词组的增删改查。
* **交互**: 支持在一个组内动态添加多行 **Regex + Comment**。

### 4.3 样式系统 (The Color System)

**这是为了解决 Tailwind JIT 限制而设计的特殊模块。**

* **常量**: **COLOR\_STYLES**
* **问题**: Tailwind 无法扫描动态类名（如 **bg-\${color}-500**）。
* **解决方案**: 预定义所有颜色的完整类名对象。

  **code**TypeScript

  ```
  const COLOR_STYLES = {
    blue: { dot: "bg-blue-500", badge: "...", ... },
    red:  { dot: "bg-red-500",  badge: "...", ... },
    // ...
  }
  ```
* **规范**: 组件中必须通过 **COLOR\_STYLES[color].element** **访问样式。**

## 5. 关键开发协议 (Critical Developer Protocols)

**AI 或开发者在修改代码时，必须严格遵守以下协议：**

### ⚠️ 1. 样式与渲染

* **禁止动态拼接类名**: 严禁使用 **className={**bg-\${color}-500**}**。必须使用 **COLOR\_STYLES** **映射表。**
* **保留 measureElement**: 在修改 **SearchPage** **的列表项结构时，**必须保留 **ref={rowVirtualizer.measureElement}**，否则长文本换行会导致列表布局崩溃。

### ⚠️ 2. 状态更新

* **使用函数式更新**: 在处理删除或并发修改时（如删除关键词、更新任务状态），必须使用回调形式：

  * **✅** **setTasks(prev => prev.filter(...))**

  * **❌** **setTasks(tasks.filter(...))**
  * **原因: 闭包陷阱会导致读取到旧数据，表现为“点击无反应”。**

### ⚠️ 3. 事件处理

* **阻止冒泡**: 封装的 **Button** **组件已内置** **e.stopPropagation()**。
* **交互反馈**: 涉及删除等危险操作，虽然目前为了流畅性移除了 **confirm**，但建议保留明显的 UI 反馈（如 Hover 变红）。

### ⚠️ 4. 接口一致性

* **Rust <-> TS**: 若修改 **src-tauri/src/lib.rs** **中的** **LogEntry** **结构，必须立即同步更新** **src/App.tsx** **中的** **interface LogEntry**，否则会导致前端解析失败。

## 6. 常见问题排查 (Troubleshooting)

* **现象**: 修改了 **tailwind.config.js** **但颜色没变。**

  * **解法**: 重启 **npm run tauri dev**，Vite 需要重新生成 CSS 上下文。
* **现象**: 点击“Filter”菜单，菜单被右侧窗口遮挡。

  * **解法**: 检查 **FilterPalette** **是否使用了** **right-0** **类名。**
* **现象**: 日志列表滚动时出现空白或抖动。

  * **解法**: 检查 **SearchPage** **的** **estimateSize** **是否设置合理（当前为 46px），以及** **overscan** **是否足够（当前为 15）。**

---

**此文档旨在作为项目的“单一真理来源 (Source of Truth)”。所有功能扩展（如接入真实文件读取、添加新图表）都应在此架构基础上进行。**
