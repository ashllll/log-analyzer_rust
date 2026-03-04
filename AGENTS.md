# AGENTS.md - 开发代理指南

本文件为编程代理（如AI助手）提供在此仓库中工作的详细指南。

## 常用命令

### 环境要求
- **Node.js**: 22.12.0+ (通过 `engines` 字段强制)
- **npm**: 10.0+
- **Rust**: 1.70+ (MSVC 工具链 on Windows)
- **系统依赖**: Tauri前置依赖 (GTK3/GTK4 开发库, Xcode Command Line Tools, 或 Microsoft C++ Build Tools)

### 开发命令
```bash
# 进入前端目录
cd log-analyzer

# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev

# TypeScript类型检查
npm run type-check

# ESLint检查
npm run lint
npm run lint:fix

# 前端测试
npm test
npm run test:watch

# 构建生产版本
npm run tauri build

# CI完整验证
npm run validate:ci
```

### Rust后端测试
```bash
cd log-analyzer/src-tauri

# 运行所有测试
cargo test --all-features

# 显示测试输出
cargo test -- --nocapture

# 运行特定模块测试
cargo test pattern_matcher

# 运行集成测试
cargo test --test '*'

# 性能基准测试
cargo bench

# 代码格式化
cargo fmt

# 代码格式检查
cargo fmt -- --check

# 静态分析
cargo clippy -- -D warnings

# CI完整检查
cargo clippy --all-features --all-targets -- -D warnings
```

### 单个测试运行
```bash
# 运行单个Rust测试
cargo test test_function_name -- --exact

# 运行单个模块的所有测试
cargo test module_name -- --exact

# 运行前端单个测试文件
npm test -- --testPathPattern=filename.test.ts

# 运行带特定模式的测试
npm test -- --testNamePattern="test description"
```

## 代码风格指南

### Rust编码规范

#### 命名约定
- **模块/函数**: `snake_case`
- **类型/Trait**: `CamelCase` (PascalCase)
- **常量**: `SCREAMING_SNAKE_CASE`
- **宏**: `snake_case!`

#### 错误处理
- 使用 `thiserror` 创建统一错误类型 `AppError`
- 使用 `?` 传播错误，避免 `unwrap/expect`
- 提供有用的错误上下文
- 使用 `eyre/miette` 进行结构化错误报告

#### 异步编程
- 使用 `async/await` 语法
- 使用 `tokio` 运行时
- 使用 `tokio::fs` 进行异步文件操作
- 使用 `parking_lot` 高性能锁，`DashMap` 并发哈希映射

#### 导入顺序
1. 标准库 (`std::`, `core::`)
2. 外部crate (`tokio::`, `serde::`, `tauri::`)
3. 内部模块 (`crate::`)
4. 本地模块 (`super::`, `crate::models::`)

### TypeScript/React编码规范

#### 命名约定
- **组件**: `PascalCase` (如 `SearchPage.tsx`)
- **类型/接口**: `PascalCase` (如 `SearchQuery`)
- **变量/函数**: `camelCase` (如 `handleSubmit`)
- **常量**: `UPPER_SNAKE_CASE`

#### 组件规范
- 使用函数式组件 + Hooks
- 使用 TypeScript 严格模式
- 使用 Tailwind Utility 类 + `clsx` 条件类名
- 文案国际化 (i18next)

#### 导入顺序
1. React相关 (`import React`, `import { useState }`)
2. 第三方库 (`import { invoke } from '@tauri-apps/api'`)
3. 内部模块 (`import { TaskInfo } from '../types'`)
4. 相对路径 (`import { SearchBar } from './components'`)

## 前后端集成规范（关键！）

### 字段命名一致性
**绝对铁律**: Rust字段名 = JSON字段名 = TypeScript字段名

```rust
// Rust后端 - 使用snake_case
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: String,        // 不是taskId
    pub task_type: String,
    pub created_at: DateTime<Utc>,
}
```

```typescript
// TypeScript前端 - 与Rust完全一致
interface TaskInfo {
  task_id: string;              // 与Rust一致
  task_type: string;
  created_at: string;
}
```

### CAS存储 UNIQUE约束处理
使用 `INSERT OR IGNORE` + `SELECT` 模式：

```rust
pub async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64> {
    sqlx::query("INSERT OR IGNORE INTO files (...) VALUES (...)")
        .execute(&self.pool).await?;

    let id = sqlx::query_as::<_, (i64,)>("SELECT id FROM files WHERE sha256_hash = ?")
        .bind(&metadata.sha256_hash)
        .fetch_one(&self.pool).await?.0;

    Ok(id)
}
```

## 错误处理规范

### Rust错误处理
- 生产代码100%消除panic
- 使用 `thiserror` 定义错误类型
- 使用 `?` 传播错误，提供上下文
- 避免使用 `String` 作为错误类型

### TypeScript错误处理
- 使用 `zod` 进行数据验证
- 使用 `react-error-boundary` 捕获组件错误
- 提供用户友好的错误信息

## 性能要求

### 必须使用业内成熟方案
| 需求 | 推荐方案 | 禁止方案 |
|------|---------|----------|
| 超时控制 | AbortController | 手写setTimeout + flag |
| 状态管理 | Zustand / React Query | 自造useState管理 |
| 多模式匹配 | Aho-Corasick算法库 | 逐行正则表达式 |
| 异步重试 | tokio-retry | 手写loop + sleep |
| 表单验证 | Zod / Validator derive | 手写正则校验 |
| 全文搜索 | Tantivy | 手写倒排索引 |
| 错误处理 | thiserror / eyre / miette | String / Box<dyn Error> |

## 测试要求

### Rust后端
- **测试覆盖率**: 80%+
- **测试框架**: rstest (增强单元测试) + proptest (属性测试) + criterion (基准测试)
- **核心测试模块**: storage/, archive/, search_engine/, services/, task_manager/

### React前端
- **测试框架**: Jest + React Testing Library
- **目标覆盖**: 80%+

## Git推送前验证

使用Git pre-push hook或手动运行：
```bash
npm run validate:ci
# 或
bash ../scripts/validate-ci.sh
```

验证内容包括：ESLint、TypeScript类型、前端测试、前端构建、Rust格式、Rust Clippy、Rust测试。

## 项目特定规则

1. **语言设置**: 本项目使用中文，所有回答、注释、文档默认使用中文
2. **离线场景**: 只在完全离线环境使用，确保所有功能可离线运行
3. **Tauri命令**: 遵循「前后端集成规范」，字段名必须一致
4. **文件处理**: 使用CAS架构解决Windows路径限制，自动去重
5. **搜索优化**: 使用Aho-Corasick算法和Tantivy搜索引擎
6. **异常处理**: 熔断自愈，Circuit Breaker + Poisoned Lock Recovery

## 重要提醒

- **不允许简单修复**: 必须使用业内成熟方案
- **不确定就问**: 任何不确定的地方需向人类询问确认
- **CI检查**: 上传代码前必须本地检查CI，通过后才可上传
- **全面分析**: 修改前全面分析代码结构，不允许简单方案
- **任务拆解**: 将任务拆解为最小可执行步骤，逐个执行