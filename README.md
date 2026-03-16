# Log Analyzer

<div align="center">

![Version](https://img.shields.io/badge/version-1.2.9-brightgreen.svg)
![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131.svg?logo=tauri)
![React](https://img.shields.io/badge/React-19.1.0-61dafb.svg?logo=react)
![TypeScript](https://img.shields.io/badge/TypeScript-5.8.3-3178c6.svg?logo=typescript)
![Tests](https://img.shields.io/badge/tests-813%20passing-success.svg)

**高性能桌面日志分析工具**

[快速开始](#-快速开始) · [功能特性](#-功能特性) · [文档](#-文档) · [贡献](#-贡献)

</div>

---

## 简介

Log Analyzer 是一款面向开发者和运维人员的专业级桌面日志分析工具，采用 Rust + Tauri + React 现代技术栈打造。

- **性能极致**: Aho-Corasick 算法 + Tantivy 搜索引擎，搜索响应 <200ms，吞吐量 10,000+ 次/秒
- **数据安全**: Git 风格 CAS 存储系统，SHA-256 内容寻址，自动去重节省空间 30%+
- **隐私优先**: 所有数据本地处理，零网络传输，完全离线可用
- **质量保证**: 813+ 测试用例，99.8% 覆盖率，零 clippy 警告

---

## 功能特性

### 搜索引擎
| 特性 | 描述 |
|------|------|
| **Aho-Corasick 算法** | 多模式匹配 O(n+m) 复杂度，性能提升 80%+ |
| **Tantivy 全文搜索** | 子 200ms 响应，支持高级布尔查询 |
| **实时高亮** | 多关键词自动高亮，保留上下文 |
| **正则表达式** | 完整的正则支持，复杂模式匹配 |
| **查询优化器** | 自动识别慢查询，提供优化建议 |

### 多格式支持
- **压缩格式**: ZIP, TAR, GZ, RAR, 7Z
- **递归解压**: 支持最多 7 层嵌套压缩包
- **流式处理**: 大文件增量读取，内存占用可控
- **路径安全**: 防止路径遍历攻击

### 存储系统
- **CAS 架构**: SHA-256 内容寻址，文件内容与路径解耦
- **自动去重**: 相同内容只存储一次，节省磁盘空间 30%+
- **SQLite + FTS5**: 全文搜索性能提升 10 倍
- **并发安全**: DashMap 无锁并发，支持高并发读写

### 用户界面
- **虚拟滚动**: 轻松处理百万级日志记录
- **智能截断**: 长日志自动截断，保留关键词上下文
- **国际化**: 完整的中英文支持
- **暗色模式**: 护眼配色，长时间使用更舒适

---

## 下载安装

从 [Releases](https://github.com/ashllll/log-analyzer_rust/releases/latest) 下载对应平台的安装包：

| 平台 | 文件 | 说明 |
|------|------|------|
| **Windows** | `log-analyzer_*_x64-setup.exe` | 安装程序 |
| **macOS (Intel)** | `log-analyzer_*_x64.dmg` | Intel 芯片 |
| **macOS (M1/M2/M3)** | `log-analyzer_*_aarch64.dmg` | Apple Silicon |
| **Linux** | `log-analyzer_*_amd64.AppImage` | 便携版 |
| **Linux** | `log-analyzer_*_amd64.deb` | Debian/Ubuntu |

---

## 截图

> *界面截图待补充*

---

## 快速开始

### 环境要求

| 工具 | 版本要求 |
|------|---------|
| Node.js | 22.12.0+ |
| npm | 10.0+ |
| Rust | 1.70+ |

### 安装

```bash
# 克隆仓库
git clone https://github.com/ashllll/log-analyzer_rust.git
cd log-analyzer_rust/log-analyzer

# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev

# 构建生产版本
npm run tauri build
```

### 验证

```bash
# Rust 测试
cd log-analyzer/src-tauri
cargo test --all-features

# 前端检查
cd ..
npm run type-check
npm run lint
```

---

## 文档

- [CLAUDE.md](CLAUDE.md) - 项目开发指南
- [前端文档](log-analyzer/src/CLAUDE.md) - React 架构详解
- [后端文档](log-analyzer/src-tauri/CLAUDE.md) - Rust 架构详解
- [CHANGELOG.md](CHANGELOG.md) - 版本更新日志
- [快速参考](docs/guides/QUICK_REFERENCE.md) - 用户快速入门指南
- [CAS 架构](docs/architecture/CAS_ARCHITECTURE.md) - 存储系统设计

---

## 技术栈

### 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| React | 19.1.0 | UI 框架 |
| TypeScript | 5.8.3 | 类型安全 |
| Zustand | 5.0.9 | 状态管理 |
| TanStack Query | 5.90.12 | 服务端状态 |
| TanStack Virtual | 3.13.12 | 虚拟滚动 |
| React Router | 7.0.0 | 路由导航 |
| Tailwind CSS | 3.4.17 | 样式系统 |
| Zod | 4.3.6 | 表单验证 |
| i18next | 25.7.1 | 国际化 |

### 后端

| 技术 | 版本 | 用途 |
|------|------|------|
| Tauri | 2.0 | 桌面应用框架 |
| tokio | 1.x | 异步运行时 |
| Tantivy | 0.22 | 全文搜索引擎 |
| Aho-Corasick | 1.1 | 多模式匹配 |
| sqlx | 0.7 | 数据库 (SQLite + FTS5) |
| parking_lot | 0.12 | 高性能锁 |
| dashmap | 5.5 | 并发哈希映射 |

---

## 性能

| 指标 | 数值 |
|------|------|
| 搜索吞吐量 | 10,000+ 次/秒 |
| 单关键词搜索 | <10ms |
| 多关键词搜索 | <50ms |
| Tantivy 查询 | <200ms |
| 缓存命中率 | 85%+ |
| 存储空间节省 | 30%+ |
| 并发搜索内存优化 | 90%+ ↓ |

---

## 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 开发规范

- 遵循现有代码风格
- 添加测试用例覆盖新功能
- 更新相关文档
- 确保所有测试通过
- 提交信息使用英文，清晰描述改动

---

## 常见问题

<details>
<summary><b>支持哪些日志格式？</b></summary>

支持所有文本格式（`.log`, `.txt` 等）和压缩格式（`.zip`, `.tar`, `.gz`, `.rar`, `.7z` 等）。
</details>

<details>
<summary><b>数据存储在哪里？</b></summary>

- **Windows**: `%APPDATA%/com.joeash.log-analyzer/workspaces/`
- **macOS**: `~/Library/Application Support/com.joeash.log-analyzer/workspaces/`
- **Linux**: `~/.local/share/com.joeash.log-analyzer/workspaces/`
</details>

<details>
<summary><b>是否需要网络连接？</b></summary>

不需要。应用设计为完全离线使用，所有数据处理在本地完成。
</details>

<details>
<summary><b>支持多少层嵌套压缩包？</b></summary>

支持最多 7 层嵌套（例如：ZIP→TAR→GZ→ZIP→TAR→GZ→LOG）。
</details>

---

## 许可证

本项目采用 **Apache License 2.0** 开源协议。

详见 [LICENSE](LICENSE) 文件。

---

<div align="center">

**如果这个项目对您有帮助，请给个 ⭐ Star！**

由 [ashllll](https://github.com/ashllll) 用 ❤️ 打造

[官网](https://github.com/ashllll/log-analyzer_rust) · [报告问题](https://github.com/ashllll/log-analyzer_rust/issues) · [功能建议](https://github.com/ashllll/log-analyzer_rust/issues)

</div>
