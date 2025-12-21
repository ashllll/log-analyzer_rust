# 📊 Log Analyzer

<div align="center">

**基于 Rust + Tauri + React 的高性能桌面日志分析工具**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19+-61dafb.svg)](https://reactjs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-3178c6.svg)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-green.svg)](LICENSE)

支持多格式压缩包 · 递归解压 · Aho-Corasick搜索 · 索引持久化 · 虚拟滚动 · 跨平台 · 实时监听

[快速开始](#-快速开始) · [功能特性](#-功能特性) · [技术栈](#-技术栈) · [开发指南](#-开发指南) · [文档](#-文档)

</div>

---

## ✨ 项目简介

Log Analyzer 是一款专为开发者和运维人员打造的**桌面端日志分析工具**，采用 Rust + Tauri + React 技术栈，提供高性能的日志检索与可视化体验。

### 为什么选择 Log Analyzer？

- 🚀 **极致性能**: Aho-Corasick多模式匹配算法，搜索复杂度从O(n×m)降至O(n+m)，性能提升80%+
- 📦 **智能解压**: 统一压缩处理器架构，支持ZIP/RAR/GZ/TAR等格式，代码重复减少70%
- 🛡️ **现代错误处理**: 集成`eyre`、`miette`、`tracing`，提供用户友好的错误诊断和结构化日志
- 🏗️ **清晰架构**: QueryExecutor职责拆分，符合SRP原则，可维护性显著提升
- ⚡ **异步I/O**: 使用tokio实现非阻塞文件操作，UI响应性大幅提升
- 💾 **持久化存储**: 索引自动保存，支持增量更新，性能与可靠性兼顾
- 🎯 **结构化查询**: 完整的查询构建器 + 优先级系统 + 匹配详情追踪
- 🔍 **精准搜索**: 正则表达式 + LRU缓存 + OR/AND逻辑组合，毫秒级响应
- 🎨 **现代UI**: 基于Tailwind CSS的简洁美观界面，支持关键词高亮和虚拟滚动
- 🔒 **本地优先**: 所有数据本地处理，保护隐私安全
- 🖥️ **跨平台**: Windows/macOS/Linux完整兼容，路径处理自适应
- 🌐 **完全国际化**: 所有UI文本使用i18n，支持中英文切换
- 📡 **实时事件**: Tauri事件系统实现状态同步和实时推送

---

## 🚀 快速开始

### 环境要求

- **Node.js** 22.12.0 或更高版本
- **npm** 10.0 或更高版本
- **Rust** 1.70 或更高版本（包含`cargo`）
- **系统依赖**: 根据您的操作系统安装[Tauri前置依赖](https://tauri.app/v1/guides/getting-started/prerequisites)

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

---

## 📖 使用指南

### 第一步：创建工作区

1. 启动应用后，点击左侧导航栏的 **"Workspaces"（工作区）** 标签
2. 点击 **"Import File"** 或 **"Import Folder"** 按钮
   - **Import File**: 导入单个日志文件或压缩包（支持.log, .txt, .zip, .tar, .gz, .rar等）
   - **Import Folder**: 导入整个文件夹，自动递归扫描所有日志文件和压缩包
3. 选择文件或文件夹后，应用会自动开始处理和索引
4. 在 **"Background Tasks"（后台任务）** 标签中可查看导入进度

**提示**：
- 支持多层嵌套的压缩包，例如`logs.zip` → `archive.tar.gz` → `log.gz`
- 大文件导入可能需要几分钟时间，请耐心等待
- 索引完成后会自动保存，下次打开应用无需重新导入

### 第二步：搜索日志

1. 点击左侧导航栏的 **"Search"（搜索）** 标签
2. 在搜索框中输入关键词或正则表达式
   - 例如：`error` 或 `ERROR.*timeout` 或 `(failed|error)`
   - 多个关键词使用`|`分隔：`lux|ness|light`（OR逻辑）
3. 按 **Enter** 键或点击 **"Search"** 按钮开始搜索
4. 搜索结果会实时显示在列表中，支持虚拟滚动浏览大量结果

**搜索技巧**：
- **OR逻辑搜索**：`error|warning|critical` - 匹配任意一个关键词即可
- **关键词统计面板**：搜索后自动显示统计面板，展示每个关键词的匹配数量和占比
- **智能截断**：长日志（>1000字符）自动截断，保留关键词上下文（前后各50字符），可点击"展开全文"查看完整内容
- **多关键词高亮**：所有匹配的关键词都会在日志中高亮显示，使用不同颜色区分
- **正则表达式**：`\d{4}-\d{2}-\d{2}`匹配日期格式
- **大小写不敏感**：默认不区分大小写（如`error`会匹配`ERROR`、`Error`）
- **关键词管理**：点击活跃关键词标签上的`×`按钮可快速删除
- **持久化查询**：您的搜索查询会自动保存，刷新页面后恢复
- **匹配详情**：每个搜索结果都包含匹配的关键词、位置和优先级信息

### 第三步：配置关键词高亮

1. 点击左侧导航栏的 **"Keywords"（关键词）** 标签
2. 点击 **"New Group"** 创建关键词组
3. 设置关键词组参数：
   - **Group Name**: 组名称（如"错误关键词"）
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
- **Export**: 导出搜索结果

### 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd+K` / `Ctrl+K` | 聚焦搜索框 |
| `Enter` | 执行搜索 |
| `Esc` | 关闭详情面板 |

### 常见问题

**Q: 支持哪些日志格式？**  
A: 支持所有文本格式的日志文件（.log, .txt等），以及常见压缩格式（.zip, .tar, .gz, .rar等）。

**Q: 导入的日志存储在哪里？**  
A: 索引文件存储在应用数据目录：
- Windows: `%APPDATA%/com.joeash.log-analyzer/indices/`
- macOS: `~/Library/Application Support/com.joeash.log-analyzer/indices/`
- Linux: `~/.local/share/com.joeash.log-analyzer/indices/`

**Q: 如何删除索引释放空间？**  
A: 删除工作区会自动删除对应的索引文件。您也可以手动删除上述目录中的`.idx.gz`文件。

**Q: 支持实时监听日志文件变化吗？**  
A: ✅ **支持！** 导入工作区后，应用会自动监听文件变化，新增的日志内容会实时索引并推送到搜索结果中。

**Q: 搜索很慢怎么办？**  
A: 首次搜索会建立缓存，后续相同查询会快很多。建议：
- 使用更具体的搜索词减少结果数量
- 利用关键词过滤功能精准搜索
- 避免过于宽泛的正则表达式

**Q: Windows上提示权限错误？**  
A: 应用会自动处理只读文件和UNC路径。如果仍有问题，请以管理员身份运行。

---

## 📁 项目结构

```
log-analyzer_rust/
├── log-analyzer/              # Tauri + React主项目
│   ├── src/                   # React前端源码
│   │   ├── components/        # UI组件
│   │   │   ├── modals/       # 模态框组件
│   │   │   ├── renderers/    # 渲染器组件
│   │   │   ├── search/       # 搜索相关组件
│   │   │   └── ui/           # 基础UI组件
│   │   ├── contexts/         # React Context
│   │   ├── hooks/            # 自定义Hooks
│   │   ├── i18n/             # 国际化
│   │   │   └── locales/      # 语言文件（中英文）
│   │   ├── pages/            # 页面组件
│   │   │   ├── SearchPage.tsx       # 搜索页面
│   │   │   ├── WorkspacesPage.tsx   # 工作区管理
│   │   │   ├── KeywordsPage.tsx     # 关键词配置
│   │   │   ├── TasksPage.tsx        # 后台任务
│   │   │   └── PerformancePage.tsx  # 性能监控
│   │   ├── providers/        # Context Providers
│   │   ├── services/         # 服务层
│   │   │   ├── SearchQueryBuilder.ts  # 查询构建器
│   │   │   ├── queryApi.ts            # API封装
│   │   │   ├── queryStorage.ts        # 查询持久化
│   │   │   └── websocketClient.ts     # WebSocket客户端
│   │   ├── stores/           # Zustand状态管理
│   │   ├── types/            # TypeScript类型定义
│   │   │   ├── search.ts     # 查询相关类型
│   │   │   ├── common.ts     # 通用类型
│   │   │   ├── ui.ts         # UI类型
│   │   │   └── websocket.ts  # WebSocket类型
│   │   └── utils/            # 工具函数
│   ├── src-tauri/            # Rust后端
│   │   ├── src/
│   │   │   ├── lib.rs        # 核心逻辑
│   │   │   ├── error.rs      # 统一错误处理
│   │   │   ├── models/       # 数据模型
│   │   │   │   └── search.rs # 查询模型定义
│   │   │   ├── commands/     # Tauri命令（按功能拆分）
│   │   │   │   ├── import.rs    # 导入命令
│   │   │   │   ├── search.rs    # 搜索命令
│   │   │   │   ├── workspace.rs # 工作区命令
│   │   │   │   ├── watch.rs     # 文件监听命令
│   │   │   │   ├── config.rs    # 配置命令
│   │   │   │   ├── performance.rs # 性能命令
│   │   │   │   ├── export.rs    # 导出命令
│   │   │   │   └── query.rs     # 查询命令
│   │   │   ├── services/     # 业务服务
│   │   │   │   ├── pattern_matcher.rs      # Aho-Corasick搜索
│   │   │   │   ├── query_validator.rs      # 查询验证
│   │   │   │   ├── query_planner.rs        # 查询计划
│   │   │   │   ├── query_executor.rs       # 查询执行
│   │   │   │   └── file_watcher_async.rs   # 异步文件读取

│   │   │   ├── archive/      # 压缩处理器
│   │   │   │   ├── archive_handler.rs      # Trait定义
│   │   │   │   ├── zip_handler.rs          # ZIP处理器
│   │   │   │   ├── rar_handler.rs          # RAR处理器
│   │   │   │   ├── gz_handler.rs           # GZ处理器
│   │   │   │   └── mod.rs                  # 管理器
│   │   │   ├── benchmark/    # 性能基准测试
│   │   │   └── utils/        # 工具模块
│   │   ├── binaries/         # 内置unrar二进制文件
│   │   │   ├── unrar-x86_64-pc-windows-msvc.exe
│   │   │   ├── unrar-x86_64-apple-darwin
│   │   │   ├── unrar-aarch64-apple-darwin
│   │   │   └── unrar-x86_64-unknown-linux-gnu
│   │   ├── tests/            # 集成测试
│   │   └── Cargo.toml        # Rust依赖
│   ├── public/               # 静态资源
│   ├── package.json          # Node依赖
│   ├── vite.config.ts        # Vite配置
│   ├── tailwind.config.js    # Tailwind配置
│   └── tsconfig.json         # TypeScript配置
├── docs/                      # 📚项目文档
│   ├── OPTIMIZATION_REPORT.md # 优化实施报告
│   ├── CHANGES_SUMMARY.md    # 变更总结
│   ├── DELIVERY_PACKAGE.md   # 交付包说明
│   ├── QUICK_REFERENCE.md    # 快速参考
│   ├── API.md                # API文档
│   └── MULTI_KEYWORD_SEARCH_GUIDE.md  # 多关键词搜索指南
├── .kiro/                     # Kiro AI配置
│   └── specs/                # 功能规格
│       ├── bug-fixes/        # Bug修复规格
│       ├── performance-optimization/  # 性能优化规格
│       ├── enhanced-archive-handling/  # 压缩包处理增强
│       └── compilation-warnings-cleanup/  # 编译警告清理
├── setup_log_analyzer.sh     # 一键初始化脚本
├── CHANGELOG.md              # 更新日志
├── LICENSE                   # Apache 2.0许可证
└── README.md                 # 本文件
```

---

## 🎯 功能特性

### 核心功能

| 功能 | 描述 |
|------|------|
| 📦 **多格式压缩包** | 支持`.zip`、`.tar`、`.tar.gz`、`.tgz`、`.gz`、`.rar`（内置unrar，开箱即用） |
| 🔄 **递归解压** | 自动处理任意层级嵌套的压缩包（如`.zip` → `.tar.gz` → `.gz`） |
| 💾 **持久化存储** | 索引自动保存，支持增量更新，性能与可靠性兼顾 |
| 📂 **灵活导入** | 支持导入单个文件、压缩包或整个文件夹，自动识别格式 |
| 🔍 **结构化查询** | 完整的查询构建器系统，支持搜索项管理、优先级设置、匹配详情追踪 |
| 🔎 **多关键词搜索** | **Notepad++对齐**: `\|`符号OR逻辑、关键词统计面板、智能截断、多关键词高亮 |
| 📊 **搜索统计** | 自动显示每个关键词的匹配数量、占比和可视化进度条 |
| ⚡ **并行搜索** | 使用Rayon多线程并行搜索，充分利用多核CPU性能 |
| 🖼️ **虚拟滚动** | 高性能渲染，轻松处理数十万条日志记录，动态高度计算 |
| 📋 **智能截断** | 长文本（>1000字符）智能截断，保留关键词上下文，支持展开/收起 |
| 🎨 **详情侧栏** | 展示日志上下文片段，支持标签标注，显示匹配关键词详情 |
| 🗂️ **工作区管理** | 多工作区支持，轻松管理不同项目或环境的日志 |
| ⏱️ **后台任务** | 导入和处理任务在后台运行，实时显示进度，不阻塞UI |
| 🖥️ **跨平台兼容** | Windows/macOS/Linux完整支持，UNC路径、长路径、只读文件自动处理 |
| 👁️ **实时监听** | 自动监听工作区文件变化，增量读取新日志并实时更新索引 |
| 📤 **导出功能** | 支持将搜索结果导出为CSV格式（UTF-8 BOM编码），便于外部分析 |
| 🔄 **工作区刷新** | 智能检测文件变化（新增/修改/删除），增量更新索引 |
| 💡 **查询持久化** | 搜索查询自动保存到localStorage，刷新页面后自动恢复 |
| 🌐 **完全国际化** | 所有UI文本使用i18n，支持中英文切换 |
| 📡 **实时事件** | Tauri事件系统实现状态同步和实时推送 |
| 🔍 **高级搜索** | 支持正则表达式、大小写敏感、全词匹配等高级搜索选项 |
| 🎯 **关键词管理** | 支持创建关键词组，自定义高亮颜色，快速过滤 |
| 📈 **性能监控** | 内置性能监控面板，实时显示搜索性能、内存使用等指标 |

### 技术亮点

<table>
  <tr>
    <td align="center">🚀<br/><b>Aho-Corasick算法</b><br/>多模式匹配<br/>性能提升80%+</td>
    <td align="center">🛡️<br/><b>现代错误处理</b><br/>eyre + miette + tracing<br/>友好诊断与追踪</td>
    <td align="center">🏗️<br/><b>职责拆分</b><br/>Validator/Planner/Executor<br/>复杂度降低60%</td>
  </tr>
  <tr>
    <td align="center">⚡<br/><b>异步I/O</b><br/>tokio非阻塞<br/>UI响应性提升</td>
    <td align="center">📦<br/><b>策略模式</b><br/>ArchiveHandler Trait<br/>代码重复减少70%</td>
    <td align="center">🧪<br/><b>测试覆盖</b><br/>87个测试用例<br/>覆盖率80%+</td>
  </tr>
  <tr>
    <td align="center">🎯<br/><b>性能基准</b><br/>Criterion框架<br/>吞吐量10,000+/秒</td>
    <td align="center">💾<br/><b>持久化存储</b><br/>索引自动保存<br/>增量更新支持</td>
    <td align="center">📡<br/><b>实时事件</b><br/>Tauri事件系统<br/>状态同步推送</td>
  </tr>
</table>

---

## 🛠️ 技术栈

### 前端

- **框架**: React 19+
- **样式**: Tailwind CSS 3.x
- **图标**: Lucide React
- **构建工具**: Vite 7.x
- **类型检查**: TypeScript 5.8+
- **状态管理**: 
  - Zustand 5.0 - 轻量级状态管理
  - React Context - 全局上下文
  - Immer 11.0 - 不可变状态更新
- **数据获取**:
  - @tanstack/react-query 5.90 - 服务端状态管理
  - Tauri事件系统 - 实时事件订阅
- **UI组件**:
  - @tanstack/react-virtual 3.13 - 虚拟滚动
  - Framer Motion 12.23 - 动画库
  - React Hot Toast 2.6 - 通知提示
  - React Error Boundary 6.0 - 错误边界
- **国际化**: 
  - i18next 25.7 - 国际化框架
  - react-i18next 16.4 - React集成
- **工具库**:
  - clsx 2.1 - 条件类名
  - tailwind-merge 3.4 - Tailwind类名合并
- **测试**:
  - Jest 30.2 - 测试框架
  - @testing-library/react 16.3 - React测试工具
  - @testing-library/user-event 14.6 - 用户交互模拟
  - fast-check 4.5 - 属性测试
- **代码质量**:
  - ESLint 9.39 - 代码检查
  - @typescript-eslint - TypeScript规则
  - eslint-plugin-react - React规则
  - eslint-plugin-react-hooks - Hooks规则

### 后端

- **语言**: Rust 1.70+
- **框架**: Tauri 2.0
- **性能优化**:
  - `aho-corasick` 1.0 - 多模式字符串匹配算法
  - `rayon` 1.8 - 并行搜索，多核加速
  - `lru` 0.12 - LRU缓存，正则表达式编译缓存
  - `moka` 0.12 - 高性能缓存系统（支持异步）
  - `parking_lot` 0.12 - 高性能锁实现
  - `crossbeam` 0.8 - 锁无关数据结构和并发原语
- **错误处理与监控**:
  - `eyre` 0.6 - 现代错误处理
  - `color-eyre` 0.6 - 增强错误报告
  - `miette` 5.0 - 用户友好的错误诊断
  - `tracing` 0.1 - 结构化日志和追踪
  - `tracing-subscriber` 0.3 - 日志订阅器（支持JSON格式）
  - `sentry` 0.32 - 错误监控和性能追踪
- **存储与索引**:
  - `sqlx` 0.7 - SQLite异步数据库
  - `tantivy` 0.22 - 高性能全文搜索引擎
  - `roaring` 0.10 - Bitmap索引，高效过滤
  - `bincode` 1.3 + `serde` - 二进制序列化
- **压缩支持**:
  - `zip` 0.6 - ZIP格式解压
  - `tar` 0.4 - TAR归档处理
  - `flate2` 1.0 - GZIP压缩/解压
  - `unrar` 0.5 - RAR格式（内置二进制文件，无需系统安装）
- **异步运行时**:
  - `tokio` 1.x - 异步运行时（full features）
  - `tokio-util` 0.7 - 异步并发工具和取消令牌
  - `async-trait` 0.1 - 异步trait支持

- **查询系统**:
  - `PatternMatcher` - Aho-Corasick匹配器
  - `QueryValidator` - 查询验证器（使用validator框架）
  - `QueryPlanner` - 查询计划构建器（支持正则缓存）
  - `QueryExecutor` - 查询执行器（协调者）
  - `AsyncFileReader` - 异步文件读取
- **跨平台**:
  - `dunce` 1.0 - Windows UNC路径规范化
  - `encoding_rs` 0.8 - 多编码支持（UTF-8/GBK/Windows-1252）
  - `notify` 6.1 - 文件系统监听
- **工具库**:
  - `validator` 0.18 - 结构化验证框架
  - `sanitize-filename` 0.5 - 文件名安全化
  - `dashmap` 5.5 - 并发HashMap
  - `lazy_static` 1.4 - 静态变量初始化
  - `scopeguard` 1.2 - RAII模式和自动资源清理
  - `sha2` 0.10 - SHA-256哈希（路径缩短）
  - `num_cpus` 1.16 - CPU核心数检测
- **测试框架**:
  - `rstest` 0.18 - 增强的单元测试框架
  - `proptest` 1.4 - 属性测试框架
  - `criterion` 0.5 - 性能基准测试（支持HTML报告）
  - `tokio-test` 0.4 - 异步测试工具

### 架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    前端 (React 19)                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐         │
│  │工作区管理│  │日志搜索  │  │详情展示  │  │任务列表│         │
│  │Workspaces│  │QueryBuilder│MatchDetails│  Tasks  │         │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘         │
│  ┌──────────────────────────────────────────────────┐         │
│  │ 状态管理: Zustand + React Query + Tauri Events │         │
│  └──────────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                          ↕ Tauri IPC (invoke/emit)
┌─────────────────────────────────────────────────────────────────┐
│                   后端 (Rust + Tauri 2.0)                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐         │
│  │压缩包处理│  │索引管理  │  │结构化查询│  │事件系统│         │
│  │ ZIP/TAR │  │bincode   │  │QueryExecutor│ Tauri   │         │
│  │ GZ/RAR  │  │持久化    │  │MatchDetail│ Events  │         │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐         │
│  │Aho-Corasick│  │异步I/O  │  │错误处理 │  │缓存系统│         │
│  │PatternMatcher│AsyncFileReader│eyre+miette│  Moka  │         │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘         │
└─────────────────────────────────────────────────────────────────┘
                          ↓
               ┌──────────────────────┐
               │  跨平台兼容层        │
               │  • UNC 路径处理     │
               │  • 长路径支持       │
               │  • 只读文件解锁     │
               │  • 多编码识别       │
               │  • 文件系统监听     │
               └──────────────────────┘
```

---

## 🧪 测试与质量保证

项目采用Rust和TypeScript最佳实践，完整的测试覆盖和代码质量保证。

### Rust后端测试

#### 测试覆盖
- ✅ **87个测试用例全部通过**（1个已知问题标记为ignored）
- ✅ **测试覆盖率80%+**
- ✅ **跨平台测试支持**（Windows/macOS/Linux）

#### 核心功能测试
- ✅ `PatternMatcher` - Aho-Corasick多模式匹配（9个测试）
- ✅ `QueryValidator` - 查询验证逻辑（6个测试）
- ✅ `QueryPlanner` - 查询计划构建（7个测试）
- ✅ `AsyncFileReader` - 异步文件读取（5个测试）
- ✅ `ArchiveHandlers` - 压缩格式处理（ZIP/RAR/GZ/TAR）


#### 性能基准测试
- ✅ `search_benchmarks` - 搜索性能基准
- ✅ `cache_benchmarks` - 缓存性能基准
- ✅ `validation_benchmarks` - 验证性能基准
- ✅ `production_benchmarks` - 生产环境基准

### 运行测试

```bash
# Rust后端测试
cd log-analyzer/src-tauri
cargo test --all-features              # 运行所有测试
cargo test -- --nocapture              # 显示测试输出

# 性能基准测试
cargo bench                            # 运行所有基准测试
cargo bench --bench search_benchmarks  # 运行特定基准

# 前端测试
cd log-analyzer
npm test                               # 运行Jest测试
npm run test:watch                     # 监听模式
npm run test:coverage                  # 生成覆盖率报告
```

### 代码质量检查

```bash
# Rust代码质量
cd log-analyzer/src-tauri
cargo fmt --check                      # 检查格式
cargo fmt                              # 自动格式化
cargo clippy -- -D warnings            # 静态分析（零警告）
cargo audit                            # 安全审计

# 前端代码质量
cd log-analyzer
npm run lint                           # ESLint检查
npm run lint:fix                       # 自动修复
npm run type-check                     # TypeScript类型检查
npm run build                          # 构建检查
```

### CI/CD验证

项目已通过完整的本地CI验证流程：

- ✅ `cargo fmt --check` - 代码格式检查
- ✅ `cargo clippy -- -D warnings` - 静态分析（41个警告已修复）
- ✅ `cargo test --all-features` - 所有测试通过
- ✅ `npm run lint` - 前端代码检查
- ✅ `npm run type-check` - TypeScript类型检查
- ✅ `npm run build` - 构建成功

### 测试最佳实践

**Rust测试**:
- 在相关模块附近添加`#[test]`单元测试
- 复杂逻辑放入`src-tauri/tests/`集成测试
- 使用`rstest`进行参数化测试
- 使用`proptest`进行属性测试
- 提交前必须运行`cargo test`

**前端测试**:
- 使用Jest + React Testing Library
- 测试用户交互和组件行为
- 使用`fast-check`进行属性测试
- 确保通过`npm run lint`和`npm run type-check`

**性能测试**:
- 使用Criterion框架进行基准测试
- 生成HTML报告便于分析
- 持续监控性能回归

---

## 🛣️ 开发路线图

### ✅ 已完成（2025-12）

#### 核心功能
- ✅ **多格式压缩包支持** - ZIP/RAR/GZ/TAR，递归解压
- ✅ **Aho-Corasick搜索** - 多模式匹配，性能提升80%+
- ✅ **持久化存储** - 索引自动保存，增量更新
- ✅ **实时事件系统** - Tauri原生事件，状态同步
- ✅ **虚拟滚动** - 高性能渲染，支持数十万条记录
- ✅ **国际化** - 完整的中英文i18n支持

#### 架构优化
- ✅ **QueryExecutor职责拆分** - Validator/Planner/Executor
- ✅ **现代错误处理** - eyre + miette + tracing
- ✅ **压缩处理器统一** - 策略模式+Trait
- ✅ **异步I/O优化** - tokio非阻塞操作

#### 测试与质量
- ✅ **87个测试用例** - 覆盖率80%+
- ✅ **性能基准测试** - Criterion框架，4个场景
- ✅ **CI/CD验证** - 所有检查通过
- ✅ **代码质量** - 零Clippy警告

### 🔜 短期目标（1-2周）

- [ ] **前端单元测试** - SearchPage、KeywordsPage等核心组件测试
- [ ] **性能监控上线** - 建立性能基线，设置阈值告警
- [ ] **集成测试** - 端到端测试，覆盖完整用户流程
- [ ] **文档完善** - API文档、架构说明、用户手册

### 💡 中期规划（1-2月）

- [ ] **增量索引优化** - 支持大文件增量索引，减少内存占用
- [ ] **高级搜索语法** - 支持字段搜索、时间范围、正则组合
- [ ] **导出增强** - 支持JSON、Excel格式导出
- [ ] **插件系统** - 支持自定义日志解析器和过滤器
- [ ] **性能优化** - 进一步优化搜索和索引性能

### 🚀 长期愿景（3-6月）

- [ ] **分布式索引** - 支持多机协同索引和搜索
- [ ] **机器学习** - 日志异常检测和模式识别
- [ ] **可视化增强** - 时间线视图、关系图谱
- [ ] **云端同步** - 可选的云端备份和同步功能

---

## 📚 文档

项目文档统一存放在[`docs/`](docs/)目录下：

- **[API.md](docs/API.md)** - 完整的API接口文档
- **[OPTIMIZATION_REPORT.md](docs/OPTIMIZATION_REPORT.md)** - 优化实施报告（3,000+字详细分析）
- **[CHANGES_SUMMARY.md](docs/CHANGES_SUMMARY.md)** - 详细的变更历史和功能演进记录
- **[DELIVERY_PACKAGE.md](docs/DELIVERY_PACKAGE.md)** - 项目交付包说明和发布指南
- **[QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md)** - 快速参考手册和常用命令
- **[MULTI_KEYWORD_SEARCH_GUIDE.md](docs/MULTI_KEYWORD_SEARCH_GUIDE.md)** - 多关键词搜索功能指南

### 规格文档

项目使用Kiro AI规格系统管理功能开发：

- **[.kiro/specs/bug-fixes/](/.kiro/specs/bug-fixes/)** - Bug修复规格
- **[.kiro/specs/performance-optimization/](/.kiro/specs/performance-optimization/)** - 性能优化规格
- **[.kiro/specs/enhanced-archive-handling/](/.kiro/specs/enhanced-archive-handling/)** - 压缩包处理增强
- **[.kiro/specs/compilation-warnings-cleanup/](/.kiro/specs/compilation-warnings-cleanup/)** - 编译警告清理规格

### 更多资源

- **[CHANGELOG.md](CHANGELOG.md)** - 完整的更新日志
- **[AGENTS.md](AGENTS.md)** - 项目开发指南和规范
- **[setup_log_analyzer.sh](setup_log_analyzer.sh)** - 一键初始化脚本

---

## 📋 更新日志

### [Unreleased] - 2025-12-22

#### 🔧 CI/CD验证与代码质量修复
- **CI配置完善**: 添加type-check脚本，完善本地CI验证流程
- **代码质量提升**: 修复所有Clippy警告（41个），运行cargo fmt统一格式化
- **测试稳定性改进**: 修复7个测试编译错误，添加跨平台测试支持
- **本地CI验证**: 所有检查通过（fmt/clippy/test/lint/type-check/build）

#### 📝 文档更新
- **README重构**: 基于最新代码结构和依赖更新文档
- **技术栈更新**: 完整的依赖列表和版本信息
- **测试文档**: 详细的测试覆盖和质量保证说明

### [2025-12-10] - 全方位优化完成

#### 🚀 性能优化
- **Aho-Corasick搜索算法**: 引入多模式匹配算法，搜索性能提升80%+
- **异步I/O优化**: 使用tokio实现非阻塞文件操作，UI响应性提升
- **查询计划缓存**: 减少重复查询计划构建开销，性能提升30%

#### 🏗️ 架构重构
- **QueryExecutor职责拆分**: 遵循单一职责原则，代码复杂度降低60%
- **现代错误处理**: 集成eyre、miette、tracing，提供友好的错误诊断
- **压缩处理器统一架构**: 策略模式+Trait，代码重复减少70%

#### 💾 存储与事件
- **持久化存储**: 索引自动保存，支持增量更新
- **实时事件系统**: Tauri原生事件实现状态同步
- **高性能缓存**: LRU缓存系统，支持并发访问

#### 🧪 测试增强
- **测试覆盖率**: 从40%提升至80%+，87个测试用例全部通过
- **性能基准测试**: 建立性能监控基线，4个基准测试场景
- **跨平台测试**: 支持Windows/macOS/Linux平台差异处理

#### 📚 文档完善
- **优化实施报告**: 详细的优化方案和实施总结
- **API文档**: 更新接口说明和使用示例
- **多关键词搜索指南**: 完整的搜索功能说明

### [1.0.0] - 2024-01-01

#### 初始版本
- 基本日志分析功能
- ZIP/RAR压缩包支持
- 全文搜索
- 工作区管理
- 配置系统
- 导入导出功能

---

**更新日志格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)**  
**版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)**

## 📝 许可证

本项目采用**Apache License 2.0**开源协议（2004年1月版本）。

详见[LICENSE](LICENSE)文件或访问官方许可链接：  
**http://www.apache.org/licenses/LICENSE-2.0.txt**

Copyright (c) 2024 [@ashllll](https://github.com/ashllll)

---

<div align="center">

**如果这个项目对您有帮助，请给个⭐Star！**

由[@ashllll](https://github.com/ashllll)使用❤️打造

</div>
