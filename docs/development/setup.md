# 环境与启动

## 前置环境

准备 Node.js `>= 22.12.0`、npm `>= 10`、Git，以及当前系统的 [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)。Rust toolchain 会由 `log-analyzer/src-tauri/rust-toolchain.toml` 固定到 `1.88.0`。

## 安装依赖

应用依赖和文档依赖分别使用独立 lockfile：

```bash
# 文档站（仓库根目录）
npm ci

# 桌面应用
cd log-analyzer
npm ci
```

## 开发模式

```bash
cd log-analyzer

# 仅启动 React / Vite 前端
npm run dev

# 启动完整 Tauri 桌面应用
npm run tauri dev
```

前端-only 模式适合组件和样式开发，但调用真实 Tauri command 时需要完整桌面模式。

## 文档站

```bash
# 从仓库根目录运行
npm run docs:dev
npm run docs:build
npm run docs:preview
```

本地 dev server 使用 `/`。生产构建默认使用 GitHub Pages 子路径 `/log-analyzer_rust/`；可以用 `DOCS_BASE=/custom/ npm run docs:build` 覆盖。

## 生产与烟雾构建

```bash
cd log-analyzer
npm run build
npm run tauri build

# 较快的桌面调试烟雾构建
npm run tauri build -- --debug --no-bundle
```

## 常见环境问题

- Linux 缺少 WebKitGTK / GTK 系统库：按 Tauri prerequisites 安装，CI 的统一清单在 `.github/actions/setup-tauri-linux/action.yml`。
- Rust 版本不一致：从 `log-analyzer/src-tauri/` 运行 `rustc --version`，确认目录 toolchain 生效。
- Node 版本过低：仓库要求 22.12.0 或更高，CI 使用 Node 22。
- macOS 首次启动权限问题：确认终端和构建产物拥有所需文件访问权限。

