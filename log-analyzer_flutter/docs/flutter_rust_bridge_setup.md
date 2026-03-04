# Flutter Rust Bridge 配置指南

## 概述

本项目使用 **flutter_rust_bridge** 实现 Flutter 前端与 Rust 后端的高性能 FFI 通信。

**当前状态**: 使用 Tauri HTTP/WebSocket 回退模式

## 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                  Flutter Desktop App                      │
│                                                           │
│  ┌────────────────────────────────────────────────────┐    │
│  │         lib/shared/services/                  │    │
│  │                                          │    │
│  │  ┌────────────────────────────────────────────┐   │    │
│  │  │      BridgeService (桥接服务)         │   │    │
│  │  │                                      │   │    │
│  │  │  Mode: FFI (flutter_rust_bridge)  │   │    │
│  │  │  Fallback: Tauri HTTP/WebSocket   │   │    │
│  │  └────────────────────────────────────────────┘   │    │
│  └────────────────────────────────────────────────────┘    │
│                                                           │
│  ┌────────────────────────────────────────────────────┐    │
│  │              generated/bridge_generated.dart  │    │
│  │     (由 flutter_rust_bridge 生成)         │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Rust Backend                              │
│                                                           │
│  ┌────────────────────────────────────────────────────┐    │
│  │   src-tauri/ffi/ (新增 FFI 模块)     │    │
│  │                                          │    │
│  │  ┌────────────────────────────────────────────┐   │    │
│  │  │   #![ffi_bridge]             │   │    │
│  │  │   mod bridge;                      │   │    │
│  │  │   mod types;                       │   │    │
│  │  │                                     │   │    │
│  │  │   pub use flutter_rust_bridge::*;   │   │    │
│  │  └────────────────────────────────────────────┘   │    │
│  └────────────────────────────────────────────────────┘    │
│                                                           │
│  现有 Tauri Commands 保持兼容                    │
└─────────────────────────────────────────────────────────────────┘
```

## 步骤 1: Rust 后端配置

### 1.1 添加依赖

在 `log-analyzer/src-tauri/Cargo.toml` 中添加:

```toml
[dependencies]
# 现有依赖...
flutter_rust_bridge = "2.0"

[build-dependencies]
flutter_rust_bridge = { version = "2.0", features = ["民兵"] }
```

### 1.2 创建 FFI 模块

创建 `log-analyzer/src-tauri/src/ffi/` 目录：

```
src-tauri/
└── src/
    ├── ffi/
    │   ├── mod.rs              # FFI 模块入口
    │   ├── bridge.rs           # 主桥接实现
    │   ├── commands_bridge.rs  # 将现有 commands 转换为 FFI
    │   └── types.rs           # FFI 专用类型定义
    ├── commands/              # 现有命令保持
    └── lib.rs               # 添加 ffi 模块
```

### 1.3 实现 FFI 模块

**文件**: `src-tauri/src/ffi/mod.rs`

```rust
//! Flutter Rust FFI 桥接模块
//!
//! 提供与 Flutter 前端的高性能通信接口

pub mod bridge;
pub mod commands_bridge;
pub mod types;

// 重新导出常用类型
pub use bridge::*;
pub use types::*;
```

**文件**: `src-tauri/src/ffi/bridge.rs`

```rust
use flutter_rust_bridge::{RustOpaque, RustGlobal};
use std::sync::Arc;
use tokio::sync::Mutex;

/// FRI 桥接上下文
#[derive(Clone)]
pub struct BridgeContext {
    // 这里可以添加全局状态
    // 实际实现时从现有 stores 获取
}

// 全局桥接上下文
static BRIDGE_CONTEXT: RustGlobal<Mutex<BridgeContext>> = RustGlobal::new();

// Flutter Rust Bridge 宏配置
flutter_rust_bridge::frb! {
    /// 初始化桥接
    #[frb(init)]
    pub fn init_bridge() -> BridgeContext {
        let context = BridgeContext {
            // 初始化逻辑
        };
        BRIDGE_CONTEXT.set(context);
        context
    }

    /// 健康检查
    pub fn health_check() -> String {
        "OK".to_string()
    }

    /// 搜索日志 (示例)
    ///
    /// 实际实现应调用现有的 search_engine 模块
    pub fn search_logs(
        query: String,
        workspace_id: Option<String>,
        max_results: i32,
    ) -> String {
        // TODO: 实现搜索逻辑
        // 返回搜索 ID
        uuid::Uuid::new_v4().to_string()
    }

    /// 取消搜索
    pub fn cancel_search(search_id: String) -> bool {
        // TODO: 实现
        true
    }

    /// 获取工作区列表
    pub fn get_workspaces() -> Vec<WorkspaceData> {
        // TODO: 调用 workspace_store
        vec![]
    }

    /// 创建工作区
    pub fn create_workspace(name: String, path: String) -> String {
        // TODO: 调用 workspace API
        uuid::Uuid::new_v4().to_string()
    }

    /// 删除工作区
    pub fn delete_workspace(workspace_id: String) -> bool {
        // TODO: 调用 workspace API
        true
    }

    /// 刷新工作区
    pub fn refresh_workspace(workspace_id: String) -> String {
        // TODO: 返回任务 ID
        uuid::Uuid::new_v4().to_string()
    }

    /// 启动文件监听
    pub fn start_watch(
        workspace_id: String,
        paths: Vec<String>,
        recursive: bool,
    ) -> bool {
        // TODO: 调用 FileWatcher
        true
    }

    /// 停止文件监听
    pub fn stop_watch(workspace_id: String) -> bool {
        // TODO: 调用 FileWatcher
        true
    }

    /// 获取关键词列表
    pub fn get_keywords() -> Vec<KeywordGroupData> {
        // TODO: 调用 keyword_store
        vec![]
    }

    /// 添加关键词组
    pub fn add_keyword_group(group: KeywordGroupInput) -> bool {
        // TODO: 调用 keyword_store
        true
    }

    /// 更新关键词组
    pub fn update_keyword_group(group_id: String, group: KeywordGroupInput) -> bool {
        // TODO: 调用 keyword_store
        true
    }

    /// 删除关键词组
    pub fn delete_keyword_group(group_id: String) -> bool {
        // TODO: 调用 keyword_store
        true
    }

    /// 获取任务列表
    pub fn get_tasks() -> Vec<TaskInfoData> {
        // TODO: 调用 task_store
        vec![]
    }

    /// 取消任务
    pub fn cancel_task(task_id: String) -> bool {
        // TODO: 调用 TaskManager
        true
    }

    /// 加载配置
    pub fn load_config() -> ConfigData {
        // TODO: 调用配置存储
        ConfigData::default()
    }

    /// 保存配置
    pub fn save_config(config: ConfigData) -> bool {
        // TODO: 调用配置存储
        true
    }

    /// 获取性能指标
    pub fn get_performance_metrics(time_range: String) -> PerformanceMetricsData {
        // TODO: 调用 monitoring 模块
        PerformanceMetricsData::default()
    }
}

// FFI 专用类型定义
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceData {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: String,
    pub size: String,
    pub files: i32,
    pub watching: Option<bool>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeywordGroupData {
    pub id: String,
    pub name: String,
    pub color: String,
    pub patterns: Vec<String>,
    pub enabled: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeywordGroupInput {
    pub name: String,
    pub color: String,
    pub patterns: Vec<String>,
    pub enabled: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskInfoData {
    pub task_id: String,
    pub target: String,
    pub message: String,
    pub status: String,
    pub progress: i32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigData {
    pub file_filter: FileFilterConfig,
    pub advanced_features: AdvancedFeaturesConfig,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FileFilterConfig {
    pub enabled: bool,
    pub binary_detection_enabled: bool,
    pub mode: String,
    pub filename_patterns: Vec<String>,
    pub allowed_extensions: Vec<String>,
    pub forbidden_extensions: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvancedFeaturesConfig {
    pub enable_filter_engine: bool,
    pub enable_regex_engine: bool,
    pub enable_time_partition: bool,
    pub enable_autocomplete: bool,
    pub regex_cache_size: i32,
    pub autocomplete_limit: i32,
    pub time_partition_size_secs: i32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetricsData {
    pub search_latency: f64,
    pub search_throughput: f64,
    pub cache_hit_rate: f64,
    pub cache_size: i32,
    pub total_queries: i32,
    pub cache_hits: i32,
    pub latency_history: Vec<f64>,
    pub avg_latency: f64,
}
```

### 1.4 修改 lib.rs

在 `src-tauri/src/lib.rs` 中添加 FFI 模块：

```rust
// 现有导入...
mod ffi; // 添加 FFI 模块

// 现有命令保持不变
mod commands;
// ...
```

### 1.5 启用 FFI 功能编译

在 `src-tauri/Cargo.toml` 中添加特性标志：

```toml
[features]
default = ["ffi"]
ffi = ["flutter_rust_bridge/flutter_rust_bridge"]  # 启用 FFI

# 默认功能
default = []
```

然后启用编译：
```bash
cd log-analyzer/src-tauri
cargo build --features ffi
```

## 步骤 2: Flutter 前端配置

### 2.1 更新 pubspec.yaml

解除注释 flutter_rust_bridge 依赖：

```yaml
dependencies:
  # Rust FFI - flutter_rust_bridge
  flutter_rust_bridge: ^2.0.0  # 取消注释
```

### 2.2 生成 FFI 绑定代码

在 Flutter 项目根目录运行：

```bash
# Windows
flutter_rust_bridge.exe --rust-input ../../log-analyzer/src-tauri/src/ffi/bridge.rs \
  --dart-output ./lib/shared/services/generated/bridge_generated.dart

# macOS/Linux
flutter_rust_bridge --rust-input ../../log-analyzer/src-tauri/src/ffi/bridge.rs \
  --dart-output ./lib/shared/services/generated/bridge_generated.dart
```

### 2.3 更新 API 服务

修改 `lib/shared/services/api_service.dart`：

```dart
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'generated/bridge_generated.dart'; // FFI 生成的代码

class ApiService {
  final BridgeService _bridge = BridgeService();

  // 搜索
  Future<String> searchLogs({
    required String query,
    String? workspaceId,
    int maxResults = 10000,
    Map<String, dynamic>? filters,
  }) async {
    if (_bridge.isFfiEnabled) {
      // 使用 FFI
      return await _bridge.searchLogs(
        query: query,
        workspaceId: workspaceId,
        maxResults: maxResults,
        filters: filters,
      );
    } else {
      // 回退到 Tauri HTTP
      return await _invokeTauri('search_logs', {
        'query': query,
        if (workspaceId != null) 'workspaceId': workspaceId,
        'maxResults': maxResults,
        if (filters != null) 'filters': filters,
      });
    }
  }

  // ... 其他方法类似
}
```

## 步骤 3: 运行时配置

### 3.1 初始化桥接

在 `main.dart` 中初始化：

```dart
void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // 初始化 FFI 桥接
  if (BridgeService().isFfiEnabled) {
    // FFI 模式不需要额外初始化
    debugPrint('使用 FFI 桥接模式');
  } else {
    // Tauri 回退模式需要初始化
    await initializeTauri();
  }

  runApp(const ProviderScope(child: MyApp()));
}
```

### 3.2 条件编译

创建编译标志：

```bash
# 启用 FFI 模式
flutter build --dart-define=USE_FFI_BRIDGE=true

# 使用 Tauri 回退
flutter build --dart-define=USE_FFI_BRIDGE=false
```

## 验证步骤

1. **编译验证**
   ```bash
   cd log-analyzer/src-tauri
   cargo build --features ffi
   ```

2. **代码生成验证**
   ```bash
   flutter_rust_bridge --rust-input ../../log-analyzer/src-tauri/src/ffi/bridge.rs
   ```

3. **运行时验证**
   - 检查 `BridgeService().isFfiEnabled` 返回值
   - 调用 `searchLogs` 确认 FFI 调用成功

## 故障排查

### 问题: FFI 绑定未生成

**症状**: 调用 FFI 函数报 `Method not found`

**解决**:
1. 检查 `bridge_generated.dart` 是否存在
2. 确认 Rust 端 `#[frb(init)]` 函数已导出
3. 重新生成绑定

### 问题: 类型不匹配

**症状**: 编译错误 `type 'X' is not a subtype of 'Y'`

**解决**:
1. 清理 Flutter 缓存: `flutter clean`
2. 重新生成绑定代码
3. 检查 Rust 端和 Flutter 端的类型定义是否一致

### 问题: 运行时崩溃

**症状**: 调用 FFI 函数时应用崩溃

**解决**:
1. 检查 Rust 端 panic
2. 查看 Flutter 日志: `flutter logs`
3. 确认 FFI 初始化顺序正确

## 参考

- [flutter_rust_bridge 官方文档](https://github.com/ltdsdev/flutter_rust_bridge)
- [Tauri 2.0 文档](https://tauri.app/v1/guides/)
- [FFI 最佳实践](https://dart.dev/guides/libraries/c-interop)
