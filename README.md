# 📊 Log Analyzer

<div align="center">

**基于 Rust + Tauri + React 的高性能桌面日志分析工具**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18+-61dafb.svg)](https://reactjs.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

支持多格式压缩包 · 递归解压 · 索引持久化 · 正则搜索 · 虚拟滚动 · Windows 兼容

[快速开始](#-快速开始) · [功能特性](#-功能特性) · [技术栈](#-技术栈) · [测试](#-测试) · [开发路线图](#-开发路线图)

</div>

---

## ✨ 项目简介

Log Analyzer 是一款专为开发者和运维人员打造的**桌面端日志分析工具**，旨在提供高效、便捷的日志检索与可视化体验。

### 为什么选择 Log Analyzer？

- 🚀 **高性能**: Rust 后端 + 并行搜索（Rayon）+ 虚拟滚动，轻松处理 GB 级日志文件
- 📦 **智能解压**: 自动识别并递归解压多层嵌套的压缩包（.zip/.tar/.gz/.rar）
- 💾 **持久化索引**: 一次导入，永久使用，索引压缩存储，应用重启即时加载
- 🎯 **精准搜索**: 正则表达式 + LRU 缓存，毫秒级响应
- 🎨 **现代 UI**: 基于 Tailwind CSS 的简洁美观界面
- 🔒 **本地优先**: 所有数据本地处理，保护隐私安全
- 🖥️ **跨平台**: Windows/macOS/Linux 完整兼容，路径处理自适应

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
| 📦 **多格式压缩包** | 支持 `.zip`、`.tar`、`.tar.gz`、`.tgz`、`.gz`、`.rar`（需系统安装 unrar） |
| 🔄 **递归解压** | 自动处理任意层级嵌套的压缩包（如 `.zip` → `.tar.gz` → `.gz`） |
| 💾 **索引持久化** | 导入一次，永久使用。索引使用 Gzip 压缩存储，节省空间 50%+ |
| 📂 **灵活导入** | 支持导入单个文件、压缩包或整个文件夹，自动识别格式 |
| 🔍 **正则搜索** | 强大的正则表达式支持 + LRU 缓存（最近 100 次搜索），精准定位目标日志 |
| ⚡ **并行搜索** | 使用 Rayon 多线程并行搜索，充分利用多核 CPU 性能 |
| 🖼️ **虚拟滚动** | 高性能渲染，轻松处理数十万条日志记录，动态高度计算 |
| 📊 **分级展示** | 清晰展示日志级别（ERROR/WARN/INFO）、时间戳、文件来源与行号 |
| 🎨 **详情侧栏** | 展示日志上下文片段，支持标签标注 |
| 🗂️ **工作区管理** | 多工作区支持，轻松管理不同项目或环境的日志 |
| ⏱️ **后台任务** | 导入和处理任务在后台运行，实时显示进度，不阻塞 UI |
| 🖥️ **Windows 兼容** | UNC 路径支持、长路径处理、只读文件自动解锁、多编码文件名识别 |

### 技术亮点

<table>
  <tr>
    <td align="center">🛡️<br/><b>错误隔离</b><br/>单个文件处理失败<br/>不影响整体流程</td>
    <td align="center">⚡<br/><b>事件驱动</b><br/>前后端通过 Tauri 事件<br/>系统实时通信</td>
    <td align="center">🗑️<br/><b>自动清理</b><br/>临时解压文件自动管理<br/>应用关闭时清理</td>
  </tr>
  <tr>
    <td align="center">🔒<br/><b>类型安全</b><br/>Rust + TypeScript<br/>双重类型保护</td>
    <td align="center">📦<br/><b>二进制序列化</b><br/>使用 bincode + Gzip<br/>索引压缩存储</td>
    <td align="center">🎯<br/><b>精准匹配</b><br/>正则引擎优化<br/>LRU 缓存加速</td>
  </tr>
  <tr>
    <td align="center">🚀<br/><b>并行处理</b><br/>Rayon 线程池<br/>多核性能最大化</td>
    <td align="center">🖥️<br/><b>跨平台优化</b><br/>Windows UNC 路径<br/>长路径支持</td>
    <td align="center">🧪<br/><b>全面测试</b><br/>13+ 单元测试<br/>5+ 集成测试</td>
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
  - `zip` 0.6 - ZIP 格式解压
  - `tar` 0.4 - TAR 归档处理
  - `flate2` 1.0 - GZIP 压缩/解压
  - `unrar` 0.5 - RAR 格式（需系统安装 unrar 命令）
- **性能优化**:
  - `rayon` 1.8 - 并行搜索，多核加速
  - `lru` 0.12 - LRU 缓存，搜索结果缓存
- **序列化**: `bincode` 1.3 - 二进制序列化（索引持久化）
- **跨平台**:
  - `dunce` 1.0 - Windows UNC 路径规范化
  - `encoding_rs` 0.8 - 多编码支持（UTF-8/GBK/Windows-1252）
- **其他**: `regex`, `uuid`, `tempfile`, `walkdir`, `chrono`

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
│  │ ZIP/TAR │  │Gzip压缩 │  │Rayon并行│  │任务进度 │ │
│  │ GZ/RAR  │  │LRU缓存  │  │正则匹配 │  │实时推送 │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────────────────────┘
                          ↓
              ┌──────────────────────┐
              │  Windows 兼容层      │
              │  • UNC 路径处理     │
              │  • 长路径支持       │
              │  • 只读文件解锁     │
              │  • 多编码识别       │
              └──────────────────────┘
```

## 🧪 测试

项目采用 Rust 最佳实践，完整的测试覆盖：

### 单元测试（lib.rs 内部）

测试**私有函数**和内部逻辑，位于 `src-tauri/src/lib.rs` 中的 `#[cfg(test)] mod tests` 模块：

- ✅ `test_canonicalize_path` - Windows UNC 路径处理
- ✅ `test_normalize_path_separator` - 跨平台路径分隔符
- ✅ `test_remove_readonly` - Windows 只读文件处理
- ✅ `test_get_file_metadata` - 文件元数据提取
- ✅ `test_parse_metadata` - 日志级别解析
- ✅ `test_safe_path_join` - 安全路径拼接
- ✅ `test_decode_filename` - 多编码文件名解码

### 集成测试（tests/ 目录）

测试**公共 API** 和整体行为，位于 `src-tauri/tests/` 目录：

- ✅ `test_tauri_app_structure` - 项目结构验证
- ✅ `test_temp_directory_operations` - 临时目录操作
- ✅ `test_file_metadata_operations` - 文件元数据操作
- ✅ `test_readonly_file_operations` - 只读文件处理
- ✅ `test_nested_directory_creation` - 嵌套目录创建

### 运行测试

```bash
# 运行所有测试
cd log-analyzer
cargo test --manifest-path=src-tauri/Cargo.toml

# 只运行单元测试
cargo test --manifest-path=src-tauri/Cargo.toml --lib

# 只运行集成测试
cargo test --manifest-path=src-tauri/Cargo.toml --test '*'
```

**测试结果**：✅ 13+ 单元测试 + 5+ 集成测试全部通过

## 🛣️ 开发路线图

### ✅ 已完成

- [x] 多格式压缩包支持（ZIP/TAR/GZ/RAR）
- [x] 递归解压机制
- [x] 索引持久化（Gzip 压缩）
- [x] 正则表达式搜索
- [x] 虚拟滚动优化
- [x] 多工作区管理
- [x] 后台任务系统
- [x] Windows 完整兼容（UNC 路径/长路径/只读文件）
- [x] 并行搜索（Rayon 多线程）
- [x] LRU 搜索缓存
- [x] 文件元数据跟踪
- [x] 全面单元测试和集成测试

### 🔜 待实现

- [ ] **增量索引更新**：自动检测文件变化，只更新修改的文件
- [ ] **高级过滤**：按时间范围、日志级别、文件来源过滤
- [ ] **搜索高亮**：搜索结果中关键词高亮显示
- [ ] **导出功能**：将搜索结果导出为 CSV/JSON
- [ ] **收藏夹**：保存常用搜索条件
- [ ] **多语言支持**：界面国际化（i18n）
- [ ] **性能监控**：显示内存使用、搜索耗时等统计信息
- [ ] **实时监听**：监听日志文件变化，自动刷新

### 💡 未来规划

- [ ] **智能分析**：自动识别异常模式，生成分析报告
- [ ] **协作功能**：分享工作区和搜索结果
- [ ] **插件系统**：支持自定义日志解析器
- [ ] **云同步**：支持工作区和索引云端备份

---

## 🤝 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)（待创建）。

## 📝 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 👏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [React](https://reactjs.org/) - 用户界面构建
- [Tailwind CSS](https://tailwindcss.com/) - CSS 框架
- [Rayon](https://github.com/rayon-rs/rayon) - Rust 并行处理库
- [Lucide Icons](https://lucide.dev/) - 精美图标库

---

<div align="center">

**如果这个项目对您有帮助，请给个 ⭐ Star ！**

由 [@ashllll](https://github.com/ashllll) 使用 ❤️ 打造

</div>
