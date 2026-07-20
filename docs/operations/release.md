# 发布流程

发布要求前端、Rust crate 与 Tauri 配置保持同一版本。权威详细步骤仍以仓库根目录的 [`RELEASE_PROCESS.md`](https://github.com/ashllll/log-analyzer_rust/blob/main/RELEASE_PROCESS.md) 为准。

## 版本一致性

以下文件必须使用相同版本：

- `log-analyzer/package.json`
- `log-analyzer/src-tauri/Cargo.toml`
- `log-analyzer/src-tauri/tauri.conf.json`

`scripts/prepare-release.mjs` 负责检查或应用版本变化，并刷新 workspace lockfile。

## 发布前检查

```bash
# 仓库根目录
bash scripts/validate-ci.sh
bash scripts/check_ipc_consistency.sh
node scripts/prepare-release.mjs check
```

还应确认：

- `CHANGELOG.md` 已记录面向用户的变化。
- release workflow 的 action SHA 和目标矩阵有效。
- 三处版本号与 `Cargo.lock` 一致。
- 本地 Tauri debug smoke build 成功。
- 文档中新增 / 改变的用户行为已经同步。

## 自动化路径

`bump-and-tag.yml` 负责版本提升 / tag 相关自动化，并触发后续 release 流程；`release.yml` 在目标平台构建桌面产物。不要从本地绕开工作流直接覆盖已发布 tag。

## 发布后

1. 检查 GitHub Release 中各平台产物。
2. 验证版本号和 changelog。
3. 观察 release workflow 与 Pages workflow 状态。
4. 如需回滚，遵循[运行手册](../RUNBOOK.md)中的回滚建议，保留失败构建日志。

