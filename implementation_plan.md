# Implementation Plan

[Overview]
完成 HTTP API 业务逻辑集成，使 Flutter 前端能够通过 RESTful API 访问真实的后端功能。

当前 HTTP API 框架已就绪，但所有端点返回模拟数据。本计划将集成真实的业务逻辑，包括搜索、工作区管理、配置管理等功能。核心挑战是解决 AppState 在 HTTP 服务器独立线程中的共享问题，采用全局状态管理模式（参考现有 `ffi/global_state.rs` 实现）。

[Types]

### HttpApiState（现有，需扩展）
```rust
// 位置: log-analyzer/src-tauri/src/commands/http_api.rs
pub struct HttpApiState {
    pub app_data_dir: std::path::PathBuf,
    pub bind_addr: String,
}
// 扩展: 添加 AppState 访问能力
```

### ApiResponse（现有）
```rust
#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}
```

### SearchRequest（现有）
```rust
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub max_results: Option<usize>,
    pub filters: Option<serde_json::Value>,
}
```

### KeywordRequest（新增）
```rust
#[derive(Deserialize)]
pub struct KeywordRequest {
    pub name: String,
    pub patterns: Vec<String>,
    pub color: String,
    pub enabled: bool,
}
```

### ImportRequest（新增）
```rust
#[derive(Deserialize)]
pub struct ImportRequest {
    pub path: String,
    pub workspace_id: Option<String>,
    pub recursive: Option<bool>,
}
```

### WatchRequest（新增）
```rust
#[derive(Deserialize)]
pub struct WatchRequest {
    pub workspace_id: String,
}
```

[Files]

### 需要修改的文件

1. **`log-analyzer/src-tauri/src/commands/http_api.rs`**
   - 扩展 `HttpApiState` 结构体
   - 修改 `start_http_server` 函数签名
   - 实现所有端点的真实业务逻辑
   - 添加缺失的 API 端点（keywords, watch, import, performance）
   - 添加统一错误处理

2. **`log-analyzer/src-tauri/src/main.rs`**
   - 修改 HTTP 服务器启动调用，传递 `AppState`

### 无需创建新文件

所有更改在现有文件中进行。

[Functions]

### 新增函数

1. **`get_keywords_list()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn get_keywords_list(State(state): State<Arc<RwLock<HttpApiState>>>) -> Json<ApiResponse<Vec<KeywordGroup>>>`
   - 用途: 获取关键词列表，调用 `crate::ffi::global_state::get_app_state()`

2. **`add_keyword_group()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn add_keyword_group(State(state): State<Arc<RwLock<HttpApiState>>>, Json(req): Json<KeywordRequest>) -> Json<ApiResponse<String>>`
   - 用途: 添加关键词组

3. **`update_keyword_group()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn update_keyword_group(axum::extract::Path(id): axum::extract::Path<String>, Json(req): Json<KeywordRequest>) -> Json<ApiResponse<bool>>`
   - 用途: 更新关键词组

4. **`delete_keyword_group()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn delete_keyword_group(axum::extract::Path(id): axum::extract::Path<String>) -> Json<ApiResponse<bool>>`
   - 用途: 删除关键词组

5. **`start_file_watch()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn start_file_watch(Json(req): Json<WatchRequest>) -> Json<ApiResponse<bool>>`
   - 用途: 启动文件监听

6. **`stop_file_watch()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn stop_file_watch(Json(req): Json<WatchRequest>) -> Json<ApiResponse<bool>>`
   - 用途: 停止文件监听

7. **`import_folder()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn import_folder(Json(req): Json<ImportRequest>) -> Json<ApiResponse<String>>`
   - 用途: 导入文件夹

8. **`get_performance_metrics()`**
   - 位置: `src/commands/http_api.rs`
   - 签名: `async fn get_performance_metrics() -> Json<ApiResponse<PerformanceMetrics>>`
   - 用途: 获取性能指标

### 修改的函数

1. **`search_logs()`**
   - 当前: 返回 `total_results: 0`
   - 修改: 调用 `crate::commands::search::search_logs` 获取真实结果

2. **`create_workspace()`**
   - 当前: 仅生成 UUID
   - 修改: 调用 `crate::commands::workspace::create_workspace` 创建真实工作区

3. **`delete_workspace()`**
   - 当前: 返回 `true`
   - 修改: 调用 `crate::commands::workspace::delete_workspace` 真实删除

4. **`load_config()`**
   - 当前: 返回硬编码配置
   - 修改: 调用 `crate::commands::config::load_config`

5. **`save_config()`**
   - 当前: 未实现
   - 修改: 调用 `crate::commands::config::save_config`

6. **`start_http_server()`**
   - 当前: `start_http_server(app_data_dir: PathBuf, bind_addr: String)`
   - 修改: 使用全局状态获取 AppState

7. **`create_router()`**
   - 修改: 添加新路由

[Classes]

本项目使用 Rust，相关结构体已在 [Types] 部分描述。

[Dependencies]

### 现有依赖（已满足）
- `axum` - HTTP 框架
- `tower-http` - CORS 支持
- `tokio` - 异步运行时
- `serde` / `serde_json` - JSON 序列化
- `parking_lot` - 高性能锁

无需新增依赖。

[Testing]

### 测试策略

1. **单元测试**
   - 为每个新增的端点处理函数编写测试
   - 位置: `src/commands/http_api.rs` 末尾的 `#[cfg(test)]` 模块

2. **集成测试**
   - 创建测试验证 HTTP 请求/响应流程
   - 使用 `tokio::test` 进行异步测试

3. **手动验证**
   - 启动 HTTP 服务器后使用 curl 测试

### 测试命令
```bash
# 运行所有测试
cd log-analyzer/src-tauri && cargo test http_api

# 手动测试
curl http://127.0.0.1:8080/health
curl -X POST http://127.0.0.1:8080/api/search -H "Content-Type: application/json" -d '{"query":"error"}'
```

[Implementation Order]

### 实施步骤

1. **第一步：修改 search_logs 端点**
   - 使用 `crate::ffi::global_state::get_app_state()` 获取状态
   - 调用真实搜索逻辑
   - 返回真实搜索结果

2. **第二步：修改 create_workspace 端点**
   - 调用 `crate::commands::workspace::create_workspace`
   - 创建真实工作区目录和元数据

3. **第三步：修改 delete_workspace 端点**
   - 调用 `crate::commands::workspace::delete_workspace`
   - 删除工作区及其数据

4. **第四步：修改 load_config/save_config 端点**
   - 调用 `crate::commands::config::load_config`
   - 调用 `crate::commands::config::save_config`

5. **第五步：添加 keywords API 端点**
   - GET /api/keywords - 获取列表
   - POST /api/keywords - 添加
   - PUT /api/keywords/:id - 更新
   - DELETE /api/keywords/:id - 删除

6. **第六步：添加 watch API 端点**
   - POST /api/watch/start - 启动监听
   - POST /api/watch/stop - 停止监听

7. **第七步：添加 import API 端点**
   - POST /api/import/folder - 导入文件夹

8. **第八步：添加 performance API 端点**
   - GET /api/performance/metrics - 获取性能指标

9. **第九步：验证和测试**
   - 运行 `cargo test`
   - 运行 `cargo clippy`
   - 手动测试所有端点