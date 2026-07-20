# 快速开始

从拉取源码到完成第一次搜索，通常只需要准备 Node.js、Rust 与当前平台对应的 Tauri 2 系统依赖。

## 环境要求

| 工具 | 版本 / 说明 |
| --- | --- |
| Node.js | `>= 22.12.0` |
| npm | `>= 10` |
| Rust | `1.88.0`，由 `log-analyzer/src-tauri/rust-toolchain.toml` 固定 |
| 系统依赖 | 按 [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) 为当前系统安装 |

::: tip 想先阅读，不运行应用？
文档站自身只需要 Node.js。在仓库根目录执行 `npm ci && npm run docs:dev` 即可。
:::

## 安装并启动

```bash
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer
npm install
npm run tauri dev
```

`npm run dev` 只启动 Vite 前端，适合界面开发；`npm run tauri dev` 同时启动 Rust 后端和桌面窗口，是验证真实 IPC 行为的入口。

## 第一次分析

1. 打开侧边栏中的 **Workspaces**。
2. 选择 **Import Folder** 导入日志目录，或用 **Import File** 导入单文件 / 归档包。
3. 等待工作区状态从 `PROCESSING` 变为 `READY`。
4. 选中工作区后进入 **Search Logs**。
5. 输入 `timeout|retry|circuit breaker`，点击 **Search**。
6. 如果结果过多，再添加时间范围、日志级别或文件模式。

![搜索结果页面](../assets/readme/search-results.png)

## 选择正确的导入方式

| 场景 | 建议 |
| --- | --- |
| 一次性事故包 | 直接导入 ZIP / TAR / GZ / 7Z 等归档文件 |
| 持续增长的本地目录 | 导入目录，并在工作区中开启监听 |
| 单个小日志 | 导入文件，快速创建独立工作区 |
| 多次收到相同内容 | 直接导入；CAS 会按内容哈希去重 |

## 下一步

- [工作区与导入](./workspaces.md)：理解状态、归档提取与目录监听。
- [搜索与过滤](./search.md)：掌握 OR 查询、正则与组合过滤。
- [功能概览](./features.md)：快速查看产品能力与边界。
- [故障排查](../operations/troubleshooting.md)：启动或搜索遇到问题时从这里开始。

