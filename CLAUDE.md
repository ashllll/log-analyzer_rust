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
>
> **版本**: 0.0.125
>
> **技术栈**: Tauri 2.0 + Rust 1.70+ + React 19.1.0 + TypeScript 5.8.3
>
> **最后更新**: 2026-01-15

---

## 📚 快速链接

- **[项目文档中心](docs/README.md)** - 架构文档、用户指南、开发指南
- **[Rust后端文档](log-analyzer/src-tauri/CLAUDE.md)** - 后端模块详细实现
- **[React前端文档](log-analyzer/src/CLAUDE.md)** - 前端模块详细实现
- **[全局编码原则](C:\Users\white\.claude\rules\global-principles.md)** - 必须使用成熟方案 + CI/CD验证

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
│       │   ├── search_engine/ # 搜索引擎(Tantivy,布尔查询,高亮引擎)
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

**场景**: 需要添加一个新的后端功能供前端调用

**步骤**:
1. 在 `log-analyzer/src-tauri/src/commands/` 创建新文件(如 `my_feature.rs`)
2. 使用 `#[tauri::command]` 宏装饰函数:
   ```rust
   #[tauri::command]
   pub async fn my_command(param: String) -> Result<String, String> {
       // 实现逻辑
       Ok("success".to_string())
   }
   ```
3. 在 `log-analyzer/src-tauri/src/commands/mod.rs` 中导出:
   ```rust
   pub mod my_feature;
   ```
4. 在 `log-analyzer/src-tauri/src/lib.rs` 的 `invoke_handler()` 中注册:
   ```rust
   .invoke_handler(|app| {
       // ...
       my_command(app)
   })
   ```
5. 前端类型定义(在 `log-analyzer/src/types/`):
   ```typescript
   export interface MyCommandParams {
     param: string;
   }
   ```
6. 前端调用:
   ```typescript
   import { invoke } from '@tauri-apps/api/core';
   const result = await invoke<string>('my_command', { param: 'value' });
   ```

**注意事项**:
- 遵循「前后端集成规范」: 字段名必须一致 (task_id 不是 taskId)
- 使用 `AppError` 进行错误处理
- 添加单元测试到 `commands/my_feature.rs` 末尾

### 调试Tauri IPC通信

**常见问题**: 前后端通信失败、数据格式错误

**调试步骤**:

1. **后端日志检查**:
   ```rust
   // 在命令中添加 tracing 日志
   use tracing::{info, debug, error};

   #[tauri::command]
   pub async fn my_command(data: MyData) -> Result<()> {
       debug!(?data, "Received data from frontend");
       // ...
       Ok(())
   }
   ```

2. **前端日志检查**:
   ```typescript
   import { invoke } from '@tauri-apps/api/core';

   try {
       const result = await invoke('my_command', { data: 'test' });
       console.log('Command result:', result);
   } catch (error) {
       console.error('Command failed:', error);
   }
   ```

3. **查看 Tauri DevTools**:
   - 启动应用后,按 `F12` 打开开发者工具
   - Console → 查看前端日志
   - Network → 查看 IPC 调用

4. **序列化调试**:
   ```rust
   // 检查实际序列化的 JSON
   println!("{}", serde_json::to_string_pretty(&my_data)?);
   ```

5. **常见错误**:
   - ❌ 字段名不一致: Rust `task_id` vs 前端 `taskId`
   - ❌ Option/null 处理: Rust `None` → JSON `null`,但 Zod 不接受 `null`
   - ❌ 枚举值不匹配: Rust `TaskType::Import` vs 前端 `"import"`

### 添加新的前端页面

**步骤**:
1. 创建页面组件 `log-analyzer/src/pages/MyNewPage.tsx`:
   ```typescript
   import React from 'react';
   import { useTranslation } from 'react-i18next';

   export const MyNewPage: React.FC = () => {
     const { t } = useTranslation();

     return (
       <div className="p-6">
         <h1 className="text-2xl font-bold">{t('myNewPage.title')}</h1>
         {/* 页面内容 */}
       </div>
     );
   };
   ```

2. 添加 i18n 翻译:
   ```json
   // log-analyzer/src/i18n/locales/zh.json
   {
     "myNewPage": {
       "title": "我的新页面"
     }
   }

   // log-analyzer/src/i18n/locales/en.json
   {
     "myNewPage": {
       "title": "My New Page"
     }
   }
   ```

3. 在导航中添加链接(如侧边栏):
   ```typescript
   // 在 Sidebar.tsx 中添加
   <Link to="/my-new">
     <FiSomeIcon />
     <span>{t('nav.myNewPage')}</span>
   </Link>
   ```

**最佳实践**:
- 使用函数式组件 + Hooks
- 所有文案走 i18n
- 使用 Tailwind Utility 类
- 添加 TypeScript 类型定义

### 修改搜索逻辑

1. 修改 `log-analyzer/src-tauri/src/services/pattern_matcher.rs`
2. 更新相关测试用例
3. 运行 `cargo test pattern_matcher`
4. 更新前端类型定义

---

## 测试要求

### Rust后端
- **测试覆盖率**: 80%+
- **测试用例数**: 580+个
- **核心测试模块**:
  - `storage/`: CAS存储、完整性验证 (53个测试)
  - `archive/`: 压缩包处理 (130+个测试)
  - `search_engine/`: 搜索引擎、性能优化 (50+个测试)
  - `services/`: 服务层、业务逻辑 (80+个测试)

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

# 发布前验证（推荐）
./scripts/validate-release.sh    # Linux/macOS
.\scripts\validate-release.ps1   # Windows PowerShell
```

---

## 编码规范

### 关键架构决策

#### 为什么选择 Aho-Corasick 算法?
- **问题**: 原始实现使用正则表达式逐行匹配,复杂度 O(n×m),n为行数,m为模式数
- **解决方案**: Aho-Corasick 多模式匹配算法,复杂度降至 O(n+m)
- **性能提升**: 搜索性能提升 80%+,10,000+ 次搜索/秒

#### 为什么采用 CAS 架构?
- **问题**:
  - 路径长度限制(Windows 260 字符)
  - 相同内容重复存储,浪费磁盘空间
  - 文件移动/重命名需要重建索引
- **解决方案**:
  - 内容寻址存储(SHA-256 哈希)
  - 自动去重,相同内容只存储一次
  - 文件路径与内容解耦
- **收益**:
  - 磁盘空间节省 30%+
  - SQLite + FTS5 全文搜索,查询性能提升 10 倍+

#### 为什么拆分 QueryExecutor 职责?
- **问题**: 单个 `QueryExecutor` 承担验证、计划、执行职责,代码复杂度高
- **解决方案**: 拆分为 Validator、Planner、Executor 三个独立组件
- **收益**:
  - 代码复杂度降低 60%
  - 符合单一职责原则(SRP)
  - 便于单元测试和维护

### 性能基准

#### 搜索性能
- **单关键词搜索**: 平均延迟 < 10ms
- **多关键词搜索(10个)**: 平均延迟 < 50ms
- **吞吐量**: 10,000+ 次搜索/秒
- **缓存命中率**: 85%+

#### 文件处理性能
- **ZIP 解压**: 100MB 文件 < 5 秒
- **索引构建**: 10,000 行日志 < 1 秒
- **增量更新**: 新增 1,000 行 < 100ms

#### 内存使用
- **空闲状态**: < 100MB
- **加载 1GB 日志**: < 500MB
- **搜索操作**: 额外 < 50MB

#### 对比优化前后
| 指标 | 优化前 | 优化后 | 提升 |
|------|--------|--------|------|
| 搜索延迟 | 200ms | 10ms | 95% |
| 并发处理能力 | 100 并发 | 1000+ 并发 | 10x |
| 内存占用 | 2GB | 500MB | 75% |
| 磁盘空间(去重后) | - | -30% | - |

### 核心原则(铁律)

#### ⚠️ 1. 必须使用业内成熟方案（绝对铁律）

**重要**: 本规则适用于**所有代码修改方案规划**，在进入实施阶段前必须验证方案符合此原则。

**强制要求**:
- ✅ **所有技术选型和方案设计**必须使用业内成熟的解决方案
- ✅ **Plan Mode阶段**必须验证方案的技术成熟度
- ✅ **优先选择**主流、广泛使用的库和框架
- ❌ **严格禁止**使用实验性技术、未验证的方案
- ❌ **严格禁止**"先凑合用，以后再改"的Hack式临时方案

**成熟度判断标准**:
1. **流行度**: GitHub stars > 1000（或领域内公认的权威方案）
2. **维护性**: 有官方文档，最近6个月有活跃更新
3. **社区**: 有活跃的社区支持，问题能及时得到解答
4. **稳定性**: 被知名项目使用，有生产环境验证案例
5. **兼容性**: 与项目现有技术栈兼容良好

**具体示例**:

| 需求 | ✅ 推荐方案 | ❌ 禁止方案 |
|------|-----------|----------|
| 超时控制 | AbortController（Web标准） | 手写setTimeout + flag |
| 状态管理 | Zustand / React Query | 自造useState管理 |
| 多模式匹配 | Aho-Corasick算法库 | 逐行正则表达式 |
| 异步重试 | retry / tokio-retry | 手写loop + sleep |
| 表单验证 | Zod / Yup | 手写正则校验 |
| 日期处理 | date-fns / Day.js | moment.js（已过时） |
| HTTP客户端 | fetch / axios | XMLHttpRequest原生 |
| 路由管理 | React Router / TanStack Router | 自造hash路由 |

**例外情况**（需特别说明）:
- 只有当**不存在任何成熟方案**满足需求时
- 必须在Plan Mode中**明确说明**为何现有方案都不适用
- 必须提供**充分的理由**和**风险评估**
- 经过**用户明确批准**后才可实施自定义方案

**违反此原则的后果**:
- ⚠️ 代码审查将被拒绝
- ⚠️ 增加技术债务和后期维护成本
- ⚠️ 可能引入不可预见的bug和安全问题

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

## 故障排查指南

### 问题1: 搜索无结果

**症状**: 执行搜索后结果列表为空

**排查步骤**:
1. 检查工作区状态是否为 `READY`
2. 查看后端日志,确认索引已加载:
   ```bash
   # macOS
   tail -f ~/Library/Logs/com.joeash.log-analyzer/

   # Linux
   tail -f ~/.local/share/com.joeash.log-analyzer/logs/

   # Windows
   # 查看 %APPDATA%\com.joeash.log-analyzer\logs\
   ```
3. 检查数据库:
   ```bash
   sqlite3 ~/.local/share/com.joeash.log-analyzer/workspaces/<workspace_id>/metadata.db
   SELECT COUNT(*) FROM files;
   ```
4. 验证搜索关键词是否正确(大小写、正则表达式)

**常见原因**:
- 工作区还在 `PROCESSING` 状态
- 数据库为空(导入失败)
- 搜索关键词与日志内容不匹配

### 问题2: 任务一直显示"处理中"

**症状**: 导入文件后,任务进度一直停留在 99% 或卡住

**排查步骤**:
1. 检查后端日志是否有 UNIQUE constraint 错误
2. 查看任务管理器中是否有任务事件更新
3. 检查 EventBus 幂等性检查是否误删更新

**常见原因**:
- EventBus 版本号重复,幂等性跳过更新
- UNIQUE 约束冲突,任务未正常完成
- 文件过大,处理时间过长

**解决方案**:
- 确保任务事件版本号单调递增
- 使用 `INSERT OR IGNORE` 处理CAS去重
- 检查后端日志中的错误信息

### 问题3: 前端报错 "TaskInfo undefined"

**症状**: 前端控制台报错 `Cannot read properties of undefined`

**排查步骤**:
1. 检查 Rust 结构体字段名是否与前端 TypeScript 类型一致
2. 检查是否有 `#[serde(rename)]` 导致字段名不匹配
3. 使用浏览器开发者工具查看实际接收的 JSON:
   ```javascript
   console.log(JSON.stringify(event.payload, null, 2));
   ```

**常见原因**:
- Rust 字段名 `task_id` vs 前端 `taskId` 不一致
- Zod Schema 验证失败
- 前后端类型定义不同步

### 问题4: Windows 上路径过长错误

**症状**: 导入文件时报错 "File path too long"

**解决方案**:
- 应用已使用 `dunce` crate 处理 UNC 路径
- 确保使用长路径前缀 `\\?\`
- 如果仍有问题,将文件移动到更短的路径

### 问题5: 前后端字段名不匹配

**症状**: IPC 调用失败,字段值为 undefined

**调试方法**:
1. 后端打印实际序列化的 JSON:
   ```rust
   println!("{}", serde_json::to_string_pretty(&my_data)?);
   ```
2. 前端检查接收到的数据:
   ```javascript
   console.log('Received:', JSON.stringify(data, null, 2));
   ```

**预防措施**:
- 严格遵守「前后端集成规范」
- 字段命名统一使用 `snake_case` (Rust = JSON = TypeScript)
- 避免 `#[serde(rename)]` 重命名字段

---

## 最近重大变更

### [0.0.125] - 2026-01-14

#### 📝 文档更新
- ✅ 更新版本号至 0.0.125
- ✅ 完善项目架构说明
- ✅ 补充 CI/CD 验证流程

### [0.0.123] - 2026-01-11

#### ⚠️ CI/CD 验证规则强化
- ✅ 新增全局规则：提交前必须通过 GitHub CI/CD 和发布版本编译
- ✅ 明确本地验证清单（cargo fmt, clippy, test, npm lint, build）

### [0.0.111] - 2026-01-09

#### 🎉 CAS架构性能优化
- ✅ **对象存在性缓存优化**: 使用 `DashSet` 缓存已存在对象
- ✅ **存储大小计算优化**: 使用 `walkdir` 替代递归遍历
- ✅ **SQLite性能优化**: 启用WAL模式，提升并发读写性能

#### [0.0.104] - 2026-01-09

#### 🎉 RAR处理器纯Rust重构
- ✅ **新增 rar crate 纯Rust支持**: 使用 `rar = "0.4"` 替代外部unrar
- ✅ **解决macOS ARM64构建问题**: sidecar二进制方案

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
