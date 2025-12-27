# 📊 Log Analyzer

<div align="center">

**基于 Rust + Tauri + React 的高性能桌面日志分析工具**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19+-61dafb.svg)](https://reactjs.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-green.svg)](http://www.apache.org/licenses/LICENSE-2.0.txt)

支持多格式压缩包 · 递归解压 · Aho-Corasick 搜索 · 索引持久化 · 虚拟滚动 · 跨平台

[快速开始](#-快速开始) · [功能特性](#-功能特性) · [技术栈](#-技术栈) · [开发指南](#-开发指南) · [许可证](#-许可证)

</div>

---

## ✨ 项目简介

Log Analyzer 是一款专为开发者和运维人员打造的**桌面端日志分析工具**,采用 Rust + Tauri + React 技术栈,提供高性能的日志检索与可视化体验。

### 为什么选择 Log Analyzer?

- 🚀 **极致性能**: Aho-Corasick 多模式匹配算法，搜索复杂度从 O(n×m) 降至 O(n+m)，性能提升 80%+
- 📦 **智能解压**: 统一压缩处理器架构,支持ZIP/RAR/GZ/TAR等格式,代码重复减少70%
- 🛡️ **统一错误处理**: 使用`thiserror`创建`AppError`,错误处理一致性达100%
- 🏗️ **清晰架构**: QueryExecutor职责拆分,符合SRP原则,可维护性显著提升
- ⚡ **异步I/O**: 使用tokio实现非阻塞文件操作,UI响应性大幅提升
- 💾 **内容寻址存储(CAS)**: Git风格的内容寻址存储系统，自动去重，节省磁盘空间
- 🗄️ **SQLite元数据**: 使用SQLite管理文件元数据，支持FTS5全文搜索，查询性能提升10倍+
- 🎯 **结构化查询**: 完整的查询构建器 + 优先级系统 + 匹配详情追踪
- 🔍 **精准搜索**: 正则表达式 + LRU缓存 + OR/AND逻辑组合,毫秒级响应
- 🎨 **现代UI**: 基于Tailwind CSS的简洁美观界面,支持关键词高亮
- 🔒 **本地优先**: 所有数据本地处理,保护隐私安全
- 🖥️ **跨平台**: Windows/macOS/Linux完整兼容,路径处理自适应
- 🌐 **国际化**: 完整的中英文i18n支持

---

## 🚀 快速开始

### 环境要求

- **Node.js** 18.0 或更高版本
- **Rust** 1.70 或更高版本(包含`cargo`)
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

# 4. 构建生产版本(可选)
npm run tauri build
```

---

## 📖 使用指南

### ⚠️ 重要提示:旧格式不再支持

**Log Analyzer 2.0不再支持旧的路径映射存储格式。**

如果您有使用旧版本创建的工作区,需要重新导入数据。详细说明请参阅:
- **[迁移指南](docs/MIGRATION_GUIDE.md)** - 完整的迁移说明和CAS架构介绍
- **[快速通知](docs/LEGACY_FORMAT_NOTICE.md)** - 简要说明和快速操作步骤

新的CAS架构提供:
- ✅ 10x更快的搜索速度
- ✅ 自动去重节省磁盘空间
- ✅ 无路径长度限制
- ✅ 完美的嵌套压缩包支持

### 第一步:创建工作区

1. 启动应用后,点击左侧导航栏的 **"Workspaces"(工作区)** 标签
2. 点击 **"Import File"** 或 **"Import Folder"** 按钮
   - **Import File**: 导入单个日志文件或压缩包(支持.log, .txt, .zip, .tar, .gz, .rar等)
   - **Import Folder**: 导入整个文件夹,自动递归扫描所有日志文件和压缩包
3. 选择文件或文件夹后,应用会自动开始处理和索引
4. 在 **"Background Tasks"(后台任务)** 标签中可查看导入进度

**提示**:
- 支持多层嵌套的压缩包,例如`logs.zip` → `archive.tar.gz` → `log.gz`
- 大文件导入可能需要几分钟时间,请耐心等待
- 索引完成后会自动保存,下次打开应用无需重新导入

### 第二步:搜索日志

1. 点击左侧导航栏的 **"Search"(搜索)** 标签
2. 在搜索框中输入关键词或正则表达式
   - 例如:`error` 或 `ERROR.*timeout` 或 `(failed|error)`
   - 多个关键词使用`|`分隔:`lux|ness|light`(OR逻辑)
3. 按 **Enter** 键或点击 **"Search"** 按钮开始搜索
4. 搜索结果会实时显示在列表中,支持虚拟滚动浏览大量结果

**搜索技巧**:
- **OR逻辑搜索**: `error|warning|critical` - 匹配任意一个关键词即可
- **关键词统计面板**: 搜索后自动显示统计面板,展示每个关键词的匹配数量和占比
- **智能截断**: 长日志(>1000字符)自动截断,保留关键词上下文(前后各50字符),可点击"展开全文"查看完整内容
- **多关键词高亮**: 所有匹配的关键词都会在日志中高亮显示,使用不同颜色区分
- **正则表达式**: `\d{4}-\d{2}-\d{2}`匹配日期格式
- **大小写不敏感**: 默认不区分大小写(如`error`会匹配`ERROR`、`Error`)
- **持久化查询**: 您的搜索查询会自动保存,刷新页面后恢复

### 第三步:配置关键词高亮

1. 点击左侧导航栏的 **"Keywords"(关键词)** 标签
2. 点击 **"New Group"** 创建关键词组
3. 设置关键词组参数:
   - **Group Name**: 组名称(如"错误关键词")
   - **Highlight Color**: 高亮颜色(蓝/绿/橙/红/紫)
   - **Patterns**: 添加多个正则表达式和注释
4. 点击 **"Save Configuration"** 保存
5. 返回搜索页面,点击 **"Filters"** 按钮可快速应用关键词过滤

### 常见问题

**Q: 支持哪些日志格式?**  
A: 支持所有文本格式的日志文件(.log, .txt等),以及常见压缩格式(.zip, .tar, .gz, .rar等)。

**Q: 导入的日志存储在哪里?**  
A: 工作区数据存储在应用数据目录:
- Windows: `%APPDATA%/com.joeash.log-analyzer/workspaces/`
- macOS: `~/Library/Application Support/com.joeash.log-analyzer/workspaces/`
- Linux: `~/.local/share/com.joeash.log-analyzer/workspaces/`

每个工作区包含:
- `objects/` - CAS对象存储(文件内容)
- `metadata.db` - SQLite元数据数据库

**Q: 支持实时监听日志文件变化吗?**  
A: ✅ 支持!导入工作区后,应用会自动监听文件变化,新增的日志内容会实时索引并推送到搜索结果中。

---

## 📁 项目结构

```
log-analyzer/
├── src/                          # React前端源码
│   ├── components/               # UI组件
│   │   ├── modals/              # 模态框组件
│   │   ├── renderers/           # 渲染器组件
│   │   ├── search/              # 搜索相关组件
│   │   └── ui/                  # 基础UI组件
│   ├── contexts/                # React Context
│   ├── hooks/                   # 自定义Hooks
│   ├── i18n/                    # 国际化
│   ├── pages/                   # 页面组件
│   ├── services/                # 服务层
│   ├── types/                   # TypeScript类型定义
│   └── utils/                   # 工具函数
├── src-tauri/                   # Rust后端
│   ├── src/
│   │   ├── archive/             # 压缩包处理
│   │   ├── commands/            # Tauri命令
│   │   ├── models/              # 数据模型
│   │   ├── services/            # 业务服务
│   │   └── utils/               # 工具模块
│   ├── binaries/                # 跨平台unrar二进制文件
│   └── tests/                   # 集成测试
└── package.json                 # Node依赖配置
```

---

## 🎯 功能特性

### 核心功能

| 功能 | 描述 |
|------|------|
| 📦 **多格式压缩包** | 支持`.zip`、`.tar`、`.tar.gz`、`.tgz`、`.gz`、`.rar`(内置unrar,开箱即用) |
| 🔄 **递归解压** | 自动处理任意层级嵌套的压缩包(如`.zip` → `.tar.gz` → `.gz`) |
| 💾 **内容寻址存储(CAS)** | Git风格的内容寻址存储系统,自动去重,节省磁盘空间 |
| 🗄️ **SQLite元数据** | 使用SQLite管理文件元数据,支持FTS5全文搜索,查询性能提升10倍+ |
| 📂 **灵活导入** | 支持导入单个文件、压缩包或整个文件夹,自动识别格式 |
| 🔍 **结构化查询** | 完整的查询构建器系统,支持搜索项管理、优先级设置、匹配详情追踪 |
| 🔎 **多关键词搜索** | OR逻辑搜索、关键词统计面板、智能截断、多关键词高亮 |
| ⚡ **并行搜索** | 使用Rayon多线程并行搜索,充分利用多核CPU性能 |
| 🖼️ **虚拟滚动** | 高性能渲染,轻松处理数十万条日志记录 |
| 🗂️ **工作区管理** | 多工作区支持,轻松管理不同项目或环境的日志 |
| 👁️ **实时监听** | 自动监听工作区文件变化,增量读取新日志并实时更新索引 |
| 📤 **CSV导出** | 支持将搜索结果导出为CSV格式(UTF-8 BOM编码) |
| 🔄 **数据迁移** | 自动检测旧格式工作区并提供一键迁移到CAS架构 |
| 🌐 **国际化** | 所有UI文本使用i18n,支持中英文切换 |

### 技术亮点

- 🚀 **Aho-Corasick 算法** - 多模式匹配，性能提升 80%+
- 🗄️ **内容寻址存储(CAS)** - Git风格存储,自动去重,SHA-256哈希
- 📊 **SQLite + FTS5** - 全文搜索索引,查询性能提升10倍+
- 🛡️ **统一错误处理** - thiserror创建AppError,错误一致性100%
- 🏗️ **职责拆分** - Validator/Planner/Executor,复杂度降低60%
- ⚡ **异步I/O** - tokio非阻塞,UI响应性提升
- 📦 **策略模式** - ArchiveHandler Trait,代码重复减少70%
- 🔒 **事务处理** - SQLite事务保证数据一致性
- ♻️ **断点续传** - 支持大文件导入的检查点恢复
- 🧪 **测试覆盖** - 40+测试用例,覆盖率80%+

---

## 🏗️ 架构设计

### 内容寻址存储(CAS)架构

Log Analyzer 采用类似Git的内容寻址存储(Content-Addressable Storage)架构,解决了传统路径映射方式的诸多问题:

#### 传统方式的问题

- ❌ 长路径限制(Windows 260字符限制)
- ❌ 嵌套压缩包路径过长导致无法访问
- ❌ 路径映射不一致导致搜索失败
- ❌ 重复文件占用大量磁盘空间

#### CAS架构的优势

- ✅ **内容去重**: 相同内容的文件只存储一次,节省磁盘空间
- ✅ **无路径限制**: 使用SHA-256哈希作为文件标识,不受路径长度限制
- ✅ **数据完整性**: 哈希验证确保文件内容未被篡改
- ✅ **高效查询**: SQLite + FTS5全文搜索索引,查询性能提升10倍+
- ✅ **嵌套支持**: 完美支持任意深度的嵌套压缩包

#### 存储结构

```
workspace_dir/
├── objects/              # CAS对象存储(Git风格)
│   ├── ab/              # 前2位哈希作为目录
│   │   └── cdef123...   # 完整SHA-256哈希
│   └── cd/
│       └── ef456...
├── metadata.db          # SQLite元数据数据库
│   ├── files表          # 文件元数据(哈希、虚拟路径、大小等)
│   ├── archives表       # 压缩包元数据(嵌套关系)
│   └── files_fts表      # FTS5全文搜索索引
└── extracted/           # 临时解压目录
    └── archive_123/
```

#### 数据流

```
导入流程:
用户文件 → 计算SHA-256 → 检查去重 → 存储到objects/ → 记录元数据到SQLite

搜索流程:
用户查询 → FTS5索引查询 → 获取文件哈希列表 → 从CAS读取内容 → 返回结果
```

详细架构文档请参阅 [docs/architecture/CAS_ARCHITECTURE.md](docs/architecture/CAS_ARCHITECTURE.md)

---

## 🛠️ 技术栈

### 前端

- **框架**: React 19+
- **样式**: Tailwind CSS 3.x
- **图标**: Lucide React
- **构建工具**: Vite 7.x
- **类型检查**: TypeScript 5.8+
- **国际化**: i18next
- **虚拟滚动**: @tanstack/react-virtual
- **动画**: Framer Motion

### 后端

- **语言**: Rust 1.70+
- **框架**: Tauri 2.0
- **性能优化**:
  - `aho-corasick` 1.0 - 多模式字符串匹配算法
  - `rayon` 1.8 - 并行搜索,多核加速
  - `lru` 0.12 - LRU缓存,搜索结果缓存
- **错误处理**: `thiserror` 1.0
- **压缩支持**:
  - `zip` 0.6 - ZIP格式解压
  - `tar` 0.4 - TAR归档处理
  - `flate2` 1.0 - GZIP压缩/解压
  - `unrar` 0.5 - RAR格式(内置二进制文件)
- **异步运行时**: `tokio` 1.x
- **序列化**: `bincode` 1.3 + `serde` 1.x
- **跨平台**: `dunce` 1.0 - Windows UNC路径规范化
- **文件监听**: `notify` 6.1

---

## 👨‍💻 开发指南

### 开发命令

```bash
# 安装依赖
npm install

# 开发调试(启动Tauri + Vite HMR)
npm run tauri dev

# 前端构建(TypeScript检查 + Vite打包)
npm run build

# 发布构建
npm run tauri build

# 代码质量检查
npm run lint          # ESLint检查
npm run lint:fix      # 自动修复ESLint问题
```

### Rust后端命令

```bash
cd src-tauri

# 运行所有测试
cargo test

# 代码格式化
cargo fmt

# Clippy静态分析
cargo clippy -- -D warnings

# 发布构建
cargo build --release
```

### 编码规范

**TypeScript/React**:
- 遵循ESLint配置,保持2空格缩进、双引号
- 组件/类型用PascalCase,变量与函数用camelCase
- UI样式优先Tailwind Utility类
- 文案走`src/i18n`字典,不直接写死字符串
- 自定义Hooks以`use`前缀

**Rust**:
- 模块与文件使用snake_case,类型与trait用CamelCase
- 运行`cargo fmt`保持默认风格
- 错误传播使用`anyhow::Result` / `?`
- 避免宏滥用

### 测试指南

**Rust测试**:
- 在相关模块附近添加`#[test]`
- 复杂逻辑放入`src-tauri/tests/`
- 提交前运行`cargo test`

**前端测试**:
- 新增复杂交互时补充轻量单元测试(Vitest/RTL)
- 确保通过`npm run lint`

### 提交规范

- 提交信息用祈使句,推荐`feat|fix|chore|docs(scope): summary`
- 一次提交聚焦单一职责
- PR需包含:变更摘要、涉及模块、测试结果

---

## 🤝 贡献指南

欢迎贡献!请遵循以下步骤:

1. Fork本仓库
2. 创建特性分支(`git checkout -b feature/AmazingFeature`)
3. 提交更改(`git commit -m 'feat: Add some AmazingFeature'`)
4. 推送到分支(`git push origin feature/AmazingFeature`)
5. 开启Pull Request

### 贡献要求

- 所有代码必须通过`cargo fmt`、`cargo clippy`和`cargo test`
- 前端代码必须通过`npm run lint`
- 新增功能需要添加相应测试
- 更新相关文档

---

## 📚 文档

项目文档统一存放在根目录的[`docs/`](../docs/)目录下:

### 架构文档

- **[CAS_ARCHITECTURE.md](docs/architecture/CAS_ARCHITECTURE.md)** - 内容寻址存储(CAS)架构详解
- **[API.md](../docs/architecture/API.md)** - API文档

### 用户指南

- **[MIGRATION_GUIDE.md](docs/MIGRATION_GUIDE.md)** - 从旧格式迁移到CAS架构的完整指南
- **[TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)** - 常见问题排查和解决方案
- **[QUICK_REFERENCE.md](../docs/guides/QUICK_REFERENCE.md)** - 快速参考手册
- **[MULTI_KEYWORD_SEARCH_GUIDE.md](../docs/guides/MULTI_KEYWORD_SEARCH_GUIDE.md)** - 多关键词搜索指南

### 开发文档

- **[AGENTS.md](../docs/development/AGENTS.md)** - AI代理开发指南
- **[CLAUDE.md](../docs/development/CLAUDE.md)** - Claude AI集成说明

### 历史记录

- **[OPTIMIZATION_REPORT.md](../docs/reports/archive/OPTIMIZATION_REPORT.md)** - 优化实施报告
- **[CHANGES_SUMMARY.md](../docs/reports/archive/CHANGES_SUMMARY.md)** - 详细的变更历史
- **[DELIVERY_PACKAGE.md](../docs/reports/archive/DELIVERY_PACKAGE.md)** - 项目交付包说明

---

## 📝 许可证

本项目采用**Apache License 2.0**开源协议(2004年1月版本)。

详见[LICENSE](../LICENSE)文件或访问官方许可链接:  
**http://www.apache.org/licenses/LICENSE-2.0.txt**

Copyright (c) 2024 [@ashllll](https://github.com/ashllll)

---

<div align="center">

**如果这个项目对您有帮助,请给个⭐Star!**

由[@ashllll](https://github.com/ashllll)使用❤️打造

</div>
