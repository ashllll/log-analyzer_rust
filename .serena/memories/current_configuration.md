# 配置文件说明

## 当前配置限制

### ArchiveConfig (压缩包配置)
位置: `log-analyzer/src-tauri/src/models/config.rs`

```rust
pub struct ArchiveConfig {
    // 当前限制
    pub max_file_size: u64,           // 100MB (需要提升到10GB)
    pub max_total_size: u64,          // 1GB (需要无限制)
    pub max_file_count: usize,        // 1000 (需要无限制)
    pub max_extraction_depth: usize,  // 10 (需要提升到15)
    pub max_compression_ratio: f64,   // 100.0
    pub max_workspace_size: u64,      // 50GB
}
```

### ExtractionPolicy (提取策略)
位置: `log-analyzer/src-tauri/src/models/extraction_policy.rs`

```rust
pub struct ExtractionPolicy {
    pub extraction: ExtractionConfig {
        pub max_depth: usize,           // 10 (需要提升到15)
        pub max_file_size: u64,         // 100MB (需要提升到10GB)
        pub max_total_size: u64,        // 10GB (需要无限制)
        pub max_workspace_size: u64,    // 50GB
        pub use_enhanced_extraction: bool, // false
    },
}
```

## 目标配置值

根据优化方案，需要调整为：

```toml
[archive]
max_file_size = 10737418240        # 10GB (从100MB提升)
max_total_size = 0                   # 无限制 (从1GB)
max_file_count = 0                   # 无限制 (从1000)
max_extraction_depth = 15           # 从10提升

[archive_processing]
max_file_size = 10737418240        # 10GB
max_total_size = 0                   # 无限制
max_file_count = 0                   # 无限制
max_nesting_depth = 15               # 从10提升

[archive_processing.nested_archive_policy]
file_count_threshold = 5000
total_size_threshold = 21474836480   # 20GB
compression_ratio_threshold = 100.0

[archive_processing.file_size_policy]
strategy = "streaming_search"
full_extraction_limit = 524288000    # 500MB
streaming_search_limit = 2147483648   # 2GB
```

## 新增配置结构

需要添加 `ArchiveProcessingConfig` 配置结构来支持：
1. 大文件策略选择
2. 嵌套压缩包智能处理
3. 流式搜索配置
