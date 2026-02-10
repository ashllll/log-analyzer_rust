# 常用开发命令

## 环境要求
- **Node.js**: 22.12.0+
- **npm**: 10.0+
- **Rust**: 1.70+

## 开发命令

### 启动开发服务器
```bash
npm run tauri dev
```

### TypeScript类型检查
```bash
npm run type-check
```

### ESLint检查
```bash
npm run lint
npm run lint:fix
```

### 构建生产版本
```bash
npm run tauri build
```

## Rust测试命令

### 运行所有测试
```bash
cd log-analyzer/src-tauri
cargo test --all-features
```

### 显示测试输出
```bash
cargo test -- --nocapture
```

### 运行特定模块测试
```bash
cargo test pattern_matcher
cargo test archive
```

### 性能基准测试
```bash
cargo bench
```

### 代码格式化
```bash
cargo fmt
```

### 静态分析
```bash
cargo clippy -- -D warnings
```

## 前端测试

### 运行Jest测试
```bash
npm test
```

### 监听模式
```bash
npm run test:watch
```

### 生成覆盖率报告
```bash
npm test -- --coverage
```

## CI/CD验证（推送前必跑）

### 完整验证
```bash
npm run validate:ci
```

### 单独验证项
```bash
# ESLint
npm run lint

# TypeScript类型
npm run type-check

# 前端测试
npm test -- --testPathIgnorePatterns=e2e

# 前端构建
npm run build

# Rust格式
cargo fmt -- --check

# Rust Clippy
cargo clippy --all-features --all-targets -- -D warnings

# Rust测试
cargo test --all-features --lib --bins
```
