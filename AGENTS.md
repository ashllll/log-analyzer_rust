# Repository Guidelines

## 项目结构与模块组织
- 根目录 `log-analyzer/` 为主工程；`src/` 存放 React 界面与业务逻辑（pages、components、contexts、services、types、utils），`public/` 为静态资源。
- `src-tauri/src/` 为 Rust 后端（models/services/utils 等），`src-tauri/tests/` 为集成测试，`src-tauri/binaries/` 存放跨平台 `unrar` 可执行文件，发布和本地运行都依赖。
- `docs/` 汇总交付与参考文档，`setup_log_analyzer.sh` 用于一键初始化开发环境。

## 构建、测试与开发命令
- 安装依赖：`cd log-analyzer && npm install`（Node 18+）。
- 本地调试：`npm run tauri dev`（启动 Tauri + Vite HMR）。
- 前端构建：`npm run build`（tsc 检查 + Vite 打包）；发布包：`npm run tauri build`。
- 质量检查：`npm run lint` / `npm run lint:fix`（ESLint TypeScript/React 规则）。
- 后端测试：`cargo test --manifest-path src-tauri/Cargo.toml`；修改 Rust 代码后同时运行 `cargo fmt`（默认风格）以保持一致性。

## 编码风格与命名约定
- TypeScript 遵循 ESLint 配置，保持 2 空格缩进、双引号、尾随逗号最小化；组件/类型用 PascalCase，变量与函数用 camelCase，常量用 SCREAMING_SNAKE_CASE。
- UI 样式优先 Tailwind Utility 类；文案走 `src/i18n` 字典，不直接写死字符串；自定义 Hooks 以 `use` 前缀。
- Rust 模块与文件使用 snake_case，类型与 trait 用 CamelCase；避免宏滥用，关注错误传播使用 ` anyhow::Result` / `?`。

## 测试指南
- Rust 侧在相关模块附近添加 `#[test]`，复杂路径/压缩/搜索逻辑可放入 `src-tauri/tests/`；提交前至少跑一次 `cargo test`。
- 前端当前未集成自动化测试脚本，新增复杂交互时建议补充轻量单元测试（Vitest/RTL）或提供可复现的手工验证步骤，并保证通过 `npm run lint`。

## 提交与 Pull Request
- 提交信息用祈使句，推荐 `feat|fix|chore|docs(scope): summary`，便于与语义化版本及 CHANGELOG 对齐；一次提交聚焦单一职责。
- PR 需包含：变更摘要、涉及模块、测试结果（lint/cargo test/手工验证）；UI 变更附截图或录屏；涉及文档或配置的更新请在描述中标明。

## 安全与配置提示
- 本地要求 Node 18+ 与 Rust 1.70+；遵循 Tauri 官方依赖安装指引，确保 `src-tauri/binaries/` 未被误删。
- 不要提交真实日志、索引或体积巨大的压缩包；如需示例，使用最小化匿名样本。
- 涉及文件路径操作时保持跨平台写法（正斜杠、避免硬编码盘符），与现有 `PathBuf` 封装保持一致。
