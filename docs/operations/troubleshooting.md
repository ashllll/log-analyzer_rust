# 故障排查

## 应用无法启动

### 前端依赖或 Node 版本

```bash
node --version
npm --version
cd log-analyzer
npm ci
npm run type-check
```

Node.js 必须是 `22.12.0` 或更高版本。

### Tauri 原生依赖

Linux 最常见原因是缺少 WebKitGTK / GTK 或构建工具。对照 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)；CI 的 Linux 依赖清单位于 `.github/actions/setup-tauri-linux/action.yml`。

### Rust toolchain

```bash
cd log-analyzer/src-tauri
rustc --version
cargo check --workspace
```

期望 toolchain 为 `1.88.0`。

## 导入失败

1. 确认源路径存在且当前进程可读。
2. 查看 Tasks 页的失败信息。
3. 对归档检查格式、损坏情况和可用磁盘空间。
4. 不要反复提交相同导入；先处理原始失败任务。
5. 不可信归档应在隔离环境中排查。

## 搜索结果为空

- 确认选择了正确工作区且状态为 `READY`（开启目录监听的工作区会同时显示 `WATCHING` 标签）。
- 暂时移除时间、级别和 File Pattern，判断是哪一层过滤过严。
- 检查 OR 查询是否正确使用 `|`。
- 正则模式先用更简单的字面量验证内容确实存在。
- 目录刚写入新内容时，确认监听仍在运行，再发起新的搜索会话。

## 搜索过慢或结果过多

- 缩短 Time Range。
- 限定 File Pattern。
- 用高信号词代替宽泛正则。
- 减少 OR 分支数量。
- 分页查看结果，不要期待 UI 一次性装载全部命中。

## IPC 或 UI 状态不同步

```bash
bash scripts/check_ipc_consistency.sh
cd log-analyzer
npm test -- --runInBand src/events/__tests__/EventBus.test.ts
```

检查 Rust command / event 名、前端 API、payload schema、event projection 与 store 更新是否一起变化。

## 文档站构建失败

```bash
# 仓库根目录
npm ci
npm run docs:build
node scripts/check_ci_workflows.mjs
```

GitHub Pages 上出现资源 404 时，确认 Pages Source 选择 **GitHub Actions**，并检查生产 base 是否为 `/log-analyzer_rust/`。

更完整的构建、日志和回滚步骤见[运行手册](../RUNBOOK.md)。

