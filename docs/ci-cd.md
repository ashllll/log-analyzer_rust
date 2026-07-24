# 产品级 CI/CD 流水线

## 不变量

每个进入 `main` 的提交都按以下顺序处理：

1. `CI/CD Pipeline` 对提交执行完整验证并汇聚到稳定的 `CI Gate`。
2. `Bump Version and Tag` 只给 `workflow_run.head_sha` 分配下一个补丁版本；它不会生成未经验证的版本提交。
3. `Release` 从不可变 tag 检出同一个 SHA，在 draft release 中并行构建各平台安装包。
4. 只有六个平台全部成功且校验清单生成后，draft 才发布为 continuous prerelease。

必须始终满足：

```text
CI SHA = tag SHA = package SHA = release manifest source_sha
```

失败的矩阵构建只会留下 draft，不会向用户暴露不完整版本。重新运行同一 tag 会复用该 draft。

## 发布平台

| 平台 | Runner | Rust target | 安装包 |
|---|---|---|---|
| Linux x64 | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | AppImage、deb、rpm |
| Linux ARM64 | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | AppImage、deb、rpm |
| macOS Apple Silicon | `macos-15` | `aarch64-apple-darwin` | dmg |
| macOS Intel | `macos-15` | `x86_64-apple-darwin` | dmg |
| Windows x64 | `windows-2025` | `x86_64-pc-windows-msvc` | MSI、NSIS |
| Windows ARM64 | `windows-11-arm` | `aarch64-pc-windows-msvc` | MSI、NSIS |

每个 release 都附带：

- `SHA256SUMS`
- `release-manifest.json`
- 源码 commit SHA
- 各平台安装包

## GitHub 仓库配置

### Actions secrets

生产签名至少需要：

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_KEY_PASSWORD`

现有兼容构建还会读取 `TAURI_PRIVATE_KEY`。密钥未配置时应先验证无签名打包是否符合当前 Tauri 配置；面向最终用户发布前必须配置签名。

Windows Authenticode 和 macOS Developer ID/notarization 需要额外的平台证书。它们应存放在受保护的 GitHub Environment 中，不能提交到仓库。

### Actions variables

- `ENABLE_PAGES=true`：仓库已在 Settings → Pages 中选择 GitHub Actions 时才设置。
- 未设置时文档仍会构建验证，但 Pages 部署会安全跳过，不会制造固定红灯。

### Branch protection

将 `CI Gate` 配置为 `main` 的 required status check，并启用：

- Require branches to be up to date
- Block force pushes
- Require pull request review
- Do not allow bypassing required checks

## 版本与重跑

版本分配 workflow 使用单一并发队列且 `cancel-in-progress: false`，因此每个成功的 `main` CI 都会获得唯一 patch tag。Tag 不允许移动或覆盖。

Release workflow 可以针对仍为 draft 的同一 tag 重跑；已经发布的 release 被视为不可变，workflow 会拒绝覆盖。

## 本地验证

```bash
node scripts/check_ci_workflows.mjs
node scripts/prepare-release.mjs check
bash scripts/validate-ci.sh
```

`check_ci_workflows.mjs` 固化 CI Gate、精确 SHA 发布、ARM64 矩阵、draft 发布、校验清单和 Pages 隔离等结构性不变量。
