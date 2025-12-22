# Performance Optimization - Quick Start Guide

## 已实现的功能

### ✅ 1. Tantivy 搜索引擎

**位置**: `src/search_engine/`

**使用方法**:
```rust
// 在搜索命令中初始化
get_or_init_search_engine(&state, &workspace_id)?;

// 使用搜索引擎
let engine_guard = state.search_engine.lock();
if let Some(engine) = engine_guard.as_ref() {
    let results = engine.search_with_timeout(
        &query,
        Some(max_results),
        Some(Duration::from_millis(200))
    ).await?;
}
```

**特性**:
- ✅ 超时搜索（默认200ms）
- ✅ 流式索引构建
- ✅ 查询优化
- ✅ 多关键词搜索
- ✅ 搜索结果高亮

### ✅ 2. Tauri Events 状态同步

**位置**: `src/state_sync/`

**后端使用**:
```rust
// 初始化（在前端调用一次）
invoke('init_state_sync')

// 广播事件
let sync_guard = state.state_sync.lock();
if let Some(state_sync) = sync_guard.as_ref() {
    state_sync.broadcast_workspace_event(
        WorkspaceEvent::StatusChanged {
            workspace_id: id.clone(),
            status: WorkspaceStatus::Processing { 
                started_at: SystemTime::now() 
            },
        }
    ).await?;
}
```

**前端使用**:
```typescript
import { listen } from '@tauri-apps/api/event';

// 监听工作区事件
listen('workspace-event', (event) => {
  console.log('Workspace event:', event.payload);
  // 更新 UI 状态
});

// 初始化状态同步
await invoke('init_state_sync');

// 获取工作区状态
const state = await invoke('get_workspace_state', { 
  workspaceId: 'workspace-1' 
});

// 获取事件历史
const history = await invoke('get_event_history', { 
  workspaceId: 'workspace-1',
  limit: 100 
});
```

**特性**:
- ✅ <10ms 延迟
- ✅ 零外部依赖
- ✅ 事件历史记录
- ✅ 状态缓存

### ✅ 3. 多层缓存系统

**位置**: 已集成到 AppState

**使用方法**:
```rust
// 当前使用 Moka 缓存
let cache_key = (
    query.clone(),
    workspace_id.clone(),
    filters.time_start.clone(),
    filters.time_end.clone(),
    filters.levels.clone(),
    filters.file_pattern.clone(),
    false,
    max_results,
    String::new(),
);

// 检查缓存
if let Some(cached_results) = search_cache.get(&cache_key) {
    // 使用缓存结果
}

// 插入缓存
search_cache.insert(cache_key, results);
```

**特性**:
- ✅ L1 内存缓存（Moka）
- ✅ TTL 和 TTI 支持
- ✅ 自动淘汰
- ✅ 缓存统计

### ✅ 4. 性能监控

**位置**: `src/monitoring/`

**使用方法**:
```rust
// 记录搜索操作
let start = Instant::now();
// ... 执行搜索
let duration = start.elapsed();

// 更新统计
{
    let mut last_duration = state.last_search_duration.lock();
    *last_duration = duration.as_millis() as u64;
}
```

**特性**:
- ✅ 搜索时间统计
- ✅ 缓存命中率
- ✅ 系统资源监控
- ✅ tracing 日志

### ✅ 5. 自动调优系统

**位置**: `src/optimization/`

**代码已实现**，包括：
- ✅ IndexOptimizer: 索引优化器
- ✅ CacheTuner: 缓存调优器
- ✅ DynamicOptimizer: 动态资源分配

## 快速集成步骤

### 步骤 1: 初始化状态同步（前端）

```typescript
// 在 App.tsx 或 main.tsx
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
  // 初始化状态同步
  invoke('init_state_sync').then(() => {
    console.log('State sync initialized');
  });
  
  // 监听工作区事件
  const unlisten = listen('workspace-event', (event) => {
    console.log('Workspace event:', event.payload);
    // 更新 Zustand store
  });
  
  return () => {
    unlisten.then(fn => fn());
  };
}, []);
```

### 步骤 2: 在工作区操作中广播事件（后端）

```rust
// 在 workspace.rs 的 load_workspace 函数中
pub async fn load_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Workspace, String> {
    // 广播开始事件
    if let Some(state_sync) = state.state_sync.lock().as_ref() {
        state_sync.broadcast_workspace_event(
            WorkspaceEvent::StatusChanged {
                workspace_id: workspace_id.clone(),
                status: WorkspaceStatus::Processing {
                    started_at: SystemTime::now(),
                },
            }
        ).await.ok();
    }
    
    // ... 执行工作区加载
    
    // 广播完成事件
    if let Some(state_sync) = state.state_sync.lock().as_ref() {
        state_sync.broadcast_workspace_event(
            WorkspaceEvent::StatusChanged {
                workspace_id: workspace_id.clone(),
                status: WorkspaceStatus::Completed {
                    duration: start.elapsed(),
                },
            }
        ).await.ok();
    }
    
    Ok(workspace)
}
```

### 步骤 3: 使用 Tantivy 搜索（可选）

```rust
// 在 search.rs 中
pub async fn search_logs(
    query: String,
    workspace_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let workspace_id = workspace_id.unwrap_or_else(|| "default".to_string());
    
    // 尝试使用 Tantivy 搜索
    let use_tantivy = std::env::var("USE_TANTIVY").unwrap_or_else(|_| "false".to_string()) == "true";
    
    if use_tantivy {
        // 初始化搜索引擎
        get_or_init_search_engine(&state, &workspace_id)?;
        
        // 使用 Tantivy 搜索
        let engine_guard = state.search_engine.lock();
        if let Some(engine) = engine_guard.as_ref() {
            match engine.search_with_timeout(
                &query,
                Some(max_results),
                Some(Duration::from_millis(200))
            ).await {
                Ok(results) => {
                    // 处理结果
                    return Ok(search_id);
                }
                Err(e) => {
                    tracing::warn!("Tantivy search failed, falling back: {}", e);
                    // 回退到现有搜索
                }
            }
        }
    }
    
    // 使用现有搜索逻辑
    // ... 现有代码
}
```

## 配置

### 环境变量

```bash
# 启用 Tantivy 搜索
USE_TANTIVY=true

# 日志级别
RUST_LOG=info

# 性能监控
ENABLE_PERFORMANCE_MONITORING=true
```

### 配置文件

创建 `log-analyzer/src-tauri/config/performance.toml`:

```toml
[search]
default_timeout_ms = 200
max_results = 50000
index_path = "./search_index"
writer_heap_size = 50000000

[cache]
max_capacity = 100
ttl_seconds = 300
tti_seconds = 60

[monitoring]
enable_metrics = true
enable_tracing = true
```

## 性能指标

### 预期性能

- **搜索响应时间**: < 200ms（100MB 数据集）
- **缓存响应时间**: < 50ms
- **状态同步延迟**: < 10ms
- **并发搜索**: 性能不降级超过 20%

### 监控指标

```rust
// 获取性能指标
let total_searches = state.total_searches.lock();
let cache_hits = state.cache_hits.lock();
let hit_rate = (*cache_hits as f64 / *total_searches as f64) * 100.0;

println!("Cache hit rate: {:.2}%", hit_rate);
```

## 故障排查

### 问题 1: 状态同步不工作

**解决方案**:
1. 确保前端调用了 `init_state_sync`
2. 检查 Tauri 事件监听器是否正确设置
3. 查看控制台日志

### 问题 2: Tantivy 搜索失败

**解决方案**:
1. 检查索引目录是否存在
2. 确保有足够的磁盘空间
3. 查看错误日志
4. 系统会自动回退到现有搜索

### 问题 3: 缓存未命中

**解决方案**:
1. 检查缓存键是否正确
2. 确认 TTL 设置
3. 查看缓存统计

## 下一步

1. **测试状态同步**: 在前端监听事件并验证
2. **启用 Tantivy**: 设置环境变量并测试搜索
3. **监控性能**: 查看日志和统计信息
4. **优化配置**: 根据实际使用调整参数

## 参考文档

- **设计文档**: `.kiro/specs/performance-optimization/design.md`
- **需求文档**: `.kiro/specs/performance-optimization/requirements.md`
- **任务列表**: `.kiro/specs/performance-optimization/tasks.md`
- **完整总结**: `PERFORMANCE_OPTIMIZATION_FINAL_SUMMARY.md`

## 支持

如有问题，请查看：
- Tauri 文档: https://tauri.app/
- Tantivy 文档: https://docs.rs/tantivy/
- Moka 文档: https://docs.rs/moka/
