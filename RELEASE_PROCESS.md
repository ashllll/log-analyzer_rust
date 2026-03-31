# 发布流程

本项目发布以 `main` 和 GitHub Releases 为中心，要求先通过本地与 CI 校验，再推进版本变更。

## 发布前检查

确保以下文件中的版本一致：

- `log-analyzer/package.json`
- `log-analyzer/src-tauri/Cargo.toml`
- `log-analyzer/src-tauri/tauri.conf.json`

本地最少验证：

```bash
cd log-analyzer
npm run lint
npm run type-check
npm test
npm run build

cd src-tauri
cargo fmt -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test -q
```

发布前还应检查：

- `CHANGELOG.md` 是否需要补充
- 工作树是否干净
- 文档是否已更新

## 版本策略

- Patch：修复与小幅优化
- Minor：向后兼容的新能力
- Major：存在破坏性变更

## 推荐发布方式

### 方式一：合并到 `main`

适用于常规迭代发布：

1. 在分支完成改动并通过校验
2. 合并到 `main`
3. 更新版本号
4. 创建并推送 tag

### 方式二：显式 tag 发布

```bash
git tag v1.2.3
git push origin v1.2.3
```

## 发布产物

Tauri 构建产物通常位于：

- macOS: `log-analyzer/src-tauri/target/release/bundle/dmg/`
- Windows: `log-analyzer/src-tauri/target/release/bundle/msi/`
- Linux: `log-analyzer/src-tauri/target/release/bundle/`

## 发布后检查

至少核对：

- 安装包是否可下载
- 应用是否可启动
- 能否导入工作区并执行一次搜索
- 文档链接是否仍有效

## 注意

- 不要在未经校验的情况下直接从脏工作树发布
- 不要把一次性报告文件混入发布文档集
- 若搜索、存储或导入链路发生变化，必须同步更新 `docs/` 中的核心文档
