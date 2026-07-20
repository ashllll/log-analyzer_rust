# 测试与质量

## 前端检查

从 `log-analyzer/` 运行：

```bash
npm run lint
npm run type-check
npm run type-check:test
npm test -- --runInBand
npm run build
```

聚焦单个 Jest 文件：

```bash
npm test -- --runInBand src/events/__tests__/EventBus.test.ts
```

## Rust 检查

从 `log-analyzer/src-tauri/` 运行：

```bash
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo check --workspace
cargo test --workspace
cargo test --all-features
```

按 crate 聚焦：

```bash
cargo test -p la-core
cargo test -p la-storage
cargo test -p la-search
cargo test -p la-archive
```

## 边界与工作流检查

```bash
# 前后端 IPC 命名一致性
bash scripts/check_ipc_consistency.sh

# GitHub Actions invariants
node scripts/check_ci_workflows.mjs

# 版本 / release 元数据一致性
node scripts/prepare-release.mjs check

# 文档生产构建
npm run docs:build
```

## 完整本地 CI

```bash
bash scripts/validate-ci.sh
```

它覆盖 workflow 配置、Node / lockfile、格式、前端测试、Rust fmt / clippy / workspace tests、IPC 一致性与 Tauri debug smoke build，耗时明显高于聚焦检查。

## 测试策略

- Domain models 和 traits 的行为放在 workspace crates 测试。
- command 测试关注验证、错误映射与委托，不复制 use case 细节。
- React hook / event projection 测试关注订阅清理、payload 兼容和 store 更新。
- 导入安全测试覆盖嵌套归档、路径穿越与符号链接边界。
- 搜索测试覆盖查询计划、取消、过滤、分页与结果生命周期。

提交前先运行与改动最相关的聚焦测试，再运行仓库要求的完整验证组合。

