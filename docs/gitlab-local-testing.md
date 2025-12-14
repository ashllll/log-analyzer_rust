# GitLab CI/CD 本地测试指南

## 方法 1: 使用 GitLab Runner 本地执行

### 安装 GitLab Runner

#### macOS
```bash
brew install gitlab-runner
```

#### Ubuntu/Debian
```bash
# 添加 GitLab 官方仓库
curl -L https://packages.gitlab.com/install/repositories/runner/gitlab-runner/script.deb.sh | sudo bash

# 安装 GitLab Runner
sudo apt-get install gitlab-runner
```

#### Windows
```powershell
# 使用 Chocolatey
choco install gitlabrunner

# 或下载 MSI 包
# https://gitlab-runner-downloads.s3.amazonaws.com/latest/binaries/gitlab-runner-windows-amd64.exe
```

### 配置 GitLab Runner

#### 1. 注册 Runner
```bash
# 在项目根目录执行
gitlab-runner register \
  --url https://gitlab.com/ \
  --token YOUR_REGISTRATION_TOKEN \
  --description "Local Log Analyzer Runner" \
  --executor docker \
  --docker-image "rust:1.70"
```

#### 2. 配置 runner (可选)
```bash
# 编辑配置文件
sudo nano /etc/gitlab-runner/config.toml

# 示例配置
[[runners]]
  name = "Local Log Analyzer Runner"
  url = "https://gitlab.com/"
  token = "YOUR_TOKEN"
  executor = "docker"
  [runners.docker]
    image = "rust:1.70"
    volumes = ["/cache"]
    shm_size = 0
```

### 本地运行 Pipeline

#### 方法 1: 使用 CI_LOCAL 脚本
```bash
# 安装 ci-local
gem install gitlab-ci-local

# 运行完整的 pipeline
gitlab-ci-local

# 运行特定 stage
gitlab-ci-local --stage install
gitlab-ci-local --stage test

# 运行特定 job
gitlab-ci-local test:rust:unit
gitlab-ci-local test:frontend:unit

# 查看 help
gitlab-ci-local --help
```

#### 方法 2: 使用 docker-compose (推荐)
```yaml
# 创建 docker-compose.gitlab-ci.yml
version: '3.8'

services:
  gitlab-runner:
    image: gitlab/gitlab-runner:latest
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./gitlab-ci-local-cache:/cache
    environment:
      - CI_PROJECT_DIR=/workspace
    working_dir: /workspace
    command: |
      bash -c "
        gitlab-runner register \
          --non-interactive \
          --url https://gitlab.com/ \
          --token $GITLAB_TOKEN \
          --name 'Local Runner' \
          --executor docker \
          --docker-image rust:1.70 &&
        gitlab-runner run
      "
```

运行命令：
```bash
GITLAB_TOKEN=your_token docker-compose -f docker-compose.gitlab-ci.yml up
```

### 使用 pre-commit hooks 验证
```bash
# 安装 pre-commit
pip install pre-commit

# 配置 .pre-commit-config.yaml
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.4.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-added-large-files

  - repo: local
    hooks:
      - id: gitlab-ci
        name: GitLab CI lint
        entry: gitlab-ci-local --dry-run
        language: system
        files: '\.gitlab-ci\.yml$'
        pass_filenames: false

# 安装 hooks
pre-commit install

# 手动运行检查
pre-commit run --all-files
```

## 方法 2: 使用 Docker 模拟 GitLab 环境

### 创建完整的 GitLab CI 环境
```bash
# 创建 gitlab-ci-local.Dockerfile
FROM rust:1.70

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libssl-dev \
    nodejs \
    npm \
    git

# 安装 Node.js 22
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
RUN apt-get install -y nodejs

# 安装 Rust 工具
RUN rustup component add rustfmt clippy

# 安装 GitLab CI Local
RUN gem install gitlab-ci-local

# 设置工作目录
WORKDIR /workspace

# 复制项目文件
COPY . .

# 设置权限
RUN chmod +x scripts/*.sh

CMD ["bash"]
```

### 构建和运行
```bash
# 构建镜像
docker build -f gitlab-ci-local.Dockerfile -t gitlab-ci-local .

# 运行容器
docker run -it --rm \
  -v $(pwd):/workspace \
  -w /workspace \
  gitlab-ci-local bash

# 在容器内运行 CI
gitlab-ci-local --dry-run
gitlab-ci-local
```

## 方法 3: 使用 GitLab Web IDE + CI

### 配置 .gitlab-ci.yml 本地验证
```bash
# 安装 gitlab-ci-local
npm install -g @gitlab-ci-local/gitlab-ci-local

# 验证 CI 配置语法
gitlab-ci-local --dry-run

# 运行本地 pipeline
gitlab-ci-local --shell

# 交互式模式
gitlab-ci-local
```

## 方法 4: 使用 VS Code 插件

### 安装 GitLab Workflow 插件
1. 在 VS Code 中安装 `GitLab Workflow` 插件
2. 配置 GitLab 访问令牌
3. 使用插件功能：
   - 查看 pipeline 状态
   - 本地运行 pipeline
   - 查看 job 日志
   - 创建 merge request

## 性能优化技巧

### 1. 缓存优化
```bash
# 启用本地缓存
export CI_LOCAL_CACHE=1
gitlab-ci-local

# 使用 Docker volume 缓存
docker run -v cargo-cache:/root/.cargo gitlab-ci-local
```

### 2. 并行执行
```bash
# 并行运行多个 job
gitlab-ci-local --parallel 4
```

### 3. 选择性运行
```bash
# 只运行修改的 job
gitlab-ci-local --changes

# 跳过某些 stage
gitlab-ci-local --skip-stage test
```

### 4. 调试模式
```bash
# 启用调试日志
export CI_DEBUG_TRACE=true
gitlab-ci-local --verbose
```

## 环境准备脚本

### 创建 setup-gitlab-ci.sh
```bash
#!/bin/bash

echo "=== GitLab CI 本地环境设置 ==="

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "Docker 未安装，请先安装 Docker"
    exit 1
fi

# 检查 Git
if ! command -v git &> /dev/null; then
    echo "Git 未安装，请先安装 Git"
    exit 1
fi

# 安装 GitLab CI Local
if ! command -v gitlab-ci-local &> /dev/null; then
    echo "安装 GitLab CI Local..."
    gem install gitlab-ci-local
fi

# 安装 pre-commit
if ! command -v pre-commit &> /dev/null; then
    echo "安装 pre-commit..."
    pip install pre-commit
fi

# 安装 Rust
if ! command -v cargo &> /dev/null; then
    echo "安装 Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    rustup component add rustfmt clippy
fi

# 安装 Node.js
if ! command -v node &> /dev/null; then
    echo "安装 Node.js..."
    curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
    sudo apt-get install -y nodejs
fi

# 设置 pre-commit hooks
if [ -f .pre-commit-config.yaml ]; then
    pre-commit install
fi

echo "=== 环境设置完成 ==="
echo "运行测试："
echo "  gitlab-ci-local --dry-run"
echo "  gitlab-ci-local test:rust:unit"
echo "  gitlab-ci-local test:frontend:unit"
```

## 常见问题解决

### Q: Docker 权限问题
A:
```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Q: 内存不足
A:
```bash
# 增加 Docker 内存限制
# Docker Desktop -> Settings -> Resources -> Memory: 4GB+
```

### Q: 网络问题
A:
```bash
# 配置代理
export HTTP_PROXY=http://proxy:8080
export HTTPS_PROXY=http://proxy:8080
gitlab-ci-local
```

### Q: Runner 注册失败
A:
```bash
# 检查 GitLab URL 和 token
gitlab-runner list
gitlab-runner verify
```

## 验证清单

- [ ] GitLab Runner 已安装并注册
- [ ] GitLab CI Local 已安装
- [ ] Docker 环境正常
- [ ] 可以运行 `gitlab-ci-local --dry-run`
- [ ] 可以运行 `gitlab-ci-local test:rust:unit`
- [ ] 可以运行 `gitlab-ci-local test:frontend:unit`
- [ ] 缓存正常工作
- [ ] pre-commit hooks 已安装
