# GitHub Actions 本地测试指南

## 方法 1: 使用 act（推荐）

### 安装 act
```bash
# macOS
brew install act

# Ubuntu/Debian
curl -s https://api.github.com/repos/nektos/act/releases/latest | grep "browser_download_url.*linux.*tar.gz" | cut -d '"' -f 4 | xargs -n 1 curl -L | tar -xz
sudo mv act /usr/local/bin/act

# Windows (使用 Chocolatey)
choco install act
```

### 配置 .actrc 文件
```bash
# 项目根目录创建 .actrc
-P ubuntu-latest=catthehacker/ubuntu:act-latest
-P ubuntu-22.04=catthehacker/ubuntu:act-latest
-P windows-latest=catthehacker/ubuntu:act-latest
-P macos-latest=catthehacker/ubuntu:act-latest
```

### 运行 CI 测试
```bash
# 运行完整的 CI 流程
act -P ubuntu-latest=catthehacker/ubuntu:act-latest

# 只运行特定 job
act -j test-rust
act -j test-frontend
act -j integration-test

# 模拟 PR 触发
act pull_request

# 模拟 push 到 main 分支
act push --branch main
```

### 本地环境准备
```bash
# 安装必需的系统依赖（Ubuntu/Debian）
sudo apt-get update
sudo apt-get install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.0-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libssl-dev

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 安装 Node.js 22.12+
# 使用 nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 22.12
nvm use 22
nvm alias default 22
```

## 方法 2: 使用 GitHub CLI

### 安装 GitHub CLI
```bash
# macOS
brew install gh

# Ubuntu/Debian)
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
sudo apt update
sudo apt install gh
```

### 本地工作流测试
```bash
# 验证工作流语法
gh workflow verify .github/workflows/ci.yml

# 查看工作流状态
gh run list

# 手动触发工作流
gh workflow run ci.yml
```

## 方法 3: 使用 Docker 容器模拟

### 创建完整的 CI 环境
```bash
# 运行完整的 Rust + Node.js CI 环境
docker run --rm -it \
  -v "$(pwd)":/workspace \
  -w /workspace \
  rust:1.70 \
  bash -c "
    apt-get update && apt-get install -y \
      libgtk-3-dev \
      libwebkit2gtk-4.0-dev \
      curl \
      build-essential

    curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
    apt-get install -y nodejs

    # 运行测试
    cd log-analyzer/src-tauri
    cargo test --all-features
  "
```

## 性能优化技巧

### 1. 并行运行
```bash
# 并行运行多个 job
act -j test-rust &
act -j test-frontend &
wait
```

### 2. 缓存优化
```bash
# 启用缓存
export ACT_CACHE=1
act
```

### 3. 自定义 runners
```bash
# 使用轻量级 runner
act -P ubuntu-latest=nektos/act-environments-ubuntu:18.04
```

## 常见问题解决

### Q: 内存不足
A: 使用轻量级镜像或增加 Docker 内存限制
```bash
docker system prune -a  # 清理未使用的镜像
```

### Q: 权限问题
A: 确保 Docker 守护进程有足够权限
```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Q: 网络问题
A: 配置代理或使用离线模式
```bash
export HTTP_PROXY=http://proxy:8080
export HTTPS_PROXY=http://proxy:8080
act
```

## 验证清单

- [ ] act 安装并正常工作
- [ ] 系统依赖已安装
- [ ] Rust 工具链已配置
- [ ] Node.js 环境已设置
- [ ] 可以成功运行 `cargo test`
- [ ] 可以成功运行 `npm test`
- [ ] GitHub Actions 工作流可以本地执行
