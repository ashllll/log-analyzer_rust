# Log Analyzer (Rust · Tauri · React)

桌面端日志检索与可视化工具，前端用 React + Tailwind，壳子是 Tauri（Rust）。当前 UI 支持正则搜索、虚拟列表展示、基础工作区管理等核心演示能力，便于在本地快速调试日志文件。

## 快速开始
- 前置：Node.js 18+、Rust（cargo）以及 Tauri 所需系统依赖。
- 安装：`cd log-analyzer && npm install`
- 开发调试：`npm run tauri dev`
- 构建应用：`npm run tauri build`
- 如需一键初始化模板，可参考仓库根目录的 `setup_log_analyzer.sh`。

## 目录结构
- `log-analyzer/`：Tauri + React 前端源码与 `src-tauri` 后端。
- `setup_log_analyzer.sh`：脚本化创建/初始化 Tauri React 模板的示例。

## 功能速览
- 正则日志搜索，虚拟滚动快速渲染长列表。
- 分级展示日志级别、时间、来源文件与行号。
- 详情侧栏展示上下文片段与标签标注。
- 工作区与任务视图样板，可扩展对多目录/多环境的管理。

## 开发提示
- 代码风格走 KISS/DRY，不要往里塞花哨的抽象。
- Tauri 调用写在 Rust `src-tauri`，前端侧通过 `@tauri-apps/api` 的 `invoke` 交互。
- Tailwind 主题色、排版集中在 `src/index.css` 与 `tailwind.config.js`，需要调整风格时优先修改这些入口。
