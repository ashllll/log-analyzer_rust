# 贡献者指南 (CONTRIBUTING)

> 本文档面向 Log Analyzer 项目的贡献者，涵盖开发工作流、可用脚本、环境配置和测试流程。

## 📋 目录

- [快速开始](#快速开始)
- [开发环境](#开发环境)
- [可用脚本](#可用脚本)
- [代码规范](#代码规范)
- [测试流程](#测试流程)
- [提交流程](#提交流程)
- [常见问题](#常见问题)

---

## 🚀 快速开始

### 1. 克隆仓库

```bash
git clone https://github.com/joeash/log-analyzer_rust.git
cd log-analyzer_rust
```

### 2. 安装依赖

```bash
cd log-analyzer
npm install
```

### 3. 启动开发服务器

```bash
npm run tauri dev
```

> **注意**: 需要先安装 [Tauri 前置依赖](https://tauri.app/v2/guides/getting-started/prerequisites)

---

## 💻 开发环境

### 环境要求

| 工具 | 最低版本 | 推荐版本 |
|------|----------|----------|
| Node.js | 22.12.0 | 22.12.0+ |
| npm | 10.0.0 | 10.0.0+ |
| Rust | 1.70+ | 1.77+ |
| Git | 2.40+ | 2.52+ |

### 前端技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| React | 19.1.0 | UI 框架 |
| TypeScript | 5.8.3 | 类型安全 |
| Zustand | 5.0.9 | 状态管理 |
| TanStack Query | 5.90.12 | 数据获取 |
| Tailwind CSS | 3.4.17 | 样式 |
| Vite | 7.0.4 | 构建工具 |

### 后端技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 1.70+ | 后端逻辑 |
| Tauri | 2.0 | 桌面框架 |
| tokio | 1.x | 异步运行时 |
| sqlx | 0.7 | 数据库 |
| Aho-Corasick | 1.0 | 多模式匹配 |

---

## 📦 可用脚本

### 前端脚本 (npm)

| 脚本 | 命令 | 描述 |
|------|------|------|
| `dev` | `npm run dev` | 启动 Vite 开发服务器 |
| `build` | `npm run build` | 构建生产版本 (TS检查 + 构建) |
| `preview` | `npm run preview` | 预览生产构建 |
| `type-check` | `npm run type-check` | TypeScript 类型检查 |
| `lint` | `npm run lint` | ESLint 检查 |
| `lint:fix` | `npm run lint:fix` | ESLint 自动修复 |
| `test` | `npm test` | 运行 Jest 测试 |
| `test:watch` | `npm run test:watch` | 监听模式运行测试 |
| `tauri` | `npm run tauri` | Tauri CLI 入口 |
| `validate:ci` | `npm run validate:ci` | 本地 CI 验证 (推荐推送前运行) |
| `prepare` | `npm run prepare` | 安装 Husky Git hooks |

### CI 验证脚本

```bash
# 运行完整本地 CI 验证
npm run validate:ci

# 验证内容:
# 1. ESLint 检查
# 2. TypeScript 类型检查
# 3. 前端测试
# 4. 前端构建
# 5. Rust 代码格式检查
# 6. Rust Clippy 检查
# (可选: Rust 测试)
```

### Rust 脚本 (cargo)

| 脚本 | 命令 | 描述 |
|------|------|------|
| 格式检查 | `cargo fmt -- --check` | 检查代码格式 |
| 代码格式化 | `cargo fmt` | 自动格式化代码 |
| Clippy | `cargo clippy -- -D warnings` | 静态分析检查 |
| 测试 | `cargo test --all-features` | 运行所有测试 |
| 构建 | `cargo build --release` | 发布构建 |

---

## 📝 代码规范

### 前后端集成规范

**关键规则**: Rust 字段名 = JSON 字段名 = TypeScript 字段名

```rust
// ✅ 正确: 直接使用 snake_case
pub struct TaskInfo {
    pub task_id: String,    // 直接用 task_id
    pub task_type: String,  // 直接用 task_type
}
```

```typescript
// ✅ TypeScript 也要保持一致
interface TaskInfo {
  task_id: string;      // 与 Rust 完全一致
  task_type: string;    // 与 Rust 完全一致
}
```

```rust
// ❌ 错误: 避免使用 serde(rename)
#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    #[serde(rename = "type")]  // 禁止！字段名不统一
    pub task_type: String,
}
```

### Rust 编码规范

- **命名**: `snake_case` (函数/变量), `CamelCase` (类型), `SCREAMING_SNAKE_CASE` (常量)
- **格式**: 使用 `cargo fmt`
- **Lint**: 使用 `cargo clippy` (零警告)
- **文档**: 公开 API 添加文档注释

### TypeScript/React 编码规范

- **命名**: `PascalCase` (组件/类型), `camelCase` (变量/函数)
- **组件**: 函数式组件 + Hooks
- **样式**: Tailwind Utility 类
- **国际化**: 文案走 `i18n` 字典

---

## 🧪 测试流程

### 前端测试

```bash
# 运行所有测试
npm test

# 监听模式
npm run test:watch

# 生成覆盖率
npm test -- --coverage
```

### Rust 测试

```bash
cd src-tauri

# 运行所有测试
cargo test --all-features

# 运行特定模块测试
cargo test pattern_matcher

# 显示详细输出
cargo test -- --nocapture
```

### 推送前验证清单

```bash
# 方式 1: 使用本地 CI 脚本 (推荐)
npm run validate:ci

# 方式 2: 手动验证
cd log-analyzer
npm run lint
npm run type-check
npm test -- --testPathIgnorePatterns=e2e
npm run build

cd src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
```

---

## 🔀 提交流程

### Git 工作流

1. **创建功能分支**
   ```bash
   git checkout -b feature/your-feature
   ```

2. **开发并提交**
   ```bash
   git add .
   git commit -m "feat: 添加新功能"
   ```

3. **推送前验证**
   ```bash
   npm run validate:ci
   ```

4. **推送到远程**
   ```bash
   git push origin feature/your-feature
   ```

5. **创建 Pull Request**

### 提交规范

| 类型 | 描述 |
|------|------|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `style` | 代码格式（不影响功能） |
| `refactor` | 重构 |
| `docs` | 文档更新 |
| `chore` | 构建/工具更新 |

### Git Hooks

项目使用 Husky 管理 Git hooks：

```bash
# pre-push hook 会自动运行 validate:ci
# 推送前自动验证，避免 CI 失败
```

---

## ❓ 常见问题

### Q: 安装依赖失败？

```bash
# 清除缓存后重试
rm -rf node_modules package-lock.json
npm install
```

### Q: TypeScript 类型错误？

```bash
# 运行类型检查定位问题
npm run type-check
```

### Q: Rust 编译失败？

```bash
# 更新依赖
cd src-tauri
cargo update

# 清理后重新构建
cargo clean
cargo build
```

### Q: 如何添加新的 Tauri 命令？

1. 在 `src-tauri/src/commands/` 创建新文件
2. 使用 `#[tauri::command]` 宏装饰函数
3. 在 `lib.rs` 中注册命令
4. 前端添加类型定义和调用

详见 [CLAUDE.md](../CLAUDE.md#添加新的tauri命令)

---

## 📚 相关资源

- [项目文档中心](../README.md)
- [CLAUDE.md](../CLAUDE.md) - AI 编程指南
- [架构文档](architecture/)
- [变更日志](../CHANGELOG.md)
