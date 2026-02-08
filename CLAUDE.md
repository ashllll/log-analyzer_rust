# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 语言设置

**重要**: 本项目使用中文作为主要交流语言。请：
- 所有回答默认使用中文
- 代码注释使用中文
- 文档内容使用中文
- 仅在引用英文原文或技术术语时使用英文

---

> **项目**: log-analyzer_rust - 高性能桌面日志分析工具
> **版本**: 0.0.135
> **技术栈**: Tauri 2.0 + Rust 1.70+ + React 19.1.0 + TypeScript 5.8.3
> **最后更新**: 2026-02-06

---

## 快速链接

- **[项目文档中心](docs/README.md)** - 架构文档、用户指南、开发指南
- **[Rust后端文档](log-analyzer/src-tauri/CLAUDE.md)** - 后端模块详细实现
- **[React前端文档](log-analyzer/src/CLAUDE.md)** - 前端模块详细实现

---

## 核心架构

### 技术栈
- **前端**: React 19.1.0 + TypeScript 5.8.3 + Zustand 5.0.9 + Tailwind CSS 3.4.17
- **后端**: Rust 1.70+ + Tauri 2.0 + tokio 1.x + SQLite (sqlx 0.7)
- **搜索**: Aho-Corasick 算法 (性能提升 80%+)
- **存储**: 内容寻址存储(CAS) + SQLite + FTS5 全文搜索

### 目录结构
```
log-analyzer_rust/
├── log-analyzer/
│   ├── src/                   # React前端
│   │   ├── components/        # UI组件
│   │   ├── pages/            # 页面(SearchPage, WorkspacesPage等)
│   │   ├── services/         # API封装、查询构建器
│   │   ├── stores/           # Zustand状态管理
│   │   └── types/            # TypeScript类型定义
│   └── src-tauri/            # Rust后端
│       ├── src/
│       │   ├── commands/     # Tauri命令(search, import, workspace等)
│       │   ├── search_engine/ # 搜索引擎(Tantivy,布尔查询,高亮引擎)
│       │   ├── services/     # 业务逻辑(PatternMatcher, QueryExecutor等)
│       │   ├── storage/      # CAS存储系统
│       │   ├── archive/      # 压缩包处理(ZIP/RAR/GZ/TAR)
│       │   ├── task_manager/ # 异步任务Actor模型
│       │   └── models/       # 数据模型
│       └── tests/            # 集成测试
├── docs/                     # 项目文档
└── CHANGELOG.md              # 更新日志
```

---

## 常用命令

### 环境要求
- **Node.js**: 22.12.0+
- **npm**: 10.0+
- **Rust**: 1.70+
- **系统依赖**: [Tauri前置依赖](https://tauri.app/v1/guides/getting-started/prerequisites)

### 开发
```bash
# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev

# TypeScript类型检查
npm run type-check

# ESLint检查
npm run lint
npm run lint:fix

# 构建生产版本
npm run tauri build
```

### Rust测试
```bash
cd log-analyzer/src-tauri

# 运行所有测试
cargo test --all-features

# 显示测试输出
cargo test -- --nocapture

# 运行特定模块测试
cargo test pattern_matcher

# 性能基准测试
cargo bench

# 代码格式化
cargo fmt

# 静态分析
cargo clippy -- -D warnings
```

### 前端测试
```bash
# 运行Jest测试
npm test

# 监听模式
npm run test:watch

# 生成覆盖率报告
npm test -- --coverage
```

---

## 核心开发任务

### 添加新的Tauri命令

**步骤**:
1. 在 `log-analyzer/src-tauri/src/commands/` 创建新文件(如 `my_feature.rs`)
2. 使用 `#[tauri::command]` 宏装饰函数
3. 在 `log-analyzer/src-tauri/src/commands/mod.rs` 中导出
4. 在 `log-analyzer/src-tauri/src/lib.rs` 的 `invoke_handler()` 中注册
5. 前端添加类型定义，使用 `invoke()` 调用

**注意事项**:
- 遵循「前后端集成规范」: 字段名必须一致 (task_id 不是 taskId)
- 使用 `AppError` 进行错误处理
- 添加单元测试到命令文件中

### 调试Tauri IPC通信

1. **后端日志**: 使用 `tracing::{info, debug, error}` 添加日志
2. **前端日志**: 使用 `console.log/error` 检查调用结果
3. **DevTools**: 按 F12 打开开发者工具，查看 Console 和 Network
4. **序列化调试**: `println!("{}", serde_json::to_string_pretty(&my_data)?);`

**常见错误**:
- 字段名不一致: Rust `task_id` vs 前端 `taskId`
- Option/null 处理: Rust `None` → JSON `null`，但 Zod 不接受 `null`

### 添加新的前端页面

1. 创建页面组件
2. 添加 i18n 翻译 (zh.json / en.json)
3. 在导航中添加链接

**最佳实践**: 函数式组件 + Hooks，所有文案走 i18n，Tailwind Utility 类

---

## 前后端集成规范

> **关键**: Rust字段名 = JSON字段名 = TypeScript字段名

```rust
// Rust后端
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String;        // 直接用 snake_case
    pub task_type: String;
}
```

```typescript
// TypeScript前端
interface TaskInfo {
  task_id: string;              // 与Rust完全一致
  task_type: string;
}
```

### CAS存储 UNIQUE约束处理
```rust
// INSERT OR IGNORE + SELECT 模式
pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
    sqlx::query("INSERT OR IGNORE INTO files (...) VALUES (...)")
        .execute(&self.pool).await?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ?")
        .bind(&metadata.sha256_hash)
        .fetch_one(&self.pool).await?.0;

    Ok(id)
}
```

---

## 测试要求

### Rust后端
- **测试覆盖率**: 80%+
- **测试用例数**: 553个
- **核心测试模块**:
  - `storage/`: CAS存储、完整性验证
  - `archive/`: 压缩包处理 (130+测试)
  - `search_engine/`: 搜索引擎
  - `services/`: 服务层、业务逻辑

### React前端
- **测试框架**: Jest + React Testing Library
- **目标覆盖**: 80%+

---

## 代码质量检查

### 推送前强制验证 (Git Pre-Push Hook)

本项目已配置 **Git pre-push hook**，使用 Husky 管理。

```bash
# 推送前自动执行
npm run validate:ci
```

### 验证内容

| 检查项 | 命令 |
|--------|------|
| ESLint | `npm run lint` |
| TypeScript 类型 | `npm run type-check` |
| 前端测试 | `npm test -- --testPathIgnorePatterns=e2e` |
| 前端构建 | `npm run build` |
| Rust 格式 | `cargo fmt -- --check` |
| Rust Clippy | `cargo clippy --all-features --all-targets -- -D warnings` |
| Rust 测试 | `cargo test --all-features --lib --bins` |

### 跳过 Hook (不推荐)
```bash
git push --no-verify
```

---

## 编码规范

### 必须使用业内成熟方案（铁律）

| 需求 | 推荐方案 | 禁止方案 |
|------|---------|----------|
| 超时控制 | AbortController | 手写setTimeout + flag |
| 状态管理 | Zustand / React Query | 自造useState管理 |
| 多模式匹配 | Aho-Corasick算法库 | 逐行正则表达式 |
| 异步重试 | retry / tokio-retry | 手写loop + sleep |
| 表单验证 | Zod / Yup | 手写正则校验 |
| 日期处理 | date-fns / Day.js | moment.js |
| HTTP客户端 | fetch / axios | XMLHttpRequest原生 |

**例外**: 只有当不存在任何成熟方案时，经过用户明确批准才可实施自定义方案。

### Rust编码规范
- **命名**: `snake_case` (模块/函数), `CamelCase` (类型/Trait)
- **风格**: `cargo fmt`, `cargo clippy`
- **错误传播**: 使用 `?` 和 `anyhow::Result`
- **文档注释**: 公开API添加文档注释

### TypeScript/React编码规范
- **命名**: `PascalCase` (组件/类型), `camelCase` (变量/函数)
- **组件**: 函数式组件 + Hooks
- **样式**: Tailwind Utility类
- **国际化**: 文案走 `i18n` 字典

---

## 故障排查

### 搜索无结果
1. 检查工作区状态是否为 `READY`
2. 查看后端日志确认索引已加载
3. 检查数据库: `SELECT COUNT(*) FROM files;`

### 任务一直显示"处理中"
- EventBus 版本号重复，幂等性跳过更新
- UNIQUE 约束冲突，任务未正常完成
- 确保任务事件版本号单调递增

### Windows路径过长错误
应用已使用 `dunce` crate 处理 UNC 路径。

---

## 核心架构决策

### 为什么选择 Aho-Corasick?
- 正则表达式逐行匹配复杂度 O(n×m)
- Aho-Corasick 多模式匹配复杂度 O(n+m)
- 性能提升 80%+，10,000+ 次搜索/秒

### 为什么采用 CAS 架构?
- 解决 Windows 260 字符路径限制
- SHA-256 内容寻址，自动去重
- 文件路径与内容解耦，节省磁盘空间 30%+

### QueryExecutor 职责拆分
- 拆分为 Validator、Planner、Executor 三个独立组件
- 符合单一职责原则(SRP)，代码复杂度降低 60%

---

## 性能基准

### 搜索性能
- 单关键词搜索: <10ms
- 多关键词搜索(10个): <50ms
- 吞吐量: 10,000+ 次搜索/秒

### 文件处理
- ZIP 解压: 100MB < 5秒
- 索引构建: 10,000 行 < 1秒
- 增量更新: 1,000 行 < 100ms

### 内存使用
- 空闲状态: <100MB
- 加载 1GB 日志: <500MB

---
