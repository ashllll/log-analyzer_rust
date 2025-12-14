// Jenkins Pipeline for Log Analyzer (Rust + Tauri + React)
// 支持多平台构建和自动化部署

pipeline {
    agent any

    // 环境变量
    environment {
        CARGO_TERM_COLOR = 'always'
        RUST_BACKTRACE = '1'
        NODE_VERSION = '18'
        CARGO_HOME = "${WORKSPACE}/.cargo"
        NPM_CONFIG_CACHE = "${WORKSPACE}/.npm"
        // 构建参数
        BUILD_TARGETS = 'x86_64-unknown-linux-gnu,x86_64-pc-windows-msvc,x86_64-apple-darwin'
    }

    // 参数化构建
    parameters {
        string(name: 'RUST_VERSION', defaultValue: '1.70', description: 'Rust toolchain version')
        string(name: 'NODE_VERSION', defaultValue: '18', description: 'Node.js version')
        booleanParam(name: 'RUN_TESTS', defaultValue: true, description: 'Run unit tests')
        booleanParam(name: 'RUN_INTEGRATION_TESTS', defaultValue: true, description: 'Run integration tests')
        booleanParam(name: 'RUN_SECURITY_SCAN', defaultValue: true, description: 'Run security scans')
        booleanParam(name: 'BUILD_RELEASE', defaultValue: false, description: 'Build release artifacts')
        choice(name: 'BUILD_TARGET', choices: ['linux', 'windows', 'macos', 'all'], description: 'Build target platform')
        string(name: 'GIT_BRANCH', defaultValue: 'main', description: 'Git branch to build')
    }

    // 工具配置
    tools {
        nodejs 'NodeJS-18'
        rust 'Rust-1.70'
    }

    // 构建选项
    options {
        // 保留构建历史
        buildDiscarder(logRotator(numToKeepStr: '30'))
        // 跳过默认检出
        skipDefaultCheckout()
        // 超时设置
        timeout(time: 60, unit: 'MINUTES')
        // 并行构建
        parallelsAlwaysFailFast()
    }

    // 触发器
    triggers {
        // GitHub hook 触发
        githubPush()
        // 定时构建
        pollSCM('H */6 * * *')
    }

    // 阶段定义
    stages {
        // 阶段 1: 检出代码
        stage('Checkout') {
            steps {
                script {
                    echo "=== 开始检出代码 ==="
                    checkout scm
                    echo "当前分支: ${env.BRANCH_NAME}"
                    echo "提交哈希: ${env.GIT_COMMIT}"
                }
            }
        }

        // 阶段 2: 环境准备
        stage('Setup Environment') {
            parallel {
                // 子阶段 2.1: Rust 环境
                stage('Setup Rust') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    steps {
                        sh '''
                            rustup --version
                            cargo --version
                            rustc --version
                            rustup component add rustfmt clippy
                            echo "Rust 环境准备完成"
                        '''
                    }
                }

                // 子阶段 2.2: Node.js 环境
                stage('Setup Node.js') {
                    agent {
                        docker {
                            image 'node:18'
                        }
                    }
                    steps {
                        sh '''
                            node --version
                            npm --version
                            npm ci
                            echo "Node.js 环境准备完成"
                        '''
                    }
                }
            }
        }

        // 阶段 3: 代码质量检查
        stage('Code Quality') {
            parallel {
                // 子阶段 3.1: Rust 格式检查
                stage('Rust Format') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        expression { return params.RUN_TESTS }
                    }
                    steps {
                        sh '''
                            cd log-analyzer/src-tauri
                            cargo fmt -- --check
                            echo "Rust 格式检查通过"
                        '''
                    }
                }

                // 子阶段 3.2: Rust Clippy 检查
                stage('Rust Clippy') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        expression { return params.RUN_TESTS }
                    }
                    steps {
                        sh '''
                            cd log-analyzer/src-tauri
                            cargo clippy -- -D warnings
                            echo "Rust Clippy 检查通过"
                        '''
                    }
                }

                // 子阶段 3.3: 前端 Lint
                stage('Frontend Lint') {
                    agent {
                        docker {
                            image 'node:18'
                        }
                    }
                    when {
                        expression { return params.RUN_TESTS }
                    }
                    steps {
                        sh '''
                            cd log-analyzer
                            npm run lint
                            echo "前端代码检查通过"
                        '''
                    }
                }
            }
        }

        // 阶段 4: 测试
        stage('Tests') {
            parallel {
                // 子阶段 4.1: Rust 单元测试
                stage('Rust Unit Tests') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        expression { return params.RUN_TESTS }
                    }
                    steps {
                        sh '''
                            cd log-analyzer/src-tauri
                            cargo test --all-features --verbose
                        '''
                        publishTestResults testResultsPattern: 'target/test-results/**/test-*.xml'
                        publishCoverage adapters: [coberturaAdapter('target/coverage/cobertura.xml')], sourceFileResolver: sourceFiles('STORE_LAST_BUILD')
                    }
                }

                // 子阶段 4.2: 前端测试
                stage('Frontend Tests') {
                    agent {
                        docker {
                            image 'node:18'
                        }
                    }
                    when {
                        expression { return params.RUN_TESTS }
                    }
                    steps {
                        sh '''
                            cd log-analyzer
                            npm test -- --coverage --watchAll=false
                        '''
                        publishTestResults testResultsPattern: 'test-results.xml'
                        publishCoverage adapters: [coberturaAdapter('coverage/cobertura-coverage.xml')]
                    }
                }

                // 子阶段 4.3: 基准测试
                stage('Benchmarks') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        branch 'main'
                    }
                    steps {
                        sh '''
                            cd log-analyzer/src-tauri
                            cargo test --bench -- --nocapture > benchmark-results.txt
                        '''
                        archiveArtifacts artifacts: 'benchmark-results.txt', allowEmptyArchive: true
                        publishHTML([
                            allowMissing: false,
                            alwaysLinkToLastBuild: true,
                            keepAll: true,
                            reportDir: '.',
                            reportFiles: 'benchmark-results.txt',
                            reportName: 'Benchmark Results'
                        ])
                    }
                }
            }
        }

        // 阶段 5: 集成测试
        stage('Integration Tests') {
            agent {
                docker {
                    image 'rust:1.70'
                    args '-v /var/run/docker.sock:/var/run/docker.sock'
                }
            }
            when {
                expression { return params.RUN_INTEGRATION_TESTS }
            }
            steps {
                sh '''
                    cd log-analyzer/src-tauri
                    cargo test --test '*' --verbose
                '''
                publishTestResults testResultsPattern: 'target/test-results/**/integration-*.xml'
            }
        }

        // 阶段 6: 安全扫描
        stage('Security Scans') {
            parallel {
                // 子阶段 6.1: Cargo Audit
                stage('Cargo Audit') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        expression { return params.RUN_SECURITY_SCAN }
                    }
                    steps {
                        sh '''
                            cargo install cargo-audit
                            cd log-analyzer/src-tauri
                            cargo audit --json > audit-report.json || true
                        '''
                        archiveArtifacts artifacts: 'audit-report.json', allowEmptyArchive: true
                    }
                }

                // 子阶段 6.2: 依赖检查
                stage('Dependency Check') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        expression { return params.RUN_SECURITY_SCAN }
                    }
                    steps {
                        sh '''
                            cargo install cargo-outdated cargo-udeps
                            cd log-analyzer/src-tauri
                            cargo outdated --format=json > outdated-report.json || true
                            cargo +nightly udeps --all-targets --output-format json > udeps-report.json || true
                        '''
                        archiveArtifacts artifacts: '*-report.json', allowEmptyArchive: true
                    }
                }

                // 子阶段 6.3: NPM Audit
                stage('NPM Audit') {
                    agent {
                        docker {
                            image 'node:18'
                        }
                    }
                    when {
                        expression { return params.RUN_SECURITY_SCAN }
                    }
                    steps {
                        sh '''
                            cd log-analyzer
                            npm audit --audit-level=high
                        '''
                    }
                }
            }
        }

        // 阶段 7: 构建
        stage('Build') {
            parallel {
                // 子阶段 7.1: 构建 Linux
                stage('Build Linux') {
                    agent {
                        docker {
                            image 'rust:1.70'
                            args '-v /var/run/docker.sock:/var/run/docker.sock'
                        }
                    }
                    when {
                        anyOf {
                            expression { return params.BUILD_TARGET == 'linux' || params.BUILD_TARGET == 'all' }
                            expression { return params.BUILD_RELEASE }
                        }
                    }
                    steps {
                        sh '''
                            # 安装系统依赖
                            apt-get update && apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf

                            cd log-analyzer
                            npm ci
                            npm run tauri build -- --target x86_64-unknown-linux-gnu
                        '''
                        archiveArtifacts artifacts: 'src-tauri/target/release/bundle/**/*', allowEmptyArchive: true
                    }
                }

                // 子阶段 7.2: 构建 Windows (需要 Windows 节点)
                stage('Build Windows') {
                    agent {
                        label 'windows'
                    }
                    when {
                        anyOf {
                            expression { return params.BUILD_TARGET == 'windows' || params.BUILD_TARGET == 'all' }
                            expression { return params.BUILD_RELEASE }
                        }
                    }
                    steps {
                        bat '''
                            cd log-analyzer
                            npm ci
                            npm run tauri build -- --target x86_64-pc-windows-msvc
                        '''
                        archiveArtifacts artifacts: 'src-tauri/target/release/bundle/**/*', allowEmptyArchive: true
                    }
                }

                // 子阶段 7.3: 构建 macOS (需要 macOS 节点)
                stage('Build macOS') {
                    agent {
                        label 'macos'
                    }
                    when {
                        anyOf {
                            expression { return params.BUILD_TARGET == 'macos' || params.BUILD_TARGET == 'all' }
                            expression { return params.BUILD_RELEASE }
                        }
                    }
                    steps {
                        sh '''
                            cd log-analyzer
                            npm ci
                            npm run tauri build -- --target x86_64-apple-darwin
                        '''
                        archiveArtifacts artifacts: 'src-tauri/target/release/bundle/**/*', allowEmptyArchive: true
                    }
                }
            }
        }

        // 阶段 8: 代码质量报告
        stage('Quality Report') {
            agent {
                docker {
                    image 'rust:1.70'
                    args '-v /var/run/docker.sock:/var/run/docker.sock'
                }
            }
            steps {
                sh '''
                    cd log-analyzer/src-tauri
                    cargo install cargo-bloat cargo-deps

                    # 生成代码统计
                    echo "# 代码质量报告" > code-quality-report.md
                    echo "## 基本信息" >> code-quality-report.md
                    echo "- 构建时间: $(date)" >> code-quality-report.md
                    echo "- Rust 版本: $(rustc --version)" >> code-quality-report.md
                    echo "- Git 提交: ${env.GIT_COMMIT}" >> code-quality-report.md
                    echo "" >> code-quality-report.md

                    # 代码统计
                    echo "## 代码统计" >> code-quality-report.md
                    echo "- 总行数: $(find src -name '*.rs' -exec wc -l {} + | tail -1 | awk '{print $1}')" >> code-quality-report.md
                    echo "- 测试用例数: $(find src -name '*.rs' -exec grep -l '#\\[test\\]' {} \\; | wc -l)" >> code-quality-report.md
                    echo "- 模块数: $(find src -name 'mod.rs' | wc -l)" >> code-quality-report.md

                    # 生成依赖报告
                    cargo deps --tree --depth 1 > deps-tree.txt
                '''
                publishHTML([
                    allowMissing: false,
                    alwaysLinkToLastBuild: true,
                    keepAll: true,
                    reportDir: 'log-analyzer/src-tauri',
                    reportFiles: 'code-quality-report.md',
                    reportName: 'Code Quality Report'
                ])
                archiveArtifacts artifacts: 'deps-tree.txt,code-quality-report.md'
            }
        }
    }

    // 构建后处理
    post {
        // 成功时
        success {
            script {
                echo "=== 构建成功 ==="
                if (env.BRANCH_NAME == 'main') {
                    // 在 main 分支上发送通知
                    slackSend channel: '#builds', color: 'good', message: "✅ Log Analyzer 构建成功: ${env.BUILD_NUMBER}"
                }
            }
        }

        // 失败时
        failure {
            script {
                echo "=== 构建失败 ==="
                slackSend channel: '#builds', color: 'danger', message: "❌ Log Analyzer 构建失败: ${env.BUILD_NUMBER}"
                // 可以添加错误报告或通知
            }
        }

        // 总是执行
        always {
            script {
                echo "=== 构建完成 ==="
                // 清理临时文件
                sh '''
                    rm -rf target/debug/deps/*.d
                    rm -rf target/debug/incremental
                '''
                // 生成构建报告
                writeFile file: 'build-report.json', text: """
                {
                    "buildNumber": "${env.BUILD_NUMBER}",
                    "buildUrl": "${env.BUILD_URL}",
                    "gitCommit": "${env.GIT_COMMIT}",
                    "gitBranch": "${env.BRANCH_NAME}",
                    "buildStatus": "${currentBuild.currentResult}",
                    "timestamp": "${new Date().toISOString()}"
                }
                """
            // 清理
 }
        }

               cleanup {
            script {
                echo "=== 清理构建环境 ==="
                // 清理 Docker 镜像
                sh 'docker system prune -f'
                // 清理 npm 缓存
                sh 'npm cache clean --force || true'
            }
        }
    }
}

// 辅助函数：并行构建多个平台
def buildMultiPlatform() {
    parallel(
        linux: { buildPlatform('linux') },
        windows: { buildPlatform('windows') },
        macos: { buildPlatform('macos') }
    )
}

// 辅助函数：构建特定平台
def buildPlatform(platform) {
    echo "构建 ${platform} 平台..."
    // 根据平台执行不同的构建命令
    switch(platform) {
        case 'linux':
            sh '''
                apt-get update && apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev
                cd log-analyzer && npm run tauri build -- --target x86_64-unknown-linux-gnu
            '''
            break
        case 'windows':
            bat '''
                cd log-analyzer && npm run tauri build -- --target x86_64-pc-windows-msvc
            '''
            break
        case 'macos':
            sh '''
                cd log-analyzer && npm run tauri build -- --target x86_64-apple-darwin
            '''
            break
    }
}
