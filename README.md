# 📊 Log Analyzer

<div align="center">

**基于 Rust + Tauri + React 的高性能桌面日志分析工具**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18+-61dafb.svg)](https://reactjs.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

支持多格式压缩包 · 递归解压 · 索引持久化 · 正则搜索 · 虚拟滚动

[快速开始](#-快速开始) · [功能特性](#-功能特性) · [技术栈](#-技术栈) · [开发指南](#-开发指南)

</div>

---

## ✨ 项目简介

Log Analyzer 是一款专为开发者和运维人员打造的**桌面端日志分析工具**，旨在提供高效、便捷的日志检索与可视化体验。

### 为什么选择 Log Analyzer？

- 🚀 **高性能**: Rust 后端 + 虚拟滚动，轻松处理 GB 级日志文件
- 📦 **智能解压**: 自动识别并递归解压多层嵌套的压缩包
- 💾 **持久化索引**: 一次导入，永久使用，无需重复解压
- 🎯 **精准搜索**: 支持正则表达式，快速定位目标日志
- 🎨 **现代 UI**: 基于 Tailwind CSS 的简洁美观界面
- 🔒 **本地优先**: 所有数据本地处理，保护隐私安全

## 🚀 快速开始

### 环境要求

- **Node.js** 18.0 或更高版本
- **Rust** 1.70 或更高版本（包含 `cargo`）
- **系统依赖**: 根据您的操作系统安装 [Tauri 前置依赖](https://tauri.app/v1/guides/getting-started/prerequisites)

### 安装步骤

```bash
# 1. 克隆仓库
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer

# 2. 安装依赖
npm install

# 3. 启动开发服务器
npm run tauri dev

# 4. 构建生产版本（可选）
npm run tauri build
```

### 一键初始化

如果您想从头创建项目，可以使用根目录的脚本：

```bash
bash setup_log_analyzer.sh
```

## 📁 项目结构

```
log-analyzer_rust/
├── log-analyzer/              # Tauri + React 主项目
│   ├── src/                   # React 前端源码
│   │   ├── App.tsx           # 主应用组件
│   │   └── index.css         # Tailwind 样式
│   ├── src-tauri/            # Rust 后端
│   │   ├── src/
│   │   │   └── lib.rs        # 核心逻辑
│   │   └── Cargo.toml        # Rust 依赖
│   └── package.json          # Node 依赖
├── setup_log_analyzer.sh     # 一键初始化脚本
└── README.md                 # 本文件
```

## 🎯 功能特性

### 核心功能

| 功能 | 描述 |
|------|------|
| 📦 **多格式压缩包** | 支持 `.zip`、`.tar`、`.tar.gz`、`.tgz`、`.gz` 等常见格式，RAR 支持框架已就绪 |
| 🔄 **递归解压** | 自动处理任意层级嵌套的压缩包（如 `.zip` → `.tar.gz` → `.gz`） |
| 💾 **索引持久化** | 导入一次，永久使用。索引自动保存到磁盘，应用重启后即时加载 |
| 📂 **灵活导入** | 支持导入单个文件、压缩包或整个文件夹，自动识别格式 |
| 🔍 **正则搜索** | 强大的正则表达式支持，精准定位目标日志 |
| ⚡ **虚拟滚动** | 高性能渲染，轻松处理数十万条日志记录 |
| 📊 **分级展示** | 清晰展示日志级别（ERROR/WARN/INFO）、时间戳、文件来源与行号 |
| 🎨 **详情侧栏** | 展示日志上下文片段，支持标签标注 |
| 🗂️ **工作区管理** | 多工作区支持，轻松管理不同项目或环境的日志 |
| ⏱️ **后台任务** | 导入和处理任务在后台运行，实时显示进度，不阻塞 UI |

### 技术亮点

<table>
  <tr>
    <td align="center">🛡️<br/><b>错误隔离</b><br/>单个文件处理失败<br/>不影响整体流程</td>
    <td align="center">⚡<br/><b>事件驱动</b><br/>前后端通过 Tauri 事件<br/>系统实时通信</td>
    <td align="center">🗑️<br/><b>自动清理</b><br/>临时解压文件自动管理<br/>应用关闭时清理</td>
  </tr>
  <tr>
    <td align="center">🔒<br/><b>类型安全</b><br/>Rust + TypeScript<br/>双重类型保护</td>
    <td align="center">📦<br/><b>二进制序列化</b><br/>使用 bincode 高效<br/>存储索引数据</td>
    <td align="center">🎯<br/><b>精准匹配</b><br/>正则引擎优化<br/>搜索性能卓越</td>
  </tr>
</table>

## 🛠️ 技术栈

### 前端

- **框架**: React 18+
- **样式**: Tailwind CSS 3.x
- **图标**: Lucide React
- **构建工具**: Vite
- **类型检查**: TypeScript

### 后端

- **语言**: Rust 1.70+
- **框架**: Tauri 2.0
- **压缩支持**:
  - `zip` - ZIP 格式
  - `tar` - TAR 归档
  - `flate2` - GZIP 压缩
  - `unrar` - RAR 格式（框架）
- **序列化**: `bincode` - 二进制序列化
- **其他**: `regex`, `uuid`, `tempfile`, `walkdir`

### 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                    前端 (React)                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │工作区管理│  │日志搜索  │  │详情展示  │  │任务列表│ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────────────────────┘
                          ↕ Tauri IPC (invoke/emit)
┌─────────────────────────────────────────────────────────┐
│                   后端 (Rust)                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │压缩包处理│  │索引管理  │  │全文搜索  │  │事件系统│ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────────────────────┘
```
