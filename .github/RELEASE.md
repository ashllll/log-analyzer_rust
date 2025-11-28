# GitHub Actions CI/CD 使用指南

## 📋 概述

本项目配置了完整的 GitHub Actions CI/CD 自动化流程，支持：

- ✅ 自动化测试和代码质量检查
- ✅ 跨平台构建（Linux、macOS、Windows）
- ✅ 自动发布 GitHub Release

## 🔄 工作流说明

### 1. CI 工作流（`.github/workflows/ci.yml`）

**触发条件**：
- 推送到 `main` 或 `develop` 分支
- 提交 Pull Request 到 `main` 或 `develop` 分支

**执行内容**：
- 在 Linux、macOS、Windows 上运行测试
- 执行 Rust 代码检查（clippy）
- 检查代码格式（rustfmt）
- 构建前端和应用

### 2. Release 工作流（`.github/workflows/release.yml`）

**触发条件**：
- 推送版本标签（如 `v1.0.0`）
- 手动触发（GitHub Actions 页面）

**执行内容**：
- 构建所有平台的发行版：
  - **Linux**: `.deb` 和 `.AppImage`
  - **macOS**: `.dmg` 和 `.app`（Intel 和 ARM64）
  - **Windows**: `.msi` 和 `.exe`
- 自动创建 GitHub Release
- 上传所有安装包

## 🚀 发布新版本

### 步骤 1：更新版本号

更新以下文件中的版本号：

```bash
# 1. package.json
cd log-analyzer
npm version patch  # 或 minor、major

# 2. Cargo.toml
# 手动编辑 src-tauri/Cargo.toml 中的 version
```

### 步骤 2：提交更改

```bash
git add .
git commit -m "chore: bump version to v1.0.0"
git push origin main
```

### 步骤 3：创建并推送标签

```bash
git tag v1.0.0
git push origin v1.0.0
```

### 步骤 4：等待构建完成

- 访问 `https://github.com/ashllll/log-analyzer_rust/actions`
- 查看 Release 工作流的进度
- 构建通常需要 10-20 分钟

### 步骤 5：检查发布

- 访问 `https://github.com/ashllll/log-analyzer_rust/releases`
- 确认所有安装包已上传

## 🔐 配置 Secrets（可选）

如需启用 Tauri 自动更新功能，需要在 GitHub 仓库设置中添加：

1. 进入仓库 Settings → Secrets and variables → Actions
2. 添加以下 secrets：
   - `TAURI_PRIVATE_KEY`: Tauri 更新签名私钥
   - `TAURI_KEY_PASSWORD`: 私钥密码

生成密钥：

```bash
cd log-analyzer
npm run tauri signer generate -- -w ~/.tauri/myapp.key
```

## 📦 支持的平台和格式

| 平台 | 安装包格式 | 说明 |
|------|-----------|------|
| Linux | `.deb` | Debian/Ubuntu 系统 |
| Linux | `.AppImage` | 通用 Linux 可执行文件 |
| macOS (Intel) | `.dmg` | Intel 芯片 Mac |
| macOS (ARM64) | `.dmg` | Apple Silicon (M1/M2) |
| Windows | `.msi` | Windows Installer |
| Windows | `.exe` | NSIS 安装程序 |

## 🐛 故障排查

### 构建失败

1. 检查 Actions 日志
2. 确保所有依赖已在 `package.json` 和 `Cargo.toml` 中声明
3. 本地测试构建：`npm run tauri build`

### 发布失败

1. 确保有推送权限
2. 检查标签格式（必须以 `v` 开头）
3. 确认 `GITHUB_TOKEN` 有 `contents: write` 权限

## 📝 手动触发发布

1. 访问 Actions 页面
2. 选择 "Release" 工作流
3. 点击 "Run workflow"
4. 选择分支并运行

## 🎯 最佳实践

1. **语义化版本**：使用 `v{major}.{minor}.{patch}` 格式
2. **测试先行**：确保 CI 通过后再发布
3. **变更日志**：在 Release 中详细说明更新内容
4. **Beta 测试**：使用 `v1.0.0-beta.1` 等预发布标签

## 📚 参考资源

- [Tauri 官方文档](https://tauri.app/v1/guides/building/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [语义化版本规范](https://semver.org/lang/zh-CN/)
