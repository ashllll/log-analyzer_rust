# 代码风格和约定

## Rust编码规范

### 命名约定
- **模块/函数**: `snake_case`
- **类型/Trait**: `CamelCase` (PascalCase)
- **常量**: `SCREAMING_SNAKE_CASE`
- **宏**: `snake_case!`

### 错误处理
- 使用 `thiserror` 创建统一错误类型 `AppError`
- 使用 `?` 传播错误
- 使用 `anyhow::Result<T>` 作为函数返回类型
- 提供有用的错误上下文

### 异步编程
- 使用 `async/await` 语法
- 使用 `tokio` 运行时
- 使用 `tokio::fs` 进行异步文件操作
- 使用 `tokio::spawn` 生成异步任务

### 日志记录
- 使用 `tracing` crate (不是 `log`)
- 使用结构化日志: `tracing::info!`, `tracing::error!`, `tracing::debug!`
- 添加上下文字段: `tracing::info!(user_id = %id, "User logged in")`

### 测试规范
- 目标覆盖率: 80%+
- 使用单元测试 (`#[cfg(test)]`)
- 使用集成测试 (`tests/` 目录)
- 使用属性测试 (proptest)
- 使用基准测试 (criterion)

## TypeScript/React编码规范

### 命名约定
- **组件**: `PascalCase` (如 `SearchPage.tsx`)
- **类型/接口**: `PascalCase` (如 `SearchQuery`)
- **变量/函数**: `camelCase` (如 `handleSubmit`)
- **常量**: `UPPER_SNAKE_CASE`

### 组件规范
- 使用函数式组件 + Hooks
- 使用 TypeScript 类型定义
- 使用 Tailwind Utility 类
- 文案国际化 (i18n)

### 样式规范
- 使用 Tailwind CSS
- 避免内联样式
- 使用 Utility 类组合

## 前后端集成规范（重要！）

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
使用 `INSERT OR IGNORE` + `SELECT` 模式处理UNIQUE约束：

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

## 文档规范
- 公开API添加文档注释 (`///`)
- 使用 `///` 而非 `//`
- 提供使用示例
- 说明性能特性

## 性能规范
- 使用 Aho-Corasick 算法进行多模式匹配
- 使用 Rayon 进行并行处理
- 使用 LRU 缓存减少重复计算
- 使用流式处理大文件
