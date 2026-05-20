# 分布式/远程工作区 — 技术评估

**日期**: 2026  
**状态**: 评估中

## 目标

允许用户将工作区数据存储在远程机器上，通过网络访问日志文件，
实现"本地搜索 + 远程存储"的混合模式。

## 技术方案对比

| 方案 | 实现方式 | 优点 | 缺点 |
|------|---------|------|------|
| **A: 网络挂载** | NFS/SMB/CIFS 挂载远程目录到本地 | 零代码改动 | 依赖系统配置，性能差 |
| **B: REST API** | 在远程机器运行代理服务，通过 HTTP API 读取文件 | 跨平台，标准化 | 需实现服务端 + 认证 |
| **C: gRPC 流式** | gRPC bidirectional streaming 传输文件块 | 高性能，已有 Tantivy mmap 基础 | 复杂度高 |
| **D: 混合模式** | 本地缓存 + 远程同步 (类似 git LFS) | 离线可用 | 存储空间翻倍 |

## 推荐方案: B (REST API) + D (本地缓存)

### 架构设计

```
远程机器                         本地机器
┌─────────────┐                ┌──────────────────┐
│ la-agent     │◄────REST────►│ log-analyzer      │
│ (轻量代理)   │               │  ├── remote_repo   │
│  /files      │               │  ├── local_cache   │
│  /search     │               │  └── CAS (partial) │
└─────────────┘                └──────────────────┘
```

### 实现阶段

1. **Phase A**: 实现 `RemoteFileRepository` (impl LogFileRepository)
   - 通过 HTTP Range 请求按需拉取文件块
   - LRU 本地缓存 (moka)
   
2. **Phase B**: 实现 `la-agent` 轻量代理
   - Rust + axum, ~500 行
   - 端点: GET /files, GET /files/{hash}, POST /search
   
3. **Phase C**: 增量同步
   - WebSocket 通知文件变更
   - 本地缓存自动失效

### 风险

- **网络延迟**: 大文件首次读取慢，需预取策略
- **认证**: 需要 token/JWT 认证机制
- **安全性**: 传输加密 (TLS)，路径遍历防护
- **离线**: 网络断开时功能降级

## 决策

**暂缓**。当前用例(本地日志分析)不需要网络功能。
等 ImportUseCase 迁移完成 + 用户需求明确后再启动。
