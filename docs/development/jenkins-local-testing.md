# Jenkins 本地测试指南

## 方法 1: 使用 Docker 运行 Jenkins

### 1. 启动 Jenkins 容器

#### 基础 Jenkins
```bash
# 拉取并运行 Jenkins
docker run -d \
  --name jenkins-local \
  -p 8080:8080 \
  -p 50000:50000 \
  -v jenkins_home:/var/jenkins_home \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd):/workspace \
  jenkins/jenkins:lts

# 获取初始密码
docker exec jenkins-local cat /var/jenkins_home/secrets/initialAdminPassword

# 访问 Jenkins (http://localhost:8080)
```

#### 完整环境 Jenkins (推荐)
```bash
# 使用 docker-compose
# 创建 docker-compose.jenkins.yml
version: '3.8'

services:
  jenkins:
    image: jenkins/jenkins:lts
    container_name: jenkins-local
    ports:
      - "8080:8080"
      - "50000:50000"
    volumes:
      - jenkins_home:/var/jenkins_home
      - /var/run/docker.sock:/var/run/docker.sock
      - $(pwd):/workspace
    environment:
      - JAVA_OPTS=-Xmx2048m
    restart: unless-stopped

volumes:
  jenkins_home:

# 启动
docker-compose -f docker-compose.jenkins.yml up -d
```

### 2. 配置 Jenkins

#### 插件安装
安装以下推荐插件：
- Pipeline
- Git
- GitHub Integration
- Docker Pipeline
- NodeJS
- Rust
- Cobertura
- JUnit
- HTML Publisher

#### 工具配置
1. **NodeJS**: 配置 NodeJS 22
2. **Docker**: 自动配置
3. **Rust**: 通过 Docker 容器使用

### 3. 创建 Pipeline Job

#### 方法 1: 通过 UI
1. 新建 Item -> Pipeline
2. Pipeline -> Definition: Pipeline script from SCM
3. SCM: Git
4. Repository URL: 本地仓库路径或 GitHub URL
5. Script Path: Jenkinsfile
6. 保存并构建

#### 方法 2: 通过 Blue Ocean
1. 安装 Blue Ocean 插件
2. 点击 "Open Blue Ocean"
3. 创建 Pipeline
4. 选择 Git 仓库
5. Jenkinsfile 自动检测

### 4. 运行 Pipeline

#### 通过 UI
1. 点击构建按钮
2. 查看构建进度
3. 查看控制台输出

#### 通过命令行
```bash
# 使用 Jenkins CLI
wget http://localhost:8080/jnlpJars/jenkins-cli.jar

# 触发构建
java -jar jenkins-cli.jar -s http://localhost:8080 build "job-name" -s

# 获取构建状态
java -jar jenkins-cli.jar -s http://localhost:8080 list-builds "job-name"
```

## 方法 2: 使用 Jenkinsfile 本地执行

### 1. 安装 Jenkins 模拟器

#### 使用 Jenkins CLI 本地运行
```bash
# 下载 Jenkins CLI
wget http://mirrors.jenkins.io/war-stable/latest/jenkins.war

# 运行 Jenkins
java -jar jenkins.war

# 在另一个终端
java -jar jenkins-cli.jar -s http://localhost:8080 help
```

#### 使用 Jenkins Pipeline Simulator
```bash
# 安装 Jenkins Pipeline Unit Testing Framework
pip install jenkins-pipeline-unit

# 创建测试脚本 (test_jenkinsfile.py)
import unittest
from jenkins_pipeline_unit.mock import mock_dsl_script

class TestJenkinsfile(unittest.TestCase):
    def test_jenkinsfile_syntax(self):
        # 测试 Jenkinsfile 语法
        result = mock_dsl_script('Jenkinsfile')
        self.assertIsNotNone(result)

if __name__ == '__main__':
    unittest.main()
```

### 2. 使用 JJB (Jenkins Job Builder)

#### 安装 JJB
```bash
pip install jenkins-job-builder

# 配置 ~/.jenkins_jobs.ini
[jenkins]
user=admin
password=admin
url=http://localhost:8080
```

#### 创建 job 配置
```yaml
# jenkins-jobs.yaml
- job:
    name: log-analyzer
    project-type: pipeline
    pipeline-scm:
      scm:
        - git:
            url: https://github.com/your-repo/log-analyzer_rust.git
            branches: '**'
      script-path: Jenkinsfile
    triggers:
      - github
    builders:
      - pipeline:
          script: ''
```

#### 部署 job
```bash
jenkins-jobs update jenkins-jobs.yaml
jenkins-jobs test jenkins-jobs.yaml  # 仅测试
```

## 方法 3: 使用 Jenkins Self-Contained Runner

### 1. 创建 Dockerfile
```dockerfile
# Dockerfile.jenkins-sim
FROM rust:1.70

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev \
    nodejs \
    npm \
    git \
    python3 \
    python3-pip

# 安装 Node.js 18
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
RUN apt-get install -y nodejs

# 安装 Python 依赖
RUN pip3 install jenkins-pipeline-unit

# 设置工作目录
WORKDIR /workspace

# 复制项目文件
COPY . .

# 权限设置
RUN chmod +x scripts/*.sh

CMD ["bash"]
```

### 2. 运行模拟环境
```bash
# 构建镜像
docker build -f Dockerfile.jenkins-sim -t jenkins-sim .

# 运行容器
docker run -it --rm \
  -v $(pwd):/workspace \
  -w /workspace \
  jenkins-sim bash

# 在容器内运行测试
python3 test_jenkinsfile.py
```

## 方法 4: 使用 Jenkinsfile Runner

### 1. 安装 Jenkinsfile Runner
```bash
# 下载 Jenkinsfile Runner
wget https://github.com/jenkinsci/jenkinsfile-runner/releases/latest

# 解压
unzip jenkinsfile-runner-*.zip

# 运行
java -jar jenkinsfile-runner.jar \
  -w /path/to/jenkins.war \
  -f . \
  -p /path/to/plugins
```

### 2. 创建测试配置
```bash
# 创建 setup.sh
#!/bin/bash

# 安装必需的工具
echo "=== 安装工具 ==="

# 安装 Rust
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    rustup component add rustfmt clippy
fi

# 安装 Node.js
if ! command -v node &> /dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
    sudo apt-get install -y nodejs
fi

# 安装依赖
echo "=== 安装项目依赖 ==="
cd log-analyzer
npm ci
cd ..

echo "=== 环境准备完成 ==="
```

### 3. 运行测试
```bash
# 设置环境
source setup.sh

# 运行 Jenkinsfile
java -jar jenkinsfile-runner.jar \
  -w jenkins.war \
  -f . \
  -p plugins.txt \
  --no-sandbox \
  --disable-http2
```

## 方法 5: 使用 Jenkins X

### 1. 安装 Jenkins X
```bash
# 安装 jx CLI
curl -L https://github.com/jenkins-x/jx/releases/latest/download/jx-linux-amd64.tar.gz | tar xzv
sudo mv jx /usr/local/bin

# 创建集群
jx create cluster kubernetes --provider minikube
```

### 2. 导入项目
```bash
# 初始化 Jenkins X
jx import

# 创建快速start
jx create spring

# 运行构建
jx build
```

## 性能优化技巧

### 1. 并行执行
```groovy
// Jenkinsfile 中的并行优化
parallel(
    linux: { /* Linux 构建 */ },
    windows: { /* Windows 构建 */ },
    macos: { /* macOS 构建 */ }
)
```

### 2. 缓存优化
```groovy
// 启用缓存
options {
    buildDiscarder(logRotator(numToKeepStr: '30'))
    timeout(time: 60, unit: 'MINUTES')
}

// Docker 缓存
agent {
    docker {
        image 'rust:1.70'
        args '-v cargo-cache:/root/.cargo'
    }
}
```

### 3. 选择性构建
```groovy
// 基于变更的构建
when {
    anyOf {
        changeset "**/*.rs"
        changeset "**/*.toml"
    }
}
```

## 环境准备脚本

### 创建 setup-jenkins.sh
```bash
#!/bin/bash

echo "=== Jenkins 本地环境设置 ==="

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "Docker 未安装，请先安装 Docker"
    exit 1
fi

# 创建 docker-compose 文件
cat > docker-compose.jenkins.yml << 'EOF'
version: '3.8'
services:
  jenkins:
    image: jenkins/jenkins:lts
    ports:
      - "8080:8080"
      - "50000:50000"
    volumes:
      - jenkins_home:/var/jenkins_home
      - /var/run/docker.sock:/var/run/docker.sock
      - $(pwd):/workspace
    environment:
      - JAVA_OPTS=-Xmx2048m
volumes:
  jenkins_home:
EOF

# 启动 Jenkins
echo "启动 Jenkins..."
docker-compose -f docker-compose.jenkins.yml up -d

# 等待 Jenkins 启动
echo "等待 Jenkins 启动..."
sleep 30

# 获取密码
PASSWORD=$(docker exec jenkins-local cat /var/jenkins_home/secrets/initialAdminPassword)
echo "Jenkins 初始密码: $PASSWORD"
echo "访问地址: http://localhost:8080"

# 安装工具
echo "安装构建工具..."

# 安装 Rust
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# 安装 Node.js
if ! command -v node &> /dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
    sudo apt-get install -y nodejs
fi

# 安装 Jenkins CLI
wget http://localhost:8080/jnlpJars/jenkins-cli.jar -O jenkins-cli.jar

echo "=== Jenkins 环境设置完成 ==="
echo "1. 访问 http://localhost:8080"
echo "2. 使用密码: $PASSWORD"
echo "3. 安装推荐插件"
echo "4. 创建 Pipeline Job，指定 Jenkinsfile"
```

## 常见问题解决

### Q: Jenkins 启动失败
A:
```bash
# 检查端口占用
sudo netstat -tulpn | grep 8080

# 清理 Docker
docker system prune -a

# 增加内存
docker run -d --memory=2g ...
```

### Q: 插件安装失败
A:
```bash
# 手动下载插件
wget https://plugins.jenkins.io/docker-plugin/latest/docker-plugin.hpi

# 通过管理界面上传
```

### Q: Docker 权限问题
A:
```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Q: 构建超时
A:
```groovy
// 增加超时时间
options {
    timeout(time: 120, unit: 'MINUTES')
}
```

## 验证清单

- [ ] Jenkins 容器成功启动
- [ ] 可以访问 http://localhost:8080
- [ ] 初始密码可获取
- [ ] 插件安装完成
- [ ] Pipeline Job 创建成功
- [ ] 可以运行完整 pipeline
- [ ] 构建产物可下载
- [ ] 测试报告可查看
- [ ] 并行构建正常工作
- [ ] 缓存机制生效
