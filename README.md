# 📊 Log Analyzer

<div align="center">

**基于 Rust + Tauri + React 的高性能桌面日志分析工具**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18+-61dafb.svg)](https://reactjs.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

支持多格式压缩包 · 递归解压 · 索引持久化 · 正则搜索 · 虚拟滚动 · Windows 兼容

[快速开始](#-快速开始) · [功能特性](#-功能特性) · [技术栈](#-技术栈) · [测试](#-测试) · [开发路线图](#-开发路线图) · [文档](#-文档)

</div>

---

## ✨ 项目简介

Log Analyzer 是一款专为开发者和运维人员打造的**桌面端日志分析工具**，旨在提供高效、便捷的日志检索与可视化体验。

### 为什么选择 Log Analyzer？

- 🚀 **高性能**: Rust 后端 + 并行搜索（Rayon）+ 虚拟滚动，轻松处理 GB 级日志文件
- 📦 **智能解压**: 自动识别并递归解压多层嵌套的压缩包（.zip/.tar/.gz/.rar）
- 💾 **持久化索引**: 一次导入，永久使用，索引压缩存储，应用重启即时加载
- 🎯 **结构化查询**: 完整的查询构建器 + 优先级系统 + 匹配详情追踪
- 🔍 **精准搜索**: 正则表达式 + LRU 缓存 + OR/AND 逻辑组合，毫秒级响应
- 🎨 **现代 UI**: 基于 Tailwind CSS 的简洁美观界面，支持关键词高亮
- 🔒 **本地优先**: 所有数据本地处理，保护隐私安全
- 🖥️ **跨平台**: Windows/macOS/Linux 完整兼容，路径处理自适应

## 🚀 快速开始

### 下载安装

#### 方式一：下载预编译版本（推荐）

访问 [Releases 页面](https://github.com/ashllll/log-analyzer_rust/releases) 下载最新版本：

| 平台 | 文件格式 | 说明 |
|------|----------|------|
| **Windows** | `.msi` 或 `.exe` | Windows 安装程序，双击安装即可 |
| **macOS** | `.dmg` | macOS 磁盘映像，拖拽到应用程序文件夹 |
| **Linux** | `.deb` 或 `.AppImage` | Debian 包或通用应用程序 |

**安装步骤**：
1. 从 Releases 页面下载适合您系统的安装包
2. 运行安装程序并按照提示完成安装
3. 启动 Log Analyzer 应用

#### 方式二：从源码编译

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

## 📖 使用指南

### 第一步：创建工作区

1. 启动应用后，点击左侧导航栏的 **"Workspaces"（工作区）** 标签
2. 点击 **"Import File"** 或 **"Import Folder"** 按钮
   - **Import File**: 导入单个日志文件或压缩包（支持 .log, .txt, .zip, .tar, .gz, .rar 等）
   - **Import Folder**: 导入整个文件夹，自动递归扫描所有日志文件和压缩包
3. 选择文件或文件夹后，应用会自动开始处理和索引
4. 在 **"Background Tasks"（后台任务）** 标签中可查看导入进度

**提示**：
- 支持多层嵌套的压缩包，例如 `logs.zip` → `archive.tar.gz` → `log.gz`
- 大文件导入可能需要几分钟时间，请耐心等待
- 索引完成后会自动保存，下次打开应用无需重新导入

### 第二步：搜索日志

1. 点击左侧导航栏的 **"Search"（搜索）** 标签
2. 在搜索框中输入关键词或正则表达式
   - 例如：`error` 或 `ERROR.*timeout` 或 `(failed|error)`
   - 多个关键词使用 `|` 分隔：`lux|ness|light`（OR 逻辑）
3. 按 **Enter** 键或点击 **"Search"** 按钮开始搜索
4. 搜索结果会实时显示在列表中，支持虚拟滚动浏览大量结果

**搜索技巧**：
- **OR 逻辑搜索**：`error|warning|critical` - 匹配任意一个关键词即可
- **正则表达式**：`\d{4}-\d{2}-\d{2}` 匹配日期格式
- **大小写不敏感**：默认不区分大小写（如 `error` 会匹配 `ERROR`、`Error`）
- **关键词管理**：点击活跃关键词标签上的 `×` 按钮可快速删除
- **持久化查询**：您的搜索查询会自动保存，刷新页面后恢复
- **匹配详情**：每个搜索结果都包含匹配的关键词、位置和优先级信息
- 点击日志条目可在右侧查看详细信息

### 第三步：配置关键词高亮

1. 点击左侧导航栏的 **"Keywords"（关键词）** 标签
2. 点击 **"New Group"** 创建关键词组
3. 设置关键词组参数：
   - **Group Name**: 组名称（如 "错误关键词"）
   - **Highlight Color**: 高亮颜色（蓝/绿/橙/红/紫）
   - **Patterns**: 添加多个正则表达式和注释
4. 点击 **"Save Configuration"** 保存
5. 返回搜索页面，点击 **"Filters"** 按钮可快速应用关键词过滤

**关键词组示例**：
- **错误级别**（红色）：`ERROR`, `FATAL`, `CRITICAL`
- **警告级别**（橙色）：`WARN`, `WARNING`
- **性能问题**（紫色）：`timeout`, `slow query`, `high memory`

### 第四步：管理工作区

在 **"Workspaces"** 页面可以：
- **切换工作区**: 点击工作区卡片切换到该工作区
- **删除工作区**: 点击工作区的删除按钮（不会删除原文件）
- **查看状态**: 
  - **READY**: 已准备就绪，可以搜索
  - **PROCESSING**: 正在处理和索引
  - **OFFLINE**: 离线（原文件已移动或删除）

### 第五步：查看后台任务

点击 **"Background Tasks"** 标签可查看：
- 当前正在运行的任务
- 任务进度和状态
- 已完成或失败的任务历史

任务类型包括：
- **Import**: 导入和索引文件
- **Export**: 导出搜索结果（未来功能）

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd+K` / `Ctrl+K` | 聚焦搜索框 |
| `Enter` | 执行搜索 |
| `Esc` | 关闭详情面板 |

### 常见问题

**Q: 支持哪些日志格式？**  
A: 支持所有文本格式的日志文件（.log, .txt 等），以及常见压缩格式（.zip, .tar, .gz, .rar 等）。

**Q: 导入的日志存储在哪里？**  
A: 索引文件存储在应用数据目录：
- Windows: `%APPDATA%/com.joeash.log-analyzer/indices/`
- macOS: `~/Library/Application Support/com.joeash.log-analyzer/indices/`
- Linux: `~/.local/share/com.joeash.log-analyzer/indices/`

**Q: 如何删除索引释放空间？**  
A: 删除工作区会自动删除对应的索引文件。您也可以手动删除上述目录中的 `.idx.gz` 文件。

**Q: 支持实时监听日志文件变化吗？**  
A: ✅ **支持！** 导入工作区后，应用会自动监听文件变化，新增的日志内容会实时索引并推送到搜索结果中。

**Q: 搜索很慢怎么办？**  
A: 首次搜索会建立缓存，后续相同查询会快很多。建议：
- 使用更具体的搜索词减少结果数量
- 利用关键词过滤功能精准搜索
- 避免过于宽泛的正则表达式

**Q: Windows 上提示权限错误？**  
A: 应用会自动处理只读文件和 UNC 路径。如果仍有问题，请以管理员身份运行。

## 📁 项目结构

```
log-analyzer_rust/
├── log-analyzer/              # Tauri + React 主项目
│   ├── src/                   # React 前端源码
│   │   ├── App.tsx           # 主应用组件
│   │   ├── services/         # 查询服务层
│   │   │   ├── SearchQueryBuilder.ts  # 查询构建器
│   │   │   ├── queryApi.ts            # API 封装
│   │   │   └── queryStorage.ts        # 查询持久化
│   │   ├── types/            # TypeScript 类型定义
│   │   │   └── search.ts     # 查询相关类型
│   │   └── index.css         # Tailwind 样式
│   ├── src-tauri/            # Rust 后端
│   │   ├── src/
│   │   │   ├── lib.rs        # 核心逻辑
│   │   │   ├── models/       # 数据模型
│   │   │   │   └── search.rs # 查询模型定义
│   │   │   └── services/     # 业务服务
│   │   │       └── query_executor.rs  # 查询执行器
│   │   ├── binaries/         # 内置 unrar 二进制文件
│   │   │   ├── unrar-x86_64-pc-windows-msvc.exe
│   │   │   ├── unrar-x86_64-apple-darwin
│   │   │   ├── unrar-aarch64-apple-darwin
│   │   │   └── unrar-x86_64-unknown-linux-gnu
│   │   └── Cargo.toml        # Rust 依赖
│   └── package.json          # Node 依赖
├── docs/                      # 📚 项目文档
│   ├── CHANGES_SUMMARY.md    # 变更总结
│   ├── DELIVERY_PACKAGE.md   # 交付包说明
│   └── QUICK_REFERENCE.md    # 快速参考
├── setup_log_analyzer.sh     # 一键初始化脚本
├── LICENSE                   # MIT 许可证
└── README.md                 # 本文件
```

## 🎯 功能特性

### 核心功能

| 功能 | 描述 |
|------|------|
| 📦 **多格式压缩包** | 支持 `.zip`、`.tar`、`.tar.gz`、`.tgz`、`.gz`、`.rar`（内置 unrar，开箱即用） |
| 🔄 **递归解压** | 自动处理任意层级嵌套的压缩包（如 `.zip` → `.tar.gz` → `.gz`） |
| 💾 **索引持久化** | 导入一次，永久使用。索引使用 Gzip 压缩存储，节省空间 50%+ |
| 📂 **灵活导入** | 支持导入单个文件、压缩包或整个文件夹，自动识别格式 |
| 🔍 **结构化查询** | 完整的查询构建器系统，支持搜索项管理、优先级设置、匹配详情追踪 |
| 🔎 **灵活搜索** | OR 逻辑组合（`lux|ness`）、正则表达式、大小写不敏感、LRU 缓存（最近 100 次） |
| ⚡ **并行搜索** | 使用 Rayon 多线程并行搜索，充分利用多核 CPU 性能 |
| 🖼️ **虚拟滚动** | 高性能渲染，轻松处理数十万条日志记录，动态高度计算 |
| 📊 **分级展示** | 清晰展示日志级别（ERROR/WARN/INFO）、时间戳、文件来源与行号 |
| 🎨 **详情侧栏** | 展示日志上下文片段，支持标签标注，显示匹配关键词详情 |
| 🗂️ **工作区管理** | 多工作区支持，轻松管理不同项目或环境的日志 |
| ⏱️ **后台任务** | 导入和处理任务在后台运行，实时显示进度，不阻塞 UI |
| 🖥️ **Windows 兼容** | UNC 路径支持、长路径处理、只读文件自动解锁、多编码文件名识别 |
| 👁️ **实时监听** | 自动监听工作区文件变化，增量读取新日志并实时更新索引 |
| 📤 **导出功能** | 支持将搜索结果导出为 CSV 格式（UTF-8 BOM 编码），便于外部分析和报表生成 |
| 🔄 **工作区刷新** | 智能检测文件变化（新增/修改/删除），增量更新索引，无变化时跳过处理 |
| 💡 **查询持久化** | 搜索查询自动保存到 localStorage，刷新页面后自动恢复 |

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
    <td align="center">🎯<br/><b>结构化查询</b><br/>完整的查询系统<br/>匹配详情追踪</td>
  </tr>
  <tr>
    <td align="center">🚀<br/><b>并行处理</b><br/>Rayon 线程池<br/>多核性能最大化</td>
    <td align="center">🖥️<br/><b>跨平台优化</b><br/>Windows UNC 路径<br/>长路径支持</td>
    <td align="center">🧪<br/><b>全面测试</b><br/>25+ 单元测试<br/>完整覆盖</td>
  </tr>
</table>

## 🛠️ 技术栈

### 前端

- **框架**: React 18+
- **样式**: Tailwind CSS 3.x
- **图标**: Lucide React
- **构建工具**: Vite
- **类型检查**: TypeScript
- **查询系统**:
  - `SearchQueryBuilder` - 流畅 API 构建器模式
  - `QueryValidation` - 查询验证系统
  - `localStorage` - 查询持久化存储

### 后端

- **语言**: Rust 1.70+
- **框架**: Tauri 2.0
- **压缩支持**:
  - `zip` 0.6 - ZIP 格式解压
  - `tar` 0.4 - TAR 归档处理
  - `flate2` 1.0 - GZIP 压缩/解压
  - `unrar` - RAR 格式（内置二进制文件，无需系统安装）
- **性能优化**:
  - `rayon` 1.8 - 并行搜索，多核加速
  - `lru` 0.12 - LRU 缓存，搜索结果缓存
- **查询系统**:
  - `QueryExecutor` - 结构化查询执行器
  - `MatchDetail` - 匹配详情追踪
  - `ExecutionPlan` - 查询执行计划（支持优先级排序）
- **序列化**: `bincode` 1.3 + `serde` - 二进制序列化（索引持久化）
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
│  │          │  │QueryBuilder│MatchDetails│          │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────────────────────┘
                          ↕ Tauri IPC (invoke/emit)
┌─────────────────────────────────────────────────────────┐
│                   后端 (Rust)                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │压缩包处理│  │索引管理  │  │结构化查询│  │事件系统│ │
│  │ ZIP/TAR │  │Gzip压缩 │  │QueryExecutor│任务进度 │ │
│  │ GZ/RAR  │  │LRU缓存  │  │MatchDetail│实时推送 │ │
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

### 单元测试

#### 核心功能测试（lib.rs）

测试**私有函数**和内部逻辑：

- ✅ `test_canonicalize_path` - Windows UNC 路径处理
- ✅ `test_normalize_path_separator` - 跨平台路径分隔符
- ✅ `test_remove_readonly` - Windows 只读文件处理
- ✅ `test_get_file_metadata` - 文件元数据提取
- ✅ `test_parse_metadata` - 日志级别解析
- ✅ `test_safe_path_join` - 安全路径拼接
- ✅ `test_decode_filename` - 多编码文件名解码

#### 查询系统测试（query_executor.rs）

测试**结构化查询**功能：

- ✅ `test_validate_empty_query` - 空查询验证
- ✅ `test_build_or_regex` - OR 逻辑正则构建
- ✅ `test_build_not_regex` - NOT 逻辑正则构建
- ✅ `test_matches_line` - 行匹配测试
- ✅ `test_filter_lines` - 批量过滤测试
- ✅ `test_match_with_details` - 匹配详情追踪
- ✅ `test_execution_plan_methods` - 执行计划方法
- ✅ `test_priority_sorting` - 优先级排序

#### 前端测试（SearchQueryBuilder.test.ts）

测试**查询构建器**功能：

- ✅ 18 个测试场景，覆盖查询构建、验证、导入导出等功能

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

**测试结果**：✅ **25+ 单元测试**全部通过（Rust 后端） + **18+ 前端测试**（TypeScript）

## 🛣️ 开发路线图

### ✅ 已完成

#### 核心功能
- [x] 多格式压缩包支持（ZIP/TAR/GZ/RAR）
- [x] 递归解压机制
- [x] 索引持久化（Gzip 压缩）
- [x] 正则表达式搜索（支持大小写不敏感）
- [x] 虚拟滚动优化
- [x] 多工作区管理
- [x] 后台任务系统
- [x] Windows 完整兼容（UNC 路径/长路径/只读文件）
- [x] 并行搜索（Rayon 多线程）
- [x] LRU 搜索缓存
- [x] 文件元数据跟踪
- [x] 全面单元测试和集成测试

#### 用户界面
- [x] 搜索关键词高亮显示（支持多颜色、自定义标注）
- [x] 工作区状态实时更新（READY/PROCESSING/OFFLINE）
- [x] 任务进度实时推送
- [x] Context API 全局状态管理
- [x] React 18+ 严格模式兼容

#### 高级功能
- [x] **实时文件监听**（自动监听工作区文件变化）
- [x] **增量日志读取**（只处理文件新增内容，避免重复索引）
- [x] **偏移量管理**（跟踪每个文件的读取位置）
- [x] **CSV 导出功能**（导出搜索结果到 CSV 文件，支持 UTF-8 BOM）
- [x] **工作区刷新**（检测文件变化并增量更新索引）
- [x] **自动版本号管理**（推送代码自动构建并递增版本号）
- [x] **高级过滤功能**（按时间范围、日志级别、文件来源过滤）
- [x] **结构化查询系统**（完整的查询构建器 + 匹配详情追踪）
- [x] **查询持久化**（自动保存查询到 localStorage，刷新后恢复）

#### 最近更新（2025-12）
- [x] **后端代码模块化清理**
  - 移除 lib.rs 中重复的遗留代码（从 3088 行精简至 1592 行）
  - 完成 models/utils/services/archive/commands 五层模块拆分
  - 消除所有重复函数定义，统一使用模块导出
- [x] **压缩包解压持久化存储**
  - 解压文件存储至应用数据目录（`extracted/{workspace_id}/`）
  - 修复压缩包导入后搜索不到结果的问题
  - 支持应用重启后继续搜索已导入的压缩包内容
- [x] **工作区切换逻辑优化**
  - 修复点击工作区时未加载后端索引的问题
  - 添加重复点击检测，避免不必要的索引重新加载
- [x] **前端性能优化**
  - HybridLogRenderer 添加 React.memo 优化
  - 提取 LogRow 独立组件减少重渲染
  - 虚拟滚动 estimateSize 简化为固定值
  - 配置保存添加 500ms 防抖机制
  - keywordGroups 缓存优化
- [x] **内置 unrar 工具**
  - 应用自带各平台 unrar 二进制文件（Windows/macOS/Linux）
  - 用户无需手动安装任何依赖，开箱即用
  - 自动检测平台并使用对应的二进制文件
  - 开发模式和发布版本均支持
- [x] **前端模块化重构完成**
  - 将所有页面组件拆分到独立文件（SearchPage/KeywordsPage/WorkspacesPage/TasksPage/PerformancePage）
  - App.tsx 精简到 66 行，仅保留布局和导航逻辑
  - 统一通过 pages/index.ts 导出页面组件
- [x] **更新 CI/CD 配置**
  - 添加 binaries 目录可执行权限设置
  - 确保跨平台构建正常工作

#### 历史更新（2024-11）
- [x] **完整实施结构化查询系统**
  - 创建 `SearchQueryBuilder` 类（流畅 API 构建器模式）
  - 实现 `QueryExecutor` 执行器（支持优先级排序）
  - 添加 `MatchDetail` 类型（追踪每个匹配的关键词、位置、优先级）
  - 实现查询持久化（localStorage 自动保存和恢复）
  - 完整的单元测试覆盖（25+ 测试全部通过）
- [x] **修复 Rust regex 不支持前向断言问题**
  - 改用字符串包含验证实现 AND 逻辑
  - 确保 OR 逻辑正常工作（`lux|ness` 匹配任意一个）
- [x] **优化搜索体验**
  - 添加关键词删除功能（悬停显示 × 按钮）
  - 统一分隔符格式（支持 `|` 无空格）
  - 自动规范化输入
- [x] **修复工作区状态更新竞态条件**
  - 移除前端手动状态更新，完全由后端事件驱动
  - 解决刷新后工作区一直显示 PROCESSING 的问题
- [x] **修复重复任务问题**
  - 使用 useRef 避免闭包陷阱
  - 防止 hot reload 期间事件监听器重复创建

### 🔜 待实现

- [ ] **JSON 导出**：支持 JSON 格式导出（CSV 已实现）
- [ ] **收藏夹**：保存常用搜索条件（查询持久化已实现）
- [ ] **多语言支持**：界面国际化（i18n）
- [ ] **性能监控 UI**：在界面显示内存使用、搜索耗时等统计信息
- [ ] **AND 逻辑搜索**：实现所有关键词都必须匹配的 AND 逻辑（当前为 OR）

### 💡 未来规划

- [ ] **智能分析**：自动识别异常模式，生成分析报告
- [ ] **协作功能**：分享工作区和搜索结果
- [ ] **插件系统**：支持自定义日志解析器
- [ ] **云同步**：支持工作区和索引云端备份

---

## 📚 文档

项目文档统一存放在 [`docs/`](docs/) 目录下：

- **[CHANGES_SUMMARY.md](docs/CHANGES_SUMMARY.md)** - 详细的变更历史和功能演进记录
- **[DELIVERY_PACKAGE.md](docs/DELIVERY_PACKAGE.md)** - 项目交付包说明和发布指南
- **[QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md)** - 快速参考手册和常用命令

## 🤝 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)（待创建）。

## 📝 许可证

本项目采用 **MIT License** 开源协议。

这意味着您可以：
- ✅ **商业使用** - 在商业项目中使用
- ✅ **修改** - 修改源代码
- ✅ **分发** - 分发软件副本
- ✅ **私用** - 私人使用

条件：
- 📄 **保留许可和版权声明** - 在所有副本中保留原始许可证和版权声明

详见 [LICENSE](LICENSE) 文件。

Copyright (c) 2024 [@ashllll](https://github.com/ashllll)

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
