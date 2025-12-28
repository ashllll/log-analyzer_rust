# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **项目**: log-analyzer_rust - 高性能桌面日志分析工具
>
> **版本**: 0.0.72
>
> **技术栈**: Tauri 2.0 + Rust + React 19 + TypeScript
>
> **最后更新**: 2025-12-28

---

## 快速链接

- **[完整项目文档](CLAUDE.md)** - 包含架构、编码规范、AI使用指引的完整文档
- **[Rust后端文档](log-analyzer/src-tauri/CLAUDE.md)** - 后端模块详细实现
- **[React前端文档](log-analyzer/src/CLAUDE.md)** - 前端模块详细实现
- **[项目文档中心](docs/README.md)** - 架构文档、用户指南、开发指南
- **[改进建议](CLAUDE_IMPROVEMENTS.md)** - 对现有CLAUDE.md的改进建议

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
├── log-analyzer/              # 主项目
│   ├── src/                   # React前端
│   │   ├── components/        # UI组件
│   │   ├── pages/            # 页面(SearchPage, WorkspacesPage等)
│   │   ├── services/         # API封装、查询构建器
│   │   ├── stores/           # Zustand状态管理
│   │   └── types/            # TypeScript类型定义
│   └── src-tauri/            # Rust后端
│       ├── src/
│       │   ├── commands/     # Tauri命令(search, import, workspace等)
│       │   ├── services/     # 业务逻辑(PatternMatcher, QueryExecutor等)
│       │   ├── storage/      # CAS存储系统
│       │   ├── archive/      # 压缩包处理(ZIP/RAR/GZ/TAR)
│       │   └── models/       # 数据模型
│       └── tests/            # 集成测试
├── docs/                     # 项目文档
└── CHANGELOG.md              # 更新日志
```

---

## 常用命令

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

1. 在 `src-tauri/src/commands/` 创建新文件
2. 使用 `#[tauri::command]` 宏装饰函数:
```rust
#[tauri::command]
pub async fn my_command(param: String) -> Result<String, String> {
    // 实现逻辑
    Ok("success".to_string())
}
```
3. 在 `src-tauri/src/commands/mod.rs` 中导出
4. 在 `src-tauri/src/lib.rs` 的 `invoke_handler()` 中注册
5. 前端调用: `invoke<string>('my_command', { param: 'value' })`

**重要**: 遵循[前后端集成规范](#前后端集成规范)中的字段命名规则!

### 添加新的前端页面

1. 创建 `src/pages/MyNewPage.tsx`
2. 使用函数式组件 + Hooks
3. 文案走 `i18n` 字典,不硬编码字符串
4. 使用 Tailwind Utility 类
5. 在导航中添加链接

### 修改搜索逻辑

1. 修改 `src-tauri/src/services/pattern_matcher.rs`
2. 更新相关测试用例
3. 运行 `cargo test pattern_matcher`
4. 更新前端类型定义

---

## 测试要求

### Rust后端
- **测试覆盖率**: 80%+
- **测试用例数**: 87个
- **核心测试模块**:
  - `pattern_matcher.rs`: 9个测试
  - `query_validator.rs`: 6个测试
  - `query_planner.rs`: 7个测试
  - `file_watcher_async.rs`: 5个测试
  - `error.rs`: 17个测试

### React前端
- **测试框架**: Jest + React Testing Library
- **当前覆盖**: SearchQueryBuilder 完整覆盖(40+测试用例)
- **目标覆盖**: 80%+

### 代码质量检查
提交前必须通过:
```bash
# Rust
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all-features

# 前端
npm run lint
npm run type-check
npm run build
```

---

## 编码规范

### 核心原则(铁律)

#### ⚠️ 1. 必须使用业内成熟方案
- ✅ 用AbortController而不是手写timeout
- ✅ 用Zustand/React Query而不是自造状态管理
- ✅ 用Aho-Corasick而不是逐行正则匹配
- ❌ 禁止使用实验性技术
- ❌ 禁止"Hack式临时方案"

**标准**: GitHub stars > 1000, 有官方文档, 最近6个月有更新

### Rust编码规范
- **命名**: `snake_case` (模块/函数), `CamelCase` (类型/Trait), `SCREAMING_SNAKE_CASE` (常量)
- **风格**: `cargo fmt`, `cargo clippy`
- **错误传播**: 使用 `?` 和 `anyhow::Result`
- **文档注释**: 公开API添加文档注释

### TypeScript/React编码规范
- **命名**: `PascalCase` (组件/类型), `camelCase` (变量/函数)
- **组件**: 函数式组件 + Hooks
- **样式**: Tailwind Utility类
- **国际化**: 文案走 `i18n` 字典

---

## 前后端集成规范

> **关键**: Rust字段名 = JSON字段名 = TypeScript字段名

### ✅ 正确做法
```rust
// Rust后端
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String,        // 直接用 task_id
    pub task_type: String,      // 直接用 task_type
}
```

```typescript
// TypeScript前端
interface TaskInfo {
  task_id: string;              // 与Rust完全一致
  task_type: string;            // 与Rust完全一致
}
```

### ❌ 错误做法
```rust
// 不要用 serde(rename) 处理字段名不一致!
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    #[serde(rename = "type")]    // ❌ 避免
    pub task_type: String,
}
```

### CAS存储 UNIQUE约束处理
```rust
// ✅ 正确: INSERT OR IGNORE + SELECT
pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
    // 跳过重复(CAS去重)
    sqlx::query("INSERT OR IGNORE INTO files (...) VALUES (...)")
        .execute(&self.pool).await?;

    // 查询ID(新插入或已存在)
    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ?")
        .bind(&metadata.sha256_hash)
        .fetch_one(&self.pool).await?.0;

    Ok(id)
}
```

---

## 常见问题排查

### 问题1: 搜索无结果
**检查**:
1. 工作区状态是否为 `READY`
2. 后端日志是否显示索引已加载
3. 数据库: `SELECT COUNT(*) FROM files;`

### 问题2: 任务卡在"处理中"
**原因**: EventBus幂等性误删更新 或 UNIQUE约束冲突
**解决**:
- 检查后端日志是否有UNIQUE constraint错误
- 使用 `INSERT OR IGNORE` 处理CAS去重
- 确保任务事件版本号单调递增

### 问题3: 前端报错 "undefined"
**原因**: Rust字段名与TypeScript不一致
**调试**:
```javascript
console.log(JSON.stringify(event.payload, null, 2));
```
**检查**: 字段名是否完全匹配(包括task_id vs taskId)

---

## 最近重大变更

### [0.1.0] - 2025-12-27
- ✅ 完成CAS架构迁移
- ✅ 移除legacy `path_map`系统
- ✅ 统一MetadataStore
- ✅ 修复EventBus幂等性导致任务卡在PROCESSING
- ✅ 修复CAS存储系统UNIQUE约束冲突

### 详见
- [完整变更日志](CHANGELOG.md)
- [项目文档中心](docs/README.md)
- [Rust后端文档](log-analyzer/src-tauri/CLAUDE.md)
- [React前端文档](log-analyzer/src/CLAUDE.md)

---

*详细的项目愿景、模块索引、AI使用指引等内容请查看[完整CLAUDE.md](CLAUDE.md)*
