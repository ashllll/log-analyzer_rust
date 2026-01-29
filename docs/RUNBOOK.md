# 运行手册 (RUNBOOK)

> 本文档涵盖 Log Analyzer 项目的部署流程、监控告警、常见问题和回滚程序。

## 📋 目录

- [部署流程](#部署流程)
- [监控与告警](#监控与告警)
- [常见问题](#常见问题)
- [回滚程序](#回滚程序)
- [紧急响应](#紧急响应)

---

## 🚀 部署流程

### 环境要求

| 组件 | 要求 |
|------|------|
| 操作系统 | Windows 10+, macOS 11+, Linux (Ubuntu 20.04+) |
| 内存 | 最低 4GB，推荐 8GB+ |
| 磁盘 | 最低 500MB 空间 |
| 权限 | 管理员/root 权限（安装时） |

### 自动化部署 (GitHub Actions)

项目使用 GitHub Actions 进行 CI/CD：

```yaml
# 触发条件
- 推送到 main 分支
- 创建 Release Tag
- 手动触发 workflow_dispatch
```

**自动化步骤**：
1. 代码检查 (ESLint + TypeScript)
2. 前端测试
3. 构建前端
4. Rust 检查 (fmt + clippy)
5. 构建 Tauri 应用
6. 创建 Release/Artifacts

### 本地构建

```bash
cd log-analyzer

# 安装依赖
npm install

# 构建生产版本
npm run build

# 构建 Tauri 应用
npm run tauri build
```

### 手动部署

1. **准备环境**
   ```bash
   # 安装 Node.js 22+
   # 安装 Rust 1.70+
   # 安装 Tauri CLI
   npm install -g @tauri-apps/cli
   ```

2. **构建应用**
   ```bash
   cd log-analyzer
   npm run build
   npm run tauri build
   ```

3. **分发安装包**
   - macOS: `src-tauri/target/release/bundle/dmg/`
   - Windows: `src-tauri/target/release/bundle/msi/`
   - Linux: `src-tauri/target/release/bundle/deb/`

---

## 📊 监控与告警

### 日志位置

| 平台 | 日志路径 |
|------|----------|
| macOS | `~/Library/Logs/com.joeash.log-analyzer/` |
| Linux | `~/.local/share/com.joeash.log-analyzer/logs/` |
| Windows | `%APPDATA%\com.joeash.log-analyzer\logs\` |

### 关键指标

#### 应用性能指标

| 指标 | 正常范围 | 警告阈值 | 危险阈值 |
|------|----------|----------|----------|
| 搜索延迟 | < 10ms | 10-50ms | > 50ms |
| 内存占用 | < 100MB | 100-500MB | > 500MB |
| 启动时间 | < 5s | 5-10s | > 10s |
| 响应时间 | < 100ms | 100-500ms | > 500ms |

#### 系统资源

| 资源 | 监控方法 |
|------|----------|
| CPU | 系统任务管理器 |
| 内存 | `cargo test` 内存报告 |
| 磁盘 | 应用设置中的存储统计 |

### 告警规则

```yaml
# GitHub Actions 监控
- workflow_failed: 立即告警
- test_coverage_drop: 覆盖率 < 80% 时警告
- clippy_warnings: 任何警告视为失败
```

### 性能监控

```bash
# Rust 性能指标
cd src-tauri
cargo bench

# 监控输出:
# - 搜索吞吐量: 10,000+ 次/秒
# - 内存占用: 优化的内存使用
# - 延迟: 毫秒级响应
```

---

## 🔧 常见问题

### 问题 1: 搜索无结果

**症状**: 执行搜索后结果列表为空

**排查步骤**:
1. 检查工作区状态是否为 `READY`
2. 查看后端日志确认索引已加载
3. 验证数据库:
   ```bash
   sqlite3 ~/.local/share/com.joeash.log-analyzer/workspaces/<workspace_id>/metadata.db
   SELECT COUNT(*) FROM files;
   ```
4. 验证搜索关键词（大小写、正则）

**解决方案**:
- 等待工作区状态变为 `READY`
- 重新导入文件
- 检查搜索关键词语法

### 问题 2: 任务一直显示"处理中"

**症状**: 导入文件后任务进度卡住

**排查步骤**:
1. 检查后端日志是否有 UNIQUE constraint 错误
2. 查看任务事件是否正常更新
3. 检查 EventBus 幂等性

**解决方案**:
- 重启应用
- 清除缓存数据
- 重新导入工作区

### 问题 3: 前端报错 "TaskInfo undefined"

**症状**: 控制台报错 `Cannot read properties of undefined`

**排查步骤**:
1. 检查 Rust 结构体字段名与前端 TypeScript 类型是否一致
2. 查看实际接收的 JSON:
   ```javascript
   console.log(JSON.stringify(event.payload, null, 2));
   ```

**解决方案**:
- 确保字段命名统一使用 `snake_case`
- 同步前后端类型定义

### 问题 4: Windows 路径过长错误

**症状**: 导入文件时报错 "File path too long"

**解决方案**:
- 应用已使用 `dunce` crate 处理 UNC 路径
- 使用长路径前缀 `\\?\`
- 将文件移动到更短路径

### 问题 5: CI 构建失败

**排查步骤**:
1. 检查 GitHub Actions 日志
2. 确认环境变量配置正确
3. 运行本地验证:
   ```bash
   npm run validate:ci
   ```

**常见原因**:
- ESLint/TypeScript 错误
- Rust 格式/Clippy 警告
- 测试失败

### 问题 6: RAR 文件解压失败

**症状**: RAR 压缩包解压无结果或报错

**排查步骤**:
1. 确认 RAR 文件版本（RAR4/RAR5）
2. 检查是否密码保护
3. 确认 RAR 文件是否受损或加密

**解决方案**:
- 使用标准 RAR 格式重新打包（避免损坏/异常头）
- 对于加密文件，确认密码正确
- 查看日志获取详细错误信息


---

## 🔄 回滚程序

### Git 回滚

#### 回滚到上一个版本

```bash
# 查看最近的提交
git log --oneline -5

# 回滚到上一个提交
git revert HEAD
git push
```

#### 回滚到指定版本

```bash
# 回滚到指定 commit
git revert <commit-hash>

# 或者使用 reset (需要 force push)
git checkout main
git reset --hard <commit-hash>
git push --force
```

### Docker/制品回滚

```bash
# GitHub Actions 回滚
# 1. 进入 Actions 页面
# 2. 选择之前的成功 workflow
# 3. 点击 "Re-run jobs"

# 或者手动部署旧版本
git checkout <old-version>
npm run tauri build
```

### 数据库回滚

```bash
# SQLite 回滚
# 1. 备份当前数据库
cp metadata.db metadata.db.backup

# 2. 恢复备份
cp metadata.db.backup metadata.db
```

### 回滚验证清单

- [ ] 应用启动正常
- [ ] 核心功能可用
- [ ] 测试通过
- [ ] 日志无异常错误

---

## 🚨 紧急响应

### P0 - 严重故障

**定义**: 应用完全不可用或数据丢失

**响应时间**: 立即

**处理步骤**:
1. 通知团队
2. 评估影响范围
3. 执行回滚
4. 调查根本原因
5. 修复并发布

**联系人**:
- 项目负责人: Joe Ash
- 紧急联系: GitHub Issues

### P1 - 高优先级

**定义**: 核心功能损坏，但有临时解决方案

**响应时间**: 4小时内

**处理步骤**:
1. 创建 Issue 记录
2. 分析问题
3. 开发修复
4. 测试验证
5. 合并发布

### P2 - 中优先级

**定义**: 非核心功能问题或性能下降

**响应时间**: 24小时内

**处理步骤**:
1. 创建 Issue 记录
2. 排入开发计划
3. 正常开发流程修复

---

## 📞 相关链接

- [项目主页](../README.md)
- [变更日志](../CHANGELOG.md)
- [架构文档](architecture/)
- [贡献者指南](CONTRIB.md)
- [GitHub Issues](https://github.com/joeash/log-analyzer_rust/issues)
