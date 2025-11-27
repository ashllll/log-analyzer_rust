# Log Analyzer (Rust · Tauri · React)

桌面端日志检索与可视化工具，前端用 React + Tailwind，壳子是 Tauri（Rust）。支持多格式压缩包解析、递归解压、索引持久化、正则搜索、虚拟列表展示等核心功能，便于在本地快速调试和分析日志文件。

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

### 核心功能
- **多格式压缩包支持**：支持 `.zip`、`.tar`、`.tar.gz`、`.tgz`、`.gz` 等常见压缩格式，RAR 格架已就绪
- **递归解压**：自动处理任意层级嵌套的压缩包（如 `.zip` 包含 `.tar.gz` 包含 `.gz`）
- **索引持久化**：导入的文件索引自动保存到磁盘，应用重启后即时加载，无需重新解压
- **灵活导入**：支持导入单个文件、压缩包或整个文件夹
- **正则日志搜索**：虚拟滚动快速渲染长列表，支持大文件高效检索
- **分级展示**：日志级别、时间戳、来源文件与行号清晰展示
- **详情侧栏**：展示上下文片段与标签标注
- **工作区管理**：多工作区支持，可管理多个日志目录或项目
- **后台任务**：导入和处理任务在后台运行，实时显示进度

### 技术特性
- **错误隔离**：单个文件处理失败不影响整体流程
- **事件驱动**：前后端通过 Tauri 事件系统实时通信
- **临时文件管理**：解压文件自动管理，应用关闭时自动清理

## 开发提示

- 代码风格走 KISS/DRY，不要往里塞花哨的抽象。
- Tauri 调用写在 Rust `src-tauri`，前端侧通过 `@tauri-apps/api` 的 `invoke` 交互。
- Tailwind 主题色、排版集中在 `src/index.css` 与 `tailwind.config.js`，需要调整风格时优先修改这些入口。
