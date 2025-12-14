# Node.js 版本升级指南

## 升级概述

项目已升级至 Node.js 22.12+，以获得更好的兼容性、性能和安全性。

## 当前版本要求

- **Node.js**: >= 22.12.0
- **npm**: >= 10.0.0

## 升级步骤

### 方法 1：使用 NVM（推荐）

#### 1. 安装 NVM（如果尚未安装）

**macOS/Linux**:
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
```

**重启终端后验证**:
```bash
nvm --version
```

#### 2. 安装 Node.js 22.12+

```bash
nvm install 22.12
nvm use 22
nvm alias default 22
```

#### 3. 验证安装

```bash
node --version  # 应显示 v22.21.1 或更高
npm --version   # 应显示 10.x.x 或更高
```

### 方法 2：使用官方安装包

1. 访问 [Node.js 官网](https://nodejs.org/)
2. 下载 Node.js 22.12+ 版本
3. 运行安装程序
4. 重启终端并验证版本

### 方法 3：使用包管理器

**macOS (Homebrew)**:
```bash
brew install node@22
brew link node@22 --force
```

**Windows (Chocolatey)**:
```powershell
choco install nodejs --version=22.12.0
```

## 项目配置更新

### .nvmrc

项目根目录包含 `.nvmrc` 文件，指定使用 Node.js 22：

```bash
cat .nvmrc  # 输出: 22
```

### package.json

`package.json` 已添加 `engines` 字段：

```json
{
  "engines": {
    "node": ">=22.12.0",
    "npm": ">=10.0.0"
  }
}
```

### 使用 .nvmrc

在项目目录中：

```bash
nvm use  # 自动使用 .nvmrc 中指定的版本
```

## 验证项目

升级后，验证项目：

```bash
# 安装依赖
npm ci

# 运行代码检查
npm run lint
npm run type-check

# 构建项目
npm run build
```

## CI/CD 更新

以下文件已更新以支持 Node.js 22：

- `.gitlab-ci.yml` - NODE_VERSION: "22"
- `Jenkinsfile` - NODE_VERSION: '22'
- `.github/workflows/local-testing.md` - 安装说明
- `docs/gitlab-local-testing.md` - NodeSource 仓库
- `docs/jenkins-local-testing.md` - 工具配置

## 兼容性说明

### Vite 7.x

Node.js 22.12+ 完全支持 Vite 7.x，解决了以下问题：

- ✅ Node.js 版本警告
- ✅ 更好的 ES 模块支持
- ✅ 改进的性能和内存使用

### Tauri 2.x

- ✅ 完全兼容 Node.js 22
- ✅ 更好的 Windows/macOS/Linux 支持

### 依赖包兼容性

所有项目依赖已验证与 Node.js 22 兼容：

- React 19.x
- TypeScript 5.8+
- Vite 7.x
- Tauri 2.x

## 故障排除

### 问题：npm ci 失败

**解决方案**:
```bash
rm -rf node_modules package-lock.json
npm cache clean --force
npm ci
```

### 问题：版本不匹配警告

**解决方案**:
确保使用正确的 Node.js 版本：
```bash
nvm use 22
```

### 问题：权限错误（macOS/Linux）

**解决方案**:
```bash
sudo chown -R $(whoami) ~/.npm
```

## 回滚

如需回滚到 Node.js 18：

```bash
nvm install 18
nvm use 18
nvm alias default 18
```

## 更多信息

- [NVM 官方文档](https://github.com/nvm-sh/nvm)
- [Node.js 官方文档](https://nodejs.org/docs/)
- [Vite 兼容性指南](https://vitejs.dev/guide/)

---

**更新时间**: 2025-12-14
**目标版本**: Node.js 22.12+
