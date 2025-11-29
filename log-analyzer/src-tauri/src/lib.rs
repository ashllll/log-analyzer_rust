use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Emitter, Manager, State};
use tempfile::TempDir;
use uuid::Uuid;
use walkdir::WalkDir;

// --- Data Structures ---

// 类型别名：简化复杂类型定义
// 搜索缓存键：包含查询、工作区ID和过滤条件
type SearchCacheKey = (
    String,                // query
    String,                // workspace_id
    Option<String>,        // time_start
    Option<String>,        // time_end
    Vec<String>,           // levels
    Option<String>,        // file_pattern
);
type SearchCache = Arc<Mutex<lru::LruCache<SearchCacheKey, Vec<LogEntry>>>>;
type PathMapType = HashMap<String, String>;
type MetadataMapType = HashMap<String, FileMetadata>;
type IndexResult = Result<(PathMapType, MetadataMapType), String>;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LogEntry {
    id: usize,
    timestamp: String,
    level: String,
    file: String,
    real_path: String,
    line: usize,
    content: String,
    tags: Vec<String>,
}

#[derive(Serialize, Clone)]
struct TaskProgress {
    task_id: String,
    task_type: String, // 任务类型: "Import", "Export", etc.
    target: String,    // 目标路径或文件名
    status: String,
    message: String,
    progress: u8,
    workspace_id: Option<String>,  // 工作区 ID，用于前端匹配
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    keyword_groups: serde_json::Value,
    workspaces: serde_json::Value,
}

// 新增：索引持久化结构（支持增量更新）
#[derive(Serialize, Deserialize, Debug, Clone)]
struct IndexData {
    path_map: HashMap<String, String>, // real_path -> virtual_path
    file_metadata: HashMap<String, FileMetadata>, // 文件元数据（用于增量更新）
    workspace_id: String,
    created_at: i64,
}

// 文件元数据（用于增量判断）
#[derive(Serialize, Deserialize, Debug, Clone)]
struct FileMetadata {
    modified_time: i64, // 修改时间戳（Unix 时间戳）
    size: u64,          // 文件大小
}

// 高级过滤器
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct SearchFilters {
    time_start: Option<String>,  // 开始时间（ISO 8601 格式）
    time_end: Option<String>,    // 结束时间（ISO 8601 格式）
    levels: Vec<String>,         // 允许的日志级别列表
    file_pattern: Option<String>, // 文件路径匹配模式
}

// 性能监控指标
#[derive(Serialize, Clone, Debug)]
struct PerformanceMetrics {
    memory_used_mb: f64,             // 当前进程内存使用量（MB）
    path_map_size: usize,            // 索引文件映射条目数
    cache_size: usize,               // 搜索缓存条目数
    last_search_duration_ms: u64,    // 最近一次搜索耗时（毫秒）
    cache_hit_rate: f64,             // 缓存命中率（0-100）
    indexed_files_count: usize,      // 已索引文件数量
    index_file_size_mb: f64,         // 索引文件磁盘大小（MB）
}

// 文件变化事件
#[derive(Serialize, Clone, Debug)]
struct FileChangeEvent {
    event_type: String,    // "modified", "created", "deleted"
    file_path: String,     // 变化的文件路径
    workspace_id: String,  // 所属工作区
    timestamp: i64,        // 事件发生时间戳
}

// 监听器状态
struct WatcherState {
    workspace_id: String,                 // 工作区 ID（用于日志记录和调试）
    watched_path: PathBuf,                // 监听的路径（用于计算相对路径）
    file_offsets: HashMap<String, u64>,  // 文件读取偏移量（用于增量读取）
    is_active: bool,                      // 监听器是否活跃
}

struct AppState {
    temp_dir: Mutex<Option<TempDir>>,
    path_map: Arc<Mutex<PathMapType>>,          // 使用 Arc 优化内存
    file_metadata: Arc<Mutex<MetadataMapType>>, // 文件元数据映射
    workspace_indices: Mutex<HashMap<String, PathBuf>>,
    search_cache: SearchCache, // 搜索缓存：(query, workspace_id) -> results
    // 性能统计
    last_search_duration: Arc<Mutex<u64>>,      // 最近搜索耗时（毫秒）
    total_searches: Arc<Mutex<u64>>,            // 总搜索次数
    cache_hits: Arc<Mutex<u64>>,                 // 缓存命中次数
    // 实时监听
    watchers: Arc<Mutex<HashMap<String, WatcherState>>>,  // workspace_id -> WatcherState
    // 临时文件清理队列（用于处理清理失败的情况）
    cleanup_queue: Arc<Mutex<Vec<PathBuf>>>,    // 待清理的路径列表
}

impl Drop for AppState {
    fn drop(&mut self) {
        eprintln!("[INFO] AppState dropping, performing final cleanup...");
        
        // 执行最后的清理队列处理
        process_cleanup_queue(&self.cleanup_queue);
        
        // 打印性能统计摘要
        if let Ok(searches) = self.total_searches.lock() {
            if let Ok(hits) = self.cache_hits.lock() {
                let hit_rate = if *searches > 0 {
                    (*hits as f64 / *searches as f64) * 100.0
                } else {
                    0.0
                };
                eprintln!(
                    "[INFO] Session stats: {} searches, {} cache hits ({:.1}% hit rate)",
                    searches, hits, hit_rate
                );
            }
        }
        
        eprintln!("[INFO] AppState cleanup completed");
    }
}

// --- Helpers ---

/// 检测unrar命令是否可用
/// 
/// # 功能
/// 检测系统中是否安装了unrar工具，用于RAR文件解压
/// 
/// # 返回值
/// - `Ok(true)`: unrar可用
/// - `Ok(false)`: unrar不可用
/// - `Err(String)`: 检测过程出错
/// 
/// # 平台支持
/// - Windows: 检测 unrar.exe
/// - Linux/macOS: 检测 unrar
fn check_unrar_available() -> Result<bool, String> {
    use std::process::Command;
    
    let unrar_cmd = if cfg!(target_os = "windows") {
        "unrar.exe"
    } else {
        "unrar"
    };
    
    match Command::new(unrar_cmd).arg("-?").output() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("Failed to check unrar availability: {}", e)),
    }
}

/// 获取unrar安装指引
/// 
/// # 功能
/// 根据当前操作系统返回相应的unrar安装指令
/// 
/// # 返回值
/// 包含unrar安装指南的字符串
fn get_unrar_install_guide() -> String {
    if cfg!(target_os = "windows") {
        "RAR support requires 'unrar' to be installed.\n\
         Please download from: https://www.rarlab.com/rar_add.htm\n\
         After installation, add unrar.exe to your system PATH.".to_string()
    } else if cfg!(target_os = "macos") {
        "RAR support requires 'unrar' to be installed.\n\
         Install with: brew install unrar".to_string()
    } else {
        "RAR support requires 'unrar' to be installed.\n\
         Install with: sudo apt install unrar (Ubuntu/Debian)\n\
         or: sudo yum install unrar (CentOS/Fedora)".to_string()
    }
}

/// 验证路径参数
/// 
/// # 功能
/// 检查路径是否非空且存在
/// 
/// # 参数
/// - `path`: 要验证的路径字符串
/// - `param_name`: 参数名称（用于错误消息）
/// 
/// # 返回值
/// - `Ok(PathBuf)`: 验证通过，返回规范化后的路径
/// - `Err(String)`: 验证失败，包含错误信息
fn validate_path_param(path: &str, param_name: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err(format!("Parameter '{}' cannot be empty", param_name));
    }
    
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    
    Ok(path_buf)
}

/// 验证workspace_id参数
/// 
/// # 功能
/// 检查workspace_id是否非空且格式合法
/// 
/// # 参数
/// - `workspace_id`: 要验证的工作区ID
/// 
/// # 返回值
/// - `Ok(())`: 验证通过
/// - `Err(String)`: 验证失败，包含错误信息
fn validate_workspace_id(workspace_id: &str) -> Result<(), String> {
    if workspace_id.trim().is_empty() {
        return Err("Workspace ID cannot be empty".to_string());
    }
    
    // 检查是否包含非法字符（避免路径穿越）
    if workspace_id.contains("..") || workspace_id.contains('/') || workspace_id.contains('\\') {
        return Err("Workspace ID contains invalid characters".to_string());
    }
    
    Ok(())
}

/// 文件操作重试辅助函数
/// 
/// # 功能
/// 对可能暂时失败的文件操作进行重试
/// 
/// # 参数
/// - `operation`: 要执行的操作闭包
/// - `max_retries`: 最大重试次数
/// - `delays_ms`: 每次重试的延迟时间（毫秒）
/// - `operation_name`: 操作名称（用于日志）
/// 
/// # 返回值
/// - `Ok(T)`: 操作成功，返回结果
/// - `Err(String)`: 所有重试都失败，返回错误信息
fn retry_file_operation<T, F>(
    mut operation: F,
    max_retries: usize,
    delays_ms: &[u64],
    operation_name: &str,
) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_error = String::new();
    
    for attempt in 0..=max_retries {
        match operation() {
            Ok(result) => {
                if attempt > 0 {
                    eprintln!(
                        "[INFO] {} succeeded after {} retries",
                        operation_name, attempt
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = e.clone();
                
                // 检查是否是可重试的错误
                let is_retryable = e.contains("permission denied") 
                    || e.contains("Access is denied")
                    || e.contains("file is being used")
                    || e.contains("cannot access");
                
                if !is_retryable || attempt >= max_retries {
                    eprintln!(
                        "[ERROR] {} failed after {} attempts: {}",
                        operation_name, attempt + 1, e
                    );
                    break;
                }
                
                // 等待后重试
                let delay = delays_ms.get(attempt).copied().unwrap_or(500);
                eprintln!(
                    "[WARN] {} failed (attempt {}), retrying in {}ms: {}",
                    operation_name, attempt + 1, delay, e
                );
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
        }
    }
    
    Err(format!(
        "{} failed after {} attempts: {}",
        operation_name,
        max_retries + 1,
        last_error
    ))
}

/// 尝试清理临时目录，失败时加入清理队列
/// 
/// # 功能
/// 尝试删除指定的临时目录，如果失败则将路径添加到清理队列
/// 
/// # 参数
/// - `path`: 要清理的路径
/// - `cleanup_queue`: 清理队列的Arc引用
fn try_cleanup_temp_dir(path: &Path, cleanup_queue: &Arc<Mutex<Vec<PathBuf>>>) {
    if !path.exists() {
        return;
    }
    
    // 尝试重试3次删除目录
    let result = retry_file_operation(
        || {
            #[cfg(target_os = "windows")]
            {
                // Windows：递归移除只读属性
                for entry in WalkDir::new(path) {
                    if let Ok(entry) = entry {
                        let _ = remove_readonly(entry.path());
                    }
                }
            }
            
            fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to remove directory: {}", e))
        },
        3,
        &[100, 500, 1000],
        &format!("cleanup_temp_dir({})", path.display()),
    );
    
    match result {
        Ok(_) => {
            eprintln!("[INFO] Successfully cleaned up temp directory: {}", path.display());
        }
        Err(e) => {
            eprintln!(
                "[WARN] Failed to clean up temp directory: {}. Adding to cleanup queue.",
                e
            );
            // 添加到清理队列
            if let Ok(mut queue) = cleanup_queue.lock() {
                queue.push(path.to_path_buf());
            }
        }
    }
}

/// 执行清理队列中的任务
/// 
/// # 功能
/// 尝试清理队列中所有待处理的临时目录
/// 
/// # 参数
/// - `cleanup_queue`: 清理队列的Arc引用
fn process_cleanup_queue(cleanup_queue: &Arc<Mutex<Vec<PathBuf>>>) {
    let paths_to_clean: Vec<PathBuf> = {
        if let Ok(queue) = cleanup_queue.lock() {
            queue.clone()
        } else {
            return;
        }
    };
    
    if paths_to_clean.is_empty() {
        return;
    }
    
    let total_count = paths_to_clean.len();
    
    eprintln!(
        "[INFO] Processing cleanup queue with {} items",
        total_count
    );
    
    let mut successful = 0;
    let mut failed_paths = Vec::new();
    
    for path in &paths_to_clean {
        if !path.exists() {
            successful += 1;
            continue;
        }
        
        match fs::remove_dir_all(path) {
            Ok(_) => {
                eprintln!("[INFO] Successfully cleaned up: {}", path.display());
                successful += 1;
            }
            Err(e) => {
                eprintln!("[WARN] Still cannot clean up {}: {}", path.display(), e);
                failed_paths.push(path.clone());
            }
        }
    }
    
    // 更新清理队列
    if let Ok(mut queue) = cleanup_queue.lock() {
        *queue = failed_paths;
    }
    
    eprintln!(
        "[INFO] Cleanup queue processed: {} successful, {} still pending",
        successful,
        total_count - successful
    );
}

/// Windows 路径规范化（处理 UNC 路径和长路径）
///
/// # 功能
/// - Windows: 使用 dunce 去除 UNC 前缀 `\\?\`，处理超过 MAX_PATH (260) 的路径
/// - Unix-like: 标准规范化，解析符号链接
///
/// # 例子
/// ```ignore
/// // 此例子仅用于说明，不会执行
/// use std::path::Path;
/// let path = Path::new("C:\\very\\long\\path");
/// let canonical = canonicalize_path(path)?;
/// ```
///
/// # 使用场景
/// - ✅ 已集成: `import_folder` 中的路径验证
fn canonicalize_path(path: &Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 dunce 去除 UNC 前缀 \\?\，并处理长路径
        dunce::canonicalize(path).map_err(|e| format!("Path canonicalization failed: {}", e))
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Unix-like: 标准规范化
        path.canonicalize()
            .map_err(|e| format!("Path canonicalization failed: {}", e))
    }
}

// Windows 兼容：移除文件只读属性（避免删除失败）
#[cfg(target_os = "windows")]
#[allow(clippy::permissions_set_readonly_false)] // Windows 特定操作，允许设置可写
fn remove_readonly(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    
    // 使用重试机制
    retry_file_operation(
        || {
            if let Ok(metadata) = path.metadata() {
                // Windows FILE_ATTRIBUTE_READONLY = 0x1
                if metadata.file_attributes() & 0x1 != 0 {
                    let mut perms = metadata.permissions();
                    perms.set_readonly(false);
                    fs::set_permissions(path, perms)
                        .map_err(|e| format!("Failed to remove readonly: {}", e))?;
                }
            }
            Ok(())
        },
        2, // 最多重试2次
        &[100, 300], // 延迟100ms和300ms
        &format!("remove_readonly({})", path.display()),
    )
}

#[cfg(not(target_os = "windows"))]
fn remove_readonly(_path: &Path) -> Result<(), String> {
    // Unix-like: 不需要处理
    Ok(())
}

// Windows 兼容：处理多种编码
fn decode_filename(bytes: &[u8]) -> String {
    // 尝试 UTF-8
    let (cow, _, had_errors) = encoding_rs::UTF_8.decode(bytes);
    if !had_errors && !cow.contains('\u{FFFD}') {
        return cow.into_owned();
    }

    // 尝试 GBK (中文 Windows)
    let (cow_gbk, _, had_errors_gbk) = encoding_rs::GBK.decode(bytes);
    if !had_errors_gbk {
        return cow_gbk.into_owned();
    }

    // Windows-1252 (西文 Windows)
    let (cow_win, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    cow_win.into_owned()
}

/// 跨平台路径规范化（统一路径分隔符）
///
/// # 功能
/// - Windows: 将 `/` 转换为 `\`
/// - Unix-like: 保持 `/` 不变
///
/// # 例子
/// ```ignore
/// // 此例子仅用于说明，不会执行
/// let path = "folder/subfolder/file.txt";
/// let normalized = normalize_path_separator(path);
/// // Windows: "folder\\subfolder\\file.txt"
/// // Linux/macOS: "folder/subfolder/file.txt"
/// ```
///
/// # 使用场景
/// - ✅ 已集成: `process_path_recursive_inner` 中的虚拟路径处理
/// - ✅ 已集成: `process_path_recursive_inner_with_metadata` 中的虚拟路径处理
fn normalize_path_separator(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        path.replace('/', "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string()
    }
}

// 安全的路径拼接（处理 Windows 路径分隔符）
fn safe_path_join(base: &Path, component: &str) -> PathBuf {
    // 移除路径穿越尝试
    let sanitized = component
        .replace("..", "")
        .replace(":", "") // Windows 驱动器符号
        .trim()
        .to_string();

    base.join(sanitized)
}

fn parse_metadata(line: &str) -> (String, String) {
    let level = if line.contains("ERROR") {
        "ERROR"
    } else if line.contains("WARN") {
        "WARN"
    } else if line.contains("INFO") {
        "INFO"
    } else {
        "DEBUG"
    };
    let timestamp = if line.len() > 19 {
        line[0..19].to_string()
    } else {
        "".to_string()
    };
    (timestamp, level.to_string())
}

/// 获取文件元数据（用于增量判断）
///
/// # 功能
/// 提取文件的修改时间和大小，用于：
/// - 增量索引更新（判断文件是否变化）
/// - 索引持久化（保存元数据到磁盘）
///
/// # 返回值
/// - `Ok(FileMetadata)`: 包含 `modified_time` (Unix 时间戳) 和 `size` (字节)
/// - `Err(String)`: 读取失败的错误信息
///
/// # 例子
/// ```ignore
/// // 此例子仅用于说明，不会执行
/// use std::path::Path;
/// let metadata = get_file_metadata(Path::new("file.txt"))?;
/// println!("Modified: {}, Size: {}", metadata.modified_time, metadata.size);
/// ```
///
/// # 使用场景
/// - ✅ 已集成: `process_path_recursive_inner_with_metadata` 中收集普通文件元数据
fn get_file_metadata(path: &Path) -> Result<FileMetadata, String> {
    use std::time::SystemTime;

    let metadata = path
        .metadata()
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modified time: {}", e))?;

    let modified_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Invalid timestamp: {}", e))?
        .as_secs() as i64;

    Ok(FileMetadata {
        modified_time,
        size: metadata.len(),
    })
}

// 保存索引到磁盘（带压缩，Windows 兼容，支持增量更新）
fn save_index(
    app: &AppHandle,
    workspace_id: &str,
    path_map: &HashMap<String, String>,
    file_metadata: &HashMap<String, FileMetadata>,
) -> Result<PathBuf, String> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");
    fs::create_dir_all(&index_dir).map_err(|e| format!("Failed to create index dir: {}", e))?;

    let index_path = index_dir.join(format!("{}.idx.gz", workspace_id)); // 压缩格式
    let index_data = IndexData {
        path_map: path_map.clone(),
        file_metadata: file_metadata.clone(),
        workspace_id: workspace_id.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };

    let encoded =
        bincode::serialize(&index_data).map_err(|e| format!("Serialization error: {}", e))?;
    let file =
        File::create(&index_path).map_err(|e| format!("Failed to create index file: {}", e))?;

    // Gzip 压缩
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(&encoded)
        .map_err(|e| format!("Write error: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Compression error: {}", e))?;

    eprintln!(
        "[DEBUG] Index saved (compressed): {} ({} entries)",
        index_path.display(),
        path_map.len()
    );
    Ok(index_path)
}

// 从磁盘加载索引（带解压，Windows 兼容，返回元数据）
fn load_index(index_path: &Path) -> IndexResult {
    use flate2::read::GzDecoder;

    if !index_path.exists() {
        return Err("Index file not found".to_string());
    }

    let file = File::open(index_path).map_err(|e| format!("Failed to open index file: {}", e))?;

    // 检查是否为压缩格式
    let mut data = Vec::new();
    if index_path.extension().and_then(|s| s.to_str()) == Some("gz") {
        // 解压
        let mut decoder = GzDecoder::new(file);
        decoder
            .read_to_end(&mut data)
            .map_err(|e| format!("Decompression error: {}", e))?;
    } else {
        // 未压缩（兼容旧版本）
        let mut reader = BufReader::new(file);
        reader
            .read_to_end(&mut data)
            .map_err(|e| format!("Read error: {}", e))?;
    }

    let index_data: IndexData =
        bincode::deserialize(&data).map_err(|e| format!("Deserialization error: {}", e))?;

    eprintln!(
        "[DEBUG] Index loaded: {} ({} entries)",
        index_path.display(),
        index_data.path_map.len()
    );
    Ok((index_data.path_map, index_data.file_metadata))
}

// --- Generic Tar Processor (Windows 兼容) ---
// 参数结构体：减少参数数量
struct ArchiveContext<'a> {
    target_root: &'a Path,
    virtual_path: &'a str,
    map: &'a mut PathMapType,
    app: &'a AppHandle,
    task_id: &'a str,
}

fn process_tar_archive<R: std::io::Read>(
    archive: &mut tar::Archive<R>,
    _path: &Path,
    file_name: &str,
    ctx: &mut ArchiveContext,
) -> Result<(), String> {
    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = ctx.target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {}", e))?;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    for mut file in entries.flatten() {
        let entry_result = (|| -> Result<(), String> {
            let entry_path = file
                .path()
                .map_err(|e| format!("Invalid tar entry path: {}", e))?;
            let entry_name = entry_path.to_string_lossy().to_string();

            // 安全检查：阻止路径穿越
            if entry_name.contains("..") {
                return Err(format!("Unsafe path detected: {}", entry_name));
            }

            // Windows 兼容：处理路径分隔符
            let normalized_name = entry_name.replace('\\', "/");
            let out_path = safe_path_join(&extract_path, &normalized_name);

            if let Some(p) = out_path.parent() {
                fs::create_dir_all(p)
                    .map_err(|e| format!("Failed to create parent dir for {}: {}", normalized_name, e))?;
            }

            // Windows 兼容：在解压前移除只读属性
            if out_path.exists() {
                remove_readonly(&out_path)
                    .map_err(|e| format!("Failed to remove readonly for {}: {}", normalized_name, e))?;
            }

            file.unpack(&out_path)
                .map_err(|e| format!("Failed to unpack {}: {}", normalized_name, e))?;

            let new_virtual = format!("{}/{}", ctx.virtual_path, normalized_name);
            // 递归处理解压后的文件
            process_path_recursive(
                &out_path,
                &new_virtual,
                ctx.target_root,
                ctx.map,
                ctx.app,
                ctx.task_id,
            );
            
            Ok(())
        })();

        match entry_result {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                eprintln!("[WARNING] TAR entry extraction failed: {}", e);
                errors.push(e);
                // 继续处理其他文件，不中断整体流程
            }
        }
    }

    eprintln!(
        "[INFO] TAR extraction complete: {} succeeded, {} failed",
        success_count, error_count
    );

    // 如果有部分文件成功，则返回Ok，但在消息中标注部分失败
    if success_count > 0 {
        if error_count > 0 {
            eprintln!("[WARN] TAR archive partially extracted ({} errors)", error_count);
        }
        Ok(())
    } else if error_count > 0 {
        Err(format!(
            "All TAR entries failed to extract. First error: {}",
            errors.first().unwrap_or(&"Unknown error".to_string())
        ))
    } else {
        Ok(())
    }
}

// --- ZIP Processor (Windows 编码优化) ---
fn process_zip_archive(
    path: &Path,
    file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid zip archive: {}", e))?;

    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    for i in 0..archive.len() {
        let entry_result = (|| -> Result<(), String> {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;
            let name_raw = file.name_raw().to_vec();
            let name = decode_filename(&name_raw);

            // 安全检查
            if name.contains("..") {
                return Err(format!("Unsafe zip entry: {}", name));
            }

            // Windows 兼容：路径分隔符规范化
            let normalized_name = name.replace('\\', "/");
            let out_path = safe_path_join(&extract_path, &normalized_name);

            if file.is_dir() {
                fs::create_dir_all(&out_path)
                    .map_err(|e| format!("Failed to create directory {}: {}", normalized_name, e))?;
            } else {
                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create parent dir for {}: {}", normalized_name, e))?;
                }

                // Windows 兼容：在解压前移除只读属性
                if out_path.exists() {
                    remove_readonly(&out_path)
                        .map_err(|e| format!("Failed to remove readonly for {}: {}", normalized_name, e))?;
                }

                let mut outfile = File::create(&out_path)
                    .map_err(|e| format!("Failed to create file {}: {}", normalized_name, e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract {}: {}", normalized_name, e))?;

                let new_virtual = format!("{}/{}", virtual_path, normalized_name);
                // 递归处理
                process_path_recursive(
                    &out_path,
                    &new_virtual,
                    target_root,
                    map,
                    app,
                    task_id,
                );
            }
            
            Ok(())
        })();

        match entry_result {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                eprintln!("[WARNING] ZIP entry extraction failed: {}", e);
                errors.push(e);
                // 继续处理其他文件
            }
        }
    }

    eprintln!(
        "[INFO] ZIP extraction complete: {} succeeded, {} failed",
        success_count, error_count
    );

    // 如果有部分文件成功，则返回Ok
    if success_count > 0 {
        if error_count > 0 {
            eprintln!("[WARN] ZIP archive partially extracted ({} errors)", error_count);
        }
        Ok(())
    } else if error_count > 0 {
        Err(format!(
            "All ZIP entries failed to extract. First error: {}",
            errors.first().unwrap_or(&"Unknown error".to_string())
        ))
    } else {
        Ok(())
    }
}

// --- GZ Processor ---
fn process_gz_file(
    path: &Path,
    file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut gz = flate2::read::GzDecoder::new(reader);

    let base_name = file_name.trim_end_matches(".gz");
    let unique_name = format!(
        "{}_{}",
        base_name,
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("tmp")
    );
    let out_path = target_root.join(&unique_name);

    let mut out_file = File::create(&out_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut gz, &mut out_file).map_err(|e| e.to_string())?;

    let decompressed_virtual = if virtual_path.ends_with(".gz") {
        virtual_path.trim_end_matches(".gz").to_string()
    } else {
        virtual_path.to_string()
    };

    // 关键：递归处理（解压后可能是 tar 或其他压缩格式）
    process_path_recursive(
        &out_path,
        &decompressed_virtual,
        target_root,
        map,
        app,
        task_id,
    );
    Ok(())
}

// --- RAR Processor (使用系统 unrar 命令) ---
fn process_rar_archive(
    path: &Path,
    file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    use std::process::Command;

    // 检测 unrar 是否可用
    let unrar_available = check_unrar_available()
        .map_err(|e| format!("Failed to check unrar availability: {}", e))?;
    
    if !unrar_available {
        let guide = get_unrar_install_guide();
        eprintln!("[WARNING] unrar command not found, skipping RAR: {}", path.display());
        eprintln!("[INFO] {}", guide);
        return Err(guide);
    }

    let unrar_cmd = if cfg!(target_os = "windows") {
        "unrar.exe"
    } else {
        "unrar"
    };

    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    eprintln!(
        "[DEBUG] Extracting RAR: {} -> {}",
        path.display(),
        extract_path.display()
    );

    // 执行 unrar 命令
    let output = Command::new(unrar_cmd)
        .arg("x") // 解压并保持路径
        .arg("-o+") // 覆盖已存在文件
        .arg("-y") // 自动确认
        .arg(path) // 源文件
        .arg(&extract_path) // 目标目录
        .output()
        .map_err(|e| format!("Failed to execute unrar: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("unrar failed: {}", stderr));
    }

    eprintln!(
        "[DEBUG] RAR extracted successfully: {}",
        extract_path.display()
    );

    // 递归处理解压后的内容
    for entry in WalkDir::new(&extract_path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        let relative = entry_path
            .strip_prefix(&extract_path)
            .map_err(|e| format!("Path strip failed: {}", e))?;
        let relative_str = relative.to_string_lossy().replace('\\', "/");
        let new_virtual = format!("{}/{}", virtual_path, relative_str);

        if entry_path.is_file() {
            process_path_recursive(entry_path, &new_virtual, target_root, map, app, task_id);
        }
    }

    Ok(())
}

// --- Deep Recursive Processor (增强版，支持元数据收集) ---

fn process_path_recursive(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) {
    // 错误处理：如果处理失败，不中断整个流程
    if let Err(e) = process_path_recursive_inner(path, virtual_path, target_root, map, app, task_id)
    {
        eprintln!("[WARNING] Failed to process {}: {}", path.display(), e);
        let _ = app.emit(
            "task-update",
            TaskProgress {
                task_id: task_id.to_string(),
                task_type: "Import".to_string(),
                target: "Processing".to_string(),
                status: "RUNNING".to_string(),
                message: format!("Warning: {}", e),
                progress: 50,
                workspace_id: None,  // 这是内部进度更新，没有 workspace_id
            },
        );
    }
}

// 带元数据收集的版本
fn process_path_recursive_with_metadata(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    metadata_map: &mut HashMap<String, FileMetadata>,
    app: &AppHandle,
    task_id: &str,
) {
    if let Err(e) = process_path_recursive_inner_with_metadata(
        path,
        virtual_path,
        target_root,
        map,
        metadata_map,
        app,
        task_id,
    ) {
        eprintln!("[WARNING] Failed to process {}: {}", path.display(), e);
        let _ = app.emit(
            "task-update",
            TaskProgress {
                task_id: task_id.to_string(),
                task_type: "Import".to_string(),
                target: "Processing".to_string(),
                status: "RUNNING".to_string(),
                message: format!("Warning: {}", e),
                progress: 50,
                workspace_id: None,  // 内部进度更新
            },
        );
    }
}

fn process_path_recursive_inner(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    // 处理目录
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);
            process_path_recursive(entry.path(), &new_virtual, target_root, map, app, task_id);
        }
        return Ok(());
    }

    let path_str = path.to_string_lossy();
    let file_name = path
        .file_name()
        .ok_or("Invalid filename")?
        .to_string_lossy();
    let lower_path = path_str.to_lowercase();

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.to_string(),
            task_type: "Import".to_string(),
            target: file_name.to_string(),
            status: "RUNNING".to_string(),
            message: format!("Processing: {}", file_name),
            progress: 50,
            workspace_id: None,  // 文件处理进度
        },
    );

    // 判断文件类型
    let is_zip = lower_path.ends_with(".zip");
    let is_rar = lower_path.ends_with(".rar");
    let is_tar = lower_path.ends_with(".tar");
    let is_tar_gz = lower_path.ends_with(".tar.gz") || lower_path.ends_with(".tgz");
    let is_plain_gz = lower_path.ends_with(".gz") && !is_tar_gz;

    // --- 处理 ZIP ---
    if is_zip {
        return process_zip_archive(
            path,
            &file_name,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
        );
    }

    // --- 处理 RAR ---
    if is_rar {
        return process_rar_archive(
            path,
            &file_name,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
        );
    }

    // --- 处理 TAR / TAR.GZ ---
    if is_tar || is_tar_gz {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut ctx = ArchiveContext {
            target_root,
            virtual_path,
            map,
            app,
            task_id,
        };
        if is_tar_gz {
            let tar = flate2::read::GzDecoder::new(reader);
            let mut archive = tar::Archive::new(tar);
            return process_tar_archive(&mut archive, path, &file_name, &mut ctx);
        } else {
            let mut archive = tar::Archive::new(reader);
            return process_tar_archive(&mut archive, path, &file_name, &mut ctx);
        }
    }

    // --- 处理纯 GZ ---
    if is_plain_gz {
        return process_gz_file(
            path,
            &file_name,
            virtual_path,
            target_root,
            map,
            app,
            task_id,
        );
    }

    // --- 普通文件 ---
    let real_path = path.to_string_lossy().to_string();

    // ✅ 使用 normalize_path_separator 统一路径分隔符
    let normalized_virtual = normalize_path_separator(virtual_path);

    map.insert(real_path, normalized_virtual.clone());
    eprintln!(
        "[DEBUG] regular file indexed: real_path={}, virtual_path={}",
        path.display(),
        normalized_virtual
    );
    Ok(())
}

// 带元数据收集的内部处理函数
fn process_path_recursive_inner_with_metadata(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    metadata_map: &mut HashMap<String, FileMetadata>,
    app: &AppHandle,
    task_id: &str,
) -> Result<(), String> {
    // 处理目录
    if path.is_dir() {
        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_name = entry.file_name().to_string_lossy();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);
            process_path_recursive_with_metadata(
                entry.path(),
                &new_virtual,
                target_root,
                map,
                metadata_map,
                app,
                task_id,
            );
        }
        return Ok(());
    }

    let path_str = path.to_string_lossy();
    let file_name = path
        .file_name()
        .ok_or("Invalid filename")?
        .to_string_lossy();
    let lower_path = path_str.to_lowercase();

    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.to_string(),
            task_type: "Import".to_string(),
            target: file_name.to_string(),
            status: "RUNNING".to_string(),
            message: format!("Processing: {}", file_name),
            progress: 50,
            workspace_id: None,  // 文件处理进度
        },
    );

    // 判断文件类型
    let is_zip = lower_path.ends_with(".zip");
    let is_rar = lower_path.ends_with(".rar");
    let is_tar = lower_path.ends_with(".tar");
    let is_tar_gz = lower_path.ends_with(".tar.gz") || lower_path.ends_with(".tgz");
    let is_plain_gz = lower_path.ends_with(".gz") && !is_tar_gz;

    // 压缩文件不收集元数据，只处理普通文件
    if is_zip || is_rar || is_tar || is_tar_gz || is_plain_gz {
        // 递归调用原始的处理函数（不收集元数据）
        return process_path_recursive_inner(path, virtual_path, target_root, map, app, task_id);
    }

    // --- 普通文件：收集元数据 ---
    let real_path = path.to_string_lossy().to_string();

    // ✅ 使用 normalize_path_separator 统一路径分隔符
    let normalized_virtual = normalize_path_separator(virtual_path);

    map.insert(real_path.clone(), normalized_virtual.clone());

    // 收集文件元数据
    if let Ok(metadata) = get_file_metadata(path) {
        metadata_map.insert(real_path.clone(), metadata);
        eprintln!(
            "[DEBUG] regular file indexed with metadata: real_path={}, virtual_path={}",
            path.display(),
            normalized_virtual
        );
    } else {
        eprintln!(
            "[DEBUG] regular file indexed (no metadata): real_path={}, virtual_path={}",
            path.display(),
            normalized_virtual
        );
    }

    Ok(())
}

// --- Commands ---

#[command]
async fn import_folder(
    app: AppHandle,
    path: String,
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 参数验证
    validate_path_param(&path, "path")?;
    validate_workspace_id(&workspaceId)?;
    
    let app_handle = app.clone();
    let task_id = Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let workspace_id_clone = workspaceId.clone();

    eprintln!(
        "[DEBUG] import_folder called: path={}, workspace_id={}, task_id={}",
        path, workspaceId, task_id
    );

    // 验证路径存在性和安全性
    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // 使用 canonicalize_path 处理 Windows UNC 路径和长路径
    let canonical_path = canonicalize_path(source_path).unwrap_or_else(|e| {
        eprintln!(
            "[WARNING] Path canonicalization failed: {}, using original path",
            e
        );
        source_path.to_path_buf()
    });

    eprintln!("[DEBUG] Canonical path: {}", canonical_path.display());

    // 立即发送初始状态
    eprintln!("[DEBUG] Sending initial task-update event: task_id={}, workspace_id={}", task_id, workspaceId);
    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.clone(),
            task_type: "Import".to_string(),
            target: canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
                .to_string(),
            status: "RUNNING".to_string(),
            message: "Starting import...".to_string(),
            progress: 0,
            workspace_id: Some(workspaceId.clone()),  // 添加 workspace_id
        },
    );

    {
        let mut temp_guard = state
            .temp_dir
            .lock()
            .map_err(|e| format!("Failed to acquire temp_dir lock: {}", e))?;
        let mut map_guard = state
            .path_map
            .lock()
            .map_err(|e| format!("Failed to acquire path_map lock: {}", e))?;
        let mut metadata_guard = state
            .file_metadata
            .lock()
            .map_err(|e| format!("Failed to acquire metadata lock: {}", e))?;

        *temp_guard = None;
        map_guard.clear();
        metadata_guard.clear();
        let new_temp = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        *temp_guard = Some(new_temp);
    }

    std::thread::spawn(move || {
        eprintln!(
            "[DEBUG] Processing thread started for task: {}",
            task_id_clone
        );

        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let state = app_handle.state::<AppState>();

            // 使用 ? 操作符代替 unwrap
            let temp_guard = state
                .temp_dir
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?;

            if let Some(ref temp_dir) = *temp_guard {
                let target_base = temp_dir.path();
                let source_path = Path::new(&path);
                let root_name = source_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();

                let _ = app_handle.emit(
                    "task-update",
                    TaskProgress {
                        task_id: task_id_clone.clone(),
                        task_type: "Import".to_string(),
                        target: root_name.to_string(),
                        status: "RUNNING".to_string(),
                        message: "Scanning...".to_string(),
                        progress: 10,
                        workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                    },
                );

                let mut map_guard = state
                    .path_map
                    .lock()
                    .map_err(|e| format!("Lock error: {}", e))?;
                let mut metadata_guard = state
                    .file_metadata
                    .lock()
                    .map_err(|e| format!("Lock error: {}", e))?;

                // 使用带元数据收集的版本
                process_path_recursive_with_metadata(
                    source_path,
                    &root_name,
                    target_base,
                    &mut map_guard,
                    &mut metadata_guard,
                    &app_handle,
                    &task_id_clone,
                );

                eprintln!("[DEBUG] Total files indexed: {}", map_guard.len());
                eprintln!("[DEBUG] Metadata collected: {} files", metadata_guard.len());

                // 保存索引到磁盘（包含元数据）
                match save_index(
                    &app_handle,
                    &workspace_id_clone,
                    &map_guard,
                    &metadata_guard,
                ) {
                    Ok(index_path) => {
                        eprintln!("[DEBUG] Index persisted to: {}", index_path.display());
                        let mut indices_guard = state
                            .workspace_indices
                            .lock()
                            .map_err(|e| format!("Lock error: {}", e))?;
                        indices_guard.insert(workspace_id_clone.clone(), index_path);
                    }
                    Err(e) => {
                        eprintln!("[WARNING] Failed to save index: {}", e);
                    }
                }
                
                // 清理临时目录
                if let Some(ref temp_dir) = *temp_guard {
                    let temp_path = temp_dir.path().to_path_buf();
                    // 释放锁后执行清理（避免在锁内执行IO操作）
                    drop(temp_guard);
                    
                    let cleanup_queue = Arc::clone(&state.cleanup_queue);
                    try_cleanup_temp_dir(&temp_path, &cleanup_queue);
                } else {
                    drop(temp_guard);
                }
            }
            Ok::<(), String>(())
        }));

        if let Err(e) = result {
            eprintln!("[ERROR] Thread panicked: {:?}", e);
            // 提取文件名
            let file_name = Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: file_name.clone(),
                    status: "FAILED".to_string(),
                    message: "Crashed".to_string(),
                    progress: 0,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );
            let _ = app_handle.emit("import-error", "Backend process crashed");
        } else {
            eprintln!(
                "[DEBUG] Processing completed successfully for task: {}",
                task_id_clone
            );
            // 提取文件名
            let file_name = Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            eprintln!("[DEBUG] Sending COMPLETED task-update event: task_id={}, workspace_id={}", task_id_clone, workspace_id_clone);
            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: file_name,
                    status: "COMPLETED".to_string(),
                    message: "Done".to_string(),
                    progress: 100,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );
            eprintln!("[DEBUG] Sending import-complete event: task_id={}", task_id_clone);
            let _ = app_handle.emit("import-complete", task_id_clone);
        }
    });

    Ok(task_id)
}

// 单文件搜索函数（用于并行处理）
fn search_single_file(
    real_path: &str,
    virtual_path: &str,
    re: &Regex,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    if let Ok(file) = File::open(real_path) {
        let reader = BufReader::with_capacity(8192, file); // 8KB 缓冲区

        for (i, line_res) in reader.lines().enumerate() {
            if let Ok(line) = line_res {
                if re.is_match(&line) {
                    let (ts, lvl) = parse_metadata(&line);
                    results.push(LogEntry {
                        id: global_offset + i,
                        timestamp: ts,
                        level: lvl,
                        file: virtual_path.to_string(),
                        real_path: real_path.to_string(),
                        line: i + 1,
                        content: line,
                        tags: vec![],
                    });
                }
            }
        }
    }

    results
}

// ============================================================================
// 增量读取相关功能（用于文件监听）
// ============================================================================

/// 从指定偏移量读取文件新增内容
/// 
/// # 参数
/// - `path`: 文件路径
/// - `offset`: 上次读取的偏移量（字节）
/// 
/// # 返回值
/// - `Ok((lines, new_offset))`: 新读取的行和新的偏移量
/// - `Err(String)`: 错误信息
fn read_file_from_offset(path: &Path, offset: u64) -> Result<(Vec<String>, u64), String> {
    use std::io::{Seek, SeekFrom};
    
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    // 获取当前文件大小
    let file_size = file.metadata()
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();
    
    // 如果文件被截断（小于上次偏移量），从头开始读取
    let start_offset = if file_size < offset {
        eprintln!("[WARNING] File truncated, reading from beginning: {}", path.display());
        0
    } else {
        offset
    };
    
    // 如果没有新内容，直接返回
    if start_offset >= file_size {
        return Ok((Vec::new(), file_size));
    }
    
    // 移动到偏移量位置
    file.seek(SeekFrom::Start(start_offset))
        .map_err(|e| format!("Failed to seek to offset: {}", e))?;
    
    // 读取新增内容
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => lines.push(line),
            Err(e) => {
                eprintln!("[WARNING] Error reading line: {}", e);
                break;
            }
        }
    }
    
    eprintln!(
        "[DEBUG] Read {} new lines from {} (offset: {} -> {})",
        lines.len(),
        path.display(),
        start_offset,
        file_size
    );
    
    Ok((lines, file_size))
}

/// 解析日志行并创建 LogEntry
/// 
/// # 参数
/// - `lines`: 日志行
/// - `file_path`: 文件路径（用于显示）
/// - `real_path`: 实际文件路径
/// - `start_id`: 起始 ID
/// - `start_line_number`: 起始行号
/// 
/// # 返回值
/// - 解析后的 LogEntry 列表
fn parse_log_lines(
    lines: &[String],
    file_path: &str,
    real_path: &str,
    start_id: usize,
    start_line_number: usize,
) -> Vec<LogEntry> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let (timestamp, level) = parse_metadata(line);
            LogEntry {
                id: start_id + i,
                timestamp,
                level,
                file: file_path.to_string(),
                real_path: real_path.to_string(),
                line: start_line_number + i,
                content: line.clone(),
                tags: vec![],
            }
        })
        .collect()
}

/// 将新日志条目添加到工作区索引（增量更新）
/// 
/// # 参数
/// - `workspace_id`: 工作区 ID
/// - `new_entries`: 新的日志条目
/// - `app`: AppHandle
/// - `state`: AppState
/// 
/// # 返回值
/// - `Ok(())`: 成功
/// - `Err(String)`: 错误信息
fn append_to_workspace_index(
    workspace_id: &str,
    new_entries: &[LogEntry],
    app: &AppHandle,
    _state: &AppState,  // 为未来扩展保留（可用于持久化索引更新）
) -> Result<(), String> {
    if new_entries.is_empty() {
        return Ok(());
    }
    
    eprintln!(
        "[DEBUG] Appending {} new entries to workspace: {}",
        new_entries.len(),
        workspace_id
    );
    
    // 发送新日志到前端（实时更新）
    let _ = app.emit("new-logs", new_entries);
    
    // 这里可以选择性地更新持久化索引
    // 为了性能考虑，可以批量更新或定期保存
    // 当前实现：只发送到前端，不立即持久化
    
    eprintln!("[DEBUG] New entries sent to frontend");
    
    Ok(())
}

#[command]
async fn search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] workspaceId: Option<String>, // 工作区ID，用于缓存隔离
    max_results: Option<usize>, // 可配置限制
    filters: Option<SearchFilters>, // 高级过滤器
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 参数验证
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }
    if query.len() > 1000 {
        return Err("Search query too long (max 1000 characters)".to_string());
    }
    
    let app_handle = app.clone();
    let path_map = Arc::clone(&state.path_map); // Arc clone
    let search_cache = Arc::clone(&state.search_cache);
    let total_searches = Arc::clone(&state.total_searches);
    let cache_hits = Arc::clone(&state.cache_hits);
    let last_search_duration = Arc::clone(&state.last_search_duration);
    
    // 限制结果数量，防止内存溢出
    let max_results = max_results.unwrap_or(50000).min(100_000);
    let filters = filters.unwrap_or_default();

    // 获取当前工作区 ID（从前端传入）
    let workspace_id = workspaceId.unwrap_or_else(|| "default".to_string());
    let cache_key: SearchCacheKey = (
        query.clone(),
        workspace_id.clone(),
        filters.time_start.clone(),
        filters.time_end.clone(),
        filters.levels.clone(),
        filters.file_pattern.clone(),
    );

    // 尝试从缓存获取（短时间持有锁）
    {
        let cache_result = {
            let mut cache_guard = search_cache
                .lock()
                .map_err(|e| format!("Failed to lock search_cache: {}", e))?;
            cache_guard.get(&cache_key).cloned()
        }; // 锁在这里释放

        if let Some(cached_results) = cache_result {
            eprintln!("[DEBUG] Cache HIT for query: {}", query);
            
            // 更新缓存统计
            if let Ok(mut hits) = cache_hits.lock() {
                *hits += 1;
            }
            if let Ok(mut searches) = total_searches.lock() {
                *searches += 1;
            }

            // 分批发送缓存结果
            for chunk in cached_results.chunks(500) {
                let _ = app_handle.emit("search-results", chunk);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            let _ = app_handle.emit("search-complete", cached_results.len());
            return Ok(());
        } else {
            eprintln!("[DEBUG] Cache MISS for query: {}", query);
        }
    }

    // 更新搜索统计
    if let Ok(mut searches) = total_searches.lock() {
        *searches += 1;
    }

    std::thread::spawn(move || {
        let start_time = std::time::Instant::now();
        
        if query.is_empty() {
            return;
        }

        // 转义用户输入的特殊字符，将其视为字面量而不是正则表达式
        // 支持 | 分隔的多个关键词，每个关键词都转义后再 OR 组合
        let terms: Vec<String> = query
            .split('|')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| regex::escape(t))  // 转义每个关键词
            .collect();
        
        if terms.is_empty() {
            let _ = app_handle.emit("search-error", "Search query is empty after processing");
            return;
        }
        
        // 使用 OR 组合多个关键词，(?i) 表示不区分大小写
        let pattern = if terms.len() == 1 {
            format!("(?i){}", terms[0])
        } else {
            format!("(?i)({})", terms.join("|"))
        };
        
        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => {
                let _ = app_handle.emit("search-error", format!("Invalid Regex: {}", e));
                return;
            }
        };

        // 锁定并获取数据
        let files: Vec<(String, String)> = {
            let guard = path_map.lock().expect("Failed to lock path_map");
            guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        eprintln!(
            "[DEBUG] Searching {} files with query: {}",
            files.len(),
            query
        );

        // 并行搜索（使用 Rayon）
        let mut all_results: Vec<LogEntry> = files
            .par_iter()
            .enumerate()
            .flat_map(|(idx, (real_path, virtual_path))| {
                search_single_file(
                    real_path,
                    virtual_path,
                    &re,
                    idx * 10000, // 简单的 ID 偏移
                )
            })
            .collect();

        eprintln!("[DEBUG] Found {} results before filtering", all_results.len());

        // 应用高级过滤器
        if !filters.levels.is_empty()
            || filters.time_start.is_some()
            || filters.time_end.is_some()
            || filters.file_pattern.is_some()
        {
            all_results.retain(|entry| {
                // 日志级别过滤
                if !filters.levels.is_empty() && !filters.levels.contains(&entry.level) {
                    return false;
                }

                // 时间范围过滤
                if let Some(ref start) = filters.time_start {
                    if entry.timestamp < *start {
                        return false;
                    }
                }
                if let Some(ref end) = filters.time_end {
                    if entry.timestamp > *end {
                        return false;
                    }
                }

                // 文件来源过滤
                if let Some(ref pattern) = filters.file_pattern {
                    if !entry.file.contains(pattern) && !entry.real_path.contains(pattern) {
                        return false;
                    }
                }

                true
            });

            eprintln!(
                "[DEBUG] {} results after filtering",
                all_results.len()
            );
        }

        // 截取结果（Rayon 不支持 .take()）
        let results_truncated = all_results.len() > max_results;
        if results_truncated {
            all_results.truncate(max_results);
            eprintln!("[WARN] Results truncated to {} (max limit)", max_results);
        }

        eprintln!("[DEBUG] Final result count: {}", all_results.len());

        // 缓存结果（仅当结果未被截断时缓存）
        if !results_truncated && all_results.len() > 0 {
            // 使用 try_lock 避免阻塞，失败时跳过缓存
            if let Ok(mut cache_guard) = search_cache.try_lock() {
                cache_guard.put(cache_key.clone(), all_results.clone());
                eprintln!("[DEBUG] Cached results for query: {}", query);
            } else {
                eprintln!("[DEBUG] Cache lock busy, skipping cache update");
            }
        }

        // 分批发送结果
        for chunk in all_results.chunks(500) {
            let _ = app_handle.emit("search-results", chunk);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // 记录搜索耗时
        let duration = start_time.elapsed().as_millis() as u64;
        eprintln!("[DEBUG] Search completed in {}ms", duration);
        
        // 更新性能统计
        if let Ok(mut last_duration) = last_search_duration.lock() {
            *last_duration = duration;
        }

        let _ = app_handle.emit("search-complete", all_results.len());
    });

    Ok(())
}

// 加载工作区索引
#[command]
async fn load_workspace(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 参数验证
    validate_workspace_id(&workspaceId)?;
    
    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    // 先尝试压缩版本
    let mut index_path = index_dir.join(format!("{}.idx.gz", workspaceId));
    if !index_path.exists() {
        // 如果压缩版不存在，尝试未压缩版（兼容旧版本）
        index_path = index_dir.join(format!("{}.idx", workspaceId));
        if !index_path.exists() {
            return Err(format!("Index not found for workspace: {}", workspaceId));
        }
    }

    // 在锁外加载索引数据（IO操作）
    let (path_map, file_metadata) = load_index(&index_path)?;

    // 短时间持有锁更新内存中的数据
    {
        let mut map_guard = state
            .path_map
            .lock()
            .map_err(|e| format!("Failed to acquire path_map lock: {}", e))?;
        let mut metadata_guard = state
            .file_metadata
            .lock()
            .map_err(|e| format!("Failed to acquire metadata lock: {}", e))?;

        *map_guard = path_map;
        *metadata_guard = file_metadata;

        eprintln!(
            "[DEBUG] Loaded {} files with {} metadata entries",
            map_guard.len(),
            metadata_guard.len()
        );
    } // 锁在这里释放

    // 保存索引路径
    {
        let mut indices_guard = state
            .workspace_indices
            .lock()
            .map_err(|e| format!("Failed to acquire indices lock: {}", e))?;
        indices_guard.insert(workspaceId, index_path);
    } // 锁在这里释放

    Ok(())
}

// 增量索引更新命令
#[command]
async fn refresh_workspace(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let app_handle = app.clone();
    let task_id = Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let workspace_id_clone = workspaceId.clone();

    eprintln!(
        "[DEBUG] refresh_workspace called: path={}, workspace_id={}, task_id={}",
        path, workspaceId, task_id
    );

    // 验证路径
    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let canonical_path = canonicalize_path(source_path).unwrap_or_else(|e| {
        eprintln!(
            "[WARNING] Path canonicalization failed: {}, using original path",
            e
        );
        source_path.to_path_buf()
    });

    // 发送初始状态
    let _ = app.emit(
        "task-update",
        TaskProgress {
            task_id: task_id.clone(),
            task_type: "Refresh".to_string(),
            target: canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
                .to_string(),
            status: "RUNNING".to_string(),
            message: "Loading existing index...".to_string(),
            progress: 0,
            workspace_id: Some(workspaceId.clone()),  // 添加 workspace_id
        },
    );

    // 加载现有索引
    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    let mut index_path = index_dir.join(format!("{}.idx.gz", workspaceId));
    if !index_path.exists() {
        index_path = index_dir.join(format!("{}.idx", workspaceId));
        if !index_path.exists() {
            // 如果索引不存在，执行完整导入
            eprintln!("[DEBUG] Index not found, performing full import");
            return import_folder(app, path, workspaceId, state).await;
        }
    }

    std::thread::spawn(move || {
        eprintln!(
            "[DEBUG] Refresh thread started for task: {}",
            task_id_clone
        );
        
        // 提取文件名用于 target 字段
        let file_name = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            // 加载现有索引
            let (mut existing_path_map, mut existing_metadata) = match load_index(&index_path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("[ERROR] Failed to load index: {}", e);
                    return Err(format!("Failed to load index: {}", e));
                }
            };

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name.clone(),  // 使用文件名而不是完整路径
                    status: "RUNNING".to_string(),
                    message: "Scanning file system...".to_string(),
                    progress: 20,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );

            // 扫描当前文件系统
            let mut current_files: HashMap<String, FileMetadata> = HashMap::new();
            let source_path = Path::new(&path);

            for entry in WalkDir::new(source_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let real_path = entry.path().to_string_lossy().to_string();
                if let Ok(metadata) = get_file_metadata(entry.path()) {
                    current_files.insert(real_path, metadata);
                }
            }

            eprintln!("[DEBUG] Current files: {}", current_files.len());
            eprintln!("[DEBUG] Existing files: {}", existing_metadata.len());

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name.clone(),  // 使用文件名
                    status: "RUNNING".to_string(),
                    message: "Analyzing changes...".to_string(),
                    progress: 40,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );

            // 识别变化
            let mut new_files: Vec<String> = Vec::new();
            let mut modified_files: Vec<String> = Vec::new();
            let mut unchanged_files = 0;

            for (real_path, current_meta) in &current_files {
                if let Some(existing_meta) = existing_metadata.get(real_path) {
                    // 文件存在，检查是否修改
                    if existing_meta.modified_time != current_meta.modified_time
                        || existing_meta.size != current_meta.size
                    {
                        modified_files.push(real_path.clone());
                    } else {
                        unchanged_files += 1;
                    }
                } else {
                    // 新文件
                    new_files.push(real_path.clone());
                }
            }

            // 识别删除的文件
            let deleted_files: Vec<String> = existing_metadata
                .keys()
                .filter(|k| !current_files.contains_key(*k))
                .cloned()
                .collect();

            eprintln!(
                "[DEBUG] Changes detected: {} new, {} modified, {} deleted, {} unchanged",
                new_files.len(),
                modified_files.len(),
                deleted_files.len(),
                unchanged_files
            );

            let total_changes = new_files.len() + modified_files.len() + deleted_files.len();

            if total_changes == 0 {
                eprintln!("[DEBUG] No changes detected, skipping update");
                return Ok::<(), String>(());
            }

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name.clone(),  // 使用文件名
                    status: "RUNNING".to_string(),
                    message: format!(
                        "Processing {} changes...",
                        total_changes
                    ),
                    progress: 60,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );

            // 处理新增和修改的文件
            let state = app_handle.state::<AppState>();
            let temp_guard = state
                .temp_dir
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?;

            if let Some(ref temp_dir) = *temp_guard {
                let _target_base = temp_dir.path();
                let root_name = source_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();

                let mut new_entries: HashMap<String, String> = HashMap::new();
                let mut new_metadata_entries: HashMap<String, FileMetadata> = HashMap::new();

                // 处理新增文件
                for real_path in &new_files {
                    let file_path = Path::new(real_path);
                    if let Ok(relative) = file_path.strip_prefix(source_path) {
                        let virtual_path = format!("{}/{}", root_name, relative.to_string_lossy().replace('\\', "/"));
                        let normalized_virtual = normalize_path_separator(&virtual_path);
                        
                        new_entries.insert(real_path.clone(), normalized_virtual);
                        if let Some(meta) = current_files.get(real_path) {
                            new_metadata_entries.insert(real_path.clone(), meta.clone());
                        }
                    }
                }

                // 处理修改的文件
                for real_path in &modified_files {
                    let file_path = Path::new(real_path);
                    if let Ok(relative) = file_path.strip_prefix(source_path) {
                        let virtual_path = format!("{}/{}", root_name, relative.to_string_lossy().replace('\\', "/"));
                        let normalized_virtual = normalize_path_separator(&virtual_path);
                        
                        new_entries.insert(real_path.clone(), normalized_virtual);
                        if let Some(meta) = current_files.get(real_path) {
                            new_metadata_entries.insert(real_path.clone(), meta.clone());
                        }
                    }
                }

                // 合并到现有索引
                for (k, v) in new_entries {
                    existing_path_map.insert(k, v);
                }
                for (k, v) in new_metadata_entries {
                    existing_metadata.insert(k, v);
                }

                // 删除已删除的文件
                for real_path in &deleted_files {
                    existing_path_map.remove(real_path);
                    existing_metadata.remove(real_path);
                }

                eprintln!("[DEBUG] Updated index: {} total files", existing_path_map.len());
            }

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name.clone(),  // 使用文件名
                    status: "RUNNING".to_string(),
                    message: "Saving index...".to_string(),
                    progress: 80,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );

            // 保存更新后的索引
            match save_index(
                &app_handle,
                &workspace_id_clone,
                &existing_path_map,
                &existing_metadata,
            ) {
                Ok(index_path) => {
                    eprintln!("[DEBUG] Index updated: {}", index_path.display());
                    
                    // 更新内存中的索引
                    let state = app_handle.state::<AppState>();
                    let mut map_guard = state
                        .path_map
                        .lock()
                        .map_err(|e| format!("Lock error: {}", e))?;
                    let mut metadata_guard = state
                        .file_metadata
                        .lock()
                        .map_err(|e| format!("Lock error: {}", e))?;
                    
                    *map_guard = existing_path_map;
                    *metadata_guard = existing_metadata;
                }
                Err(e) => {
                    eprintln!("[WARNING] Failed to save index: {}", e);
                    return Err(e);
                }
            }

            Ok::<(), String>(())
        }));

        if let Err(e) = result {
            eprintln!("[ERROR] Refresh thread panicked: {:?}", e);
            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name.clone(),  // 使用文件名
                    status: "FAILED".to_string(),
                    message: "Refresh failed".to_string(),
                    progress: 0,
                    workspace_id: Some(workspace_id_clone.clone()),  // 添加 workspace_id
                },
            );
        } else {
            eprintln!("[DEBUG] Refresh completed for task: {}", task_id_clone);
            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name,  // 使用文件名
                    status: "COMPLETED".to_string(),
                    message: "Refresh complete".to_string(),
                    progress: 100,
                    workspace_id: Some(workspace_id_clone),  // 添加 workspace_id
                },
            );
            let _ = app_handle.emit("import-complete", task_id_clone);
        }
    });

    Ok(task_id)
}

// 导出结果命令
#[command]
async fn export_results(
    results: Vec<LogEntry>,
    format: String,
    #[allow(non_snake_case)] savePath: String,
) -> Result<String, String> {
    eprintln!(
        "[DEBUG] export_results called: format={}, path={}, count={}",
        format,
        savePath,
        results.len()
    );

    match format.as_str() {
        "csv" => export_to_csv(&results, &savePath),
        "json" => export_to_json(&results, &savePath),
        _ => Err(format!("Unsupported export format: {}", format)),
    }
}

// CSV 导出功能
fn export_to_csv(results: &[LogEntry], path: &str) -> Result<String, String> {
    use std::io::Write;

    let file = File::create(path).map_err(|e| format!("Failed to create CSV file: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);

    // 写入 UTF-8 BOM（兼容 Excel）
    writer
        .write_all(b"\xEF\xBB\xBF")
        .map_err(|e| format!("Failed to write BOM: {}", e))?;

    // 写入 CSV 头部
    writeln!(
        writer,
        "ID,Timestamp,Level,File,Line,Content"
    )
    .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // 写入数据行
    for entry in results {
        // CSV 转义：双引号需要加倍，包含逗号、换行符或双引号的字段需用双引号包裹
        let content_escaped = entry
            .content
            .replace('"', "\"\"")
            .replace('\n', " ")
            .replace('\r', "");
        let file_escaped = entry.file.replace('"', "\"\"");

        writeln!(
            writer,
            "{},\"{}\",{},\"{}\",{},\"{}\"",
            entry.id,
            entry.timestamp,
            entry.level,
            file_escaped,
            entry.line,
            content_escaped
        )
        .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    writer
        .flush()
        .map_err(|e| format!("Failed to flush CSV file: {}", e))?;

    eprintln!("[DEBUG] CSV export completed: {} rows written", results.len());
    Ok(path.to_string())
}

// JSON 导出功能
fn export_to_json(results: &[LogEntry], path: &str) -> Result<String, String> {
    use serde_json::json;

    let export_data = json!({
        "metadata": {
            "exportTime": chrono::Utc::now().to_rfc3339(),
            "totalCount": results.len(),
        },
        "results": results,
    });

    let json_string = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    fs::write(path, json_string).map_err(|e| format!("Failed to write JSON file: {}", e))?;

    eprintln!("[DEBUG] JSON export completed: {} entries", results.len());
    Ok(path.to_string())
}

// 获取性能指标命令
#[command]
async fn get_performance_metrics(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PerformanceMetrics, String> {
    // 1. 获取内存使用量
    let memory_used_mb = get_process_memory_mb();

    // 2. 获取 path_map 大小
    let path_map_size = state
        .path_map
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .len();

    // 3. 获取缓存大小
    let cache_size = state
        .search_cache
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .len();

    // 4. 获取性能统计
    let last_search_duration_ms = *state
        .last_search_duration
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let total_searches = *state
        .total_searches
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let cache_hits = *state
        .cache_hits
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // 5. 计算缓存命中率
    let cache_hit_rate = if total_searches > 0 {
        (cache_hits as f64 / total_searches as f64) * 100.0
    } else {
        0.0
    };

    // 6. 获取索引文件大小（递归计算）
    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    let index_file_size_mb = if index_dir.exists() {
        calculate_dir_size(&index_dir)
            .map(|bytes| bytes as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    Ok(PerformanceMetrics {
        memory_used_mb,
        path_map_size,
        cache_size,
        last_search_duration_ms,
        cache_hit_rate,
        indexed_files_count: path_map_size,
        index_file_size_mb,
    })
}

/// 获取当前进程内存使用量（MB）
/// 
/// 使用平台特定的 API 获取进程的常驻内存大小（RSS）
fn get_process_memory_mb() -> f64 {
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        
        #[repr(C)]
        #[allow(non_snake_case)]
        struct PROCESS_MEMORY_COUNTERS {
            cb: u32,
            PageFaultCount: u32,
            PeakWorkingSetSize: usize,
            WorkingSetSize: usize,
            QuotaPeakPagedPoolUsage: usize,
            QuotaPagedPoolUsage: usize,
            QuotaPeakNonPagedPoolUsage: usize,
            QuotaNonPagedPoolUsage: usize,
            PagefileUsage: usize,
            PeakPagefileUsage: usize,
        }
        
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn GetProcessMemoryInfo(
                process: *mut std::ffi::c_void,
                ppsmemCounters: *mut PROCESS_MEMORY_COUNTERS,
                cb: u32,
            ) -> i32;
        }
        
        unsafe {
            let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
            pmc.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            
            let process = GetCurrentProcess();
            if GetProcessMemoryInfo(process, &mut pmc, pmc.cb) != 0 {
                return pmc.WorkingSetSize as f64 / 1024.0 / 1024.0;
            }
        }
        
        0.0
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/macOS: 简化实现，返回 0
        0.0
    }
}

/// 递归计算目录总大小
fn calculate_dir_size(dir: &Path) -> Result<u64, std::io::Error> {
    let mut total_size = 0u64;
    
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                total_size += calculate_dir_size(&path)?;
            } else if path.is_file() {
                total_size += entry.metadata()?.len();
            }
        }
    }
    
    Ok(total_size)
}

/// 检查RAR支持状态
#[command]
async fn check_rar_support() -> Result<serde_json::Value, String> {
    let available = check_unrar_available()
        .map_err(|e| format!("Failed to check RAR support: {}", e))?;
    
    let install_guide = if !available {
        Some(get_unrar_install_guide())
    } else {
        None
    };
    
    Ok(serde_json::json!({
        "available": available,
        "install_guide": install_guide,
    }))
}

// 实时监听命令
#[command]
async fn start_watch(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    #[allow(non_snake_case)] autoSearch: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use notify::{Watcher, RecursiveMode, recommended_watcher, Event};
    use std::sync::mpsc::channel;
    
    // 参数验证
    validate_workspace_id(&workspaceId)?;
    validate_path_param(&path, "path")?;
    
    eprintln!(
        "[DEBUG] start_watch called: workspace_id={}, path={}, auto_search={:?}",
        workspaceId, path, autoSearch
    );

    // 验证路径
    let watch_path = PathBuf::from(&path);
    if !watch_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // 检查是否已经在监听
    {
        let watchers = state.watchers.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        if watchers.contains_key(&workspaceId) {
            return Err("Workspace is already being watched".to_string());
        }
    }

    // 创建监听器状态
    let watcher_state = WatcherState {
        workspace_id: workspaceId.clone(),
        watched_path: watch_path.clone(),
        file_offsets: HashMap::new(),
        is_active: true,
    };

    // 添加到状态管理
    {
        let mut watchers = state.watchers.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        watchers.insert(workspaceId.clone(), watcher_state);
    }

    // 在后台线程中启动监听器
    let app_handle = app.clone();
    let workspace_id_clone = workspaceId.clone();
    let watch_path_clone = watch_path.clone();
    let watchers_arc = Arc::clone(&state.watchers);
    
    std::thread::spawn(move || {
        eprintln!("[DEBUG] File watcher thread started for workspace: {}", workspace_id_clone);
        
        // 创建事件通道
        let (tx, rx) = channel::<Result<Event, notify::Error>>();
        
        // 创建监听器
        let mut watcher = match recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[ERROR] Failed to create watcher: {}", e);
                return;
            }
        };
        
        // 开始监听
        if let Err(e) = watcher.watch(&watch_path_clone, RecursiveMode::Recursive) {
            eprintln!("[ERROR] Failed to start watching: {}", e);
            return;
        }
        
        eprintln!("[DEBUG] Successfully started watching: {}", watch_path_clone.display());
        
        // 事件处理循环
        for res in rx {
            match res {
                Ok(event) => {
                    eprintln!("[DEBUG] File event received: {:?}", event);
                    
                    // 处理事件
                    let event_type = match event.kind {
                        notify::EventKind::Create(_) => "created",
                        notify::EventKind::Modify(_) => "modified",
                        notify::EventKind::Remove(_) => "deleted",
                        _ => continue,
                    };
                    
                    // 处理每个受影响的文件
                    for path in event.paths {
                        let file_path_str = path.to_string_lossy().to_string();
                        
                        // 发送文件变化事件到前端
                        let file_change = FileChangeEvent {
                            event_type: event_type.to_string(),
                            file_path: file_path_str.clone(),
                            workspace_id: workspace_id_clone.clone(),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        let _ = app_handle.emit("file-changed", file_change);
                        
                        // 如果是文件修改事件，执行增量读取
                        if event_type == "modified" && path.is_file() {
                            eprintln!("[DEBUG] Processing modified file: {}", path.display());
                            
                            // 获取上次偏移量
                            let (offset, watcher_workspace_id, watcher_watched_path) = {
                                if let Ok(mut watchers) = watchers_arc.lock() {
                                    if let Some(watcher) = watchers.get_mut(&workspace_id_clone) {
                                        let offset = *watcher.file_offsets.get(&file_path_str).unwrap_or(&0);
                                        let workspace_id = watcher.workspace_id.clone();
                                        let watched_path = watcher.watched_path.clone();
                                        (offset, workspace_id, watched_path)
                                    } else {
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            };
                            
                            eprintln!(
                                "[DEBUG] Reading from offset {} for file: {}",
                                offset,
                                path.display()
                            );
                            
                            // 增量读取文件
                            match read_file_from_offset(&path, offset) {
                                Ok((new_lines, new_offset)) => {
                                    if !new_lines.is_empty() {
                                        eprintln!(
                                            "[DEBUG] Read {} new lines from {}",
                                            new_lines.len(),
                                            path.display()
                                        );
                                        
                                        // 计算起始行号
                                        let start_line_number = if offset == 0 {
                                            1
                                        } else {
                                            // 估算行号（简化实现）
                                            (offset / 100) as usize + 1
                                        };
                                        
                                        // 解析日志行
                                        let virtual_path = path
                                            .strip_prefix(&watcher_watched_path)
                                            .ok()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or(path.to_str().unwrap_or("unknown"));
                                        
                                        let new_entries = parse_log_lines(
                                            &new_lines,
                                            virtual_path,
                                            &file_path_str,
                                            0, // 临时 ID，实际应用中应该使用全局计数器
                                            start_line_number,
                                        );
                                        
                                        // 发送新日志到前端
                                        let state = app_handle.state::<AppState>();
                                        let _ = append_to_workspace_index(
                                            &watcher_workspace_id,
                                            &new_entries,
                                            &app_handle,
                                            &state,
                                        );
                                        
                                        eprintln!(
                                            "[DEBUG] Sent {} new log entries to frontend",
                                            new_entries.len()
                                        );
                                    }
                                    
                                    // 更新偏移量
                                    if let Ok(mut watchers) = watchers_arc.lock() {
                                        if let Some(watcher) = watchers.get_mut(&workspace_id_clone) {
                                            watcher.file_offsets.insert(file_path_str.clone(), new_offset);
                                            eprintln!(
                                                "[DEBUG] Updated offset for {}: {} -> {}",
                                                path.display(),
                                                offset,
                                                new_offset
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[WARNING] Failed to read file incrementally: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                    
                    // 检查是否还在活跃
                    let is_active = {
                        if let Ok(watchers) = watchers_arc.lock() {
                            watchers.get(&workspace_id_clone)
                                .map(|w| w.is_active)
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    };
                    
                    if !is_active {
                        eprintln!("[DEBUG] Watcher deactivated, stopping thread");
                        break;
                    }
                },
                Err(e) => {
                    eprintln!("[ERROR] Watch error: {}", e);
                }
            }
        }
        
        eprintln!("[DEBUG] File watcher thread terminated for workspace: {}", workspace_id_clone);
    });

    Ok(())
}

#[command]
async fn stop_watch(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!("[DEBUG] stop_watch called: workspace_id={}", workspaceId);
    
    // 标记监听器为不活跃
    let mut watchers = state.watchers.lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    
    if let Some(watcher_state) = watchers.get_mut(&workspaceId) {
        watcher_state.is_active = false;
        eprintln!("[DEBUG] Watcher deactivated for workspace: {}", workspaceId);
    } else {
        return Err("No active watcher found for this workspace".to_string());
    }
    
    // 从状态中移除
    watchers.remove(&workspaceId);
    
    eprintln!("[DEBUG] Watcher removed from state for workspace: {}", workspaceId);
    Ok(())
}

#[command]
fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }
    let path = config_dir.join("config.json");
    fs::write(path, serde_json::to_string_pretty(&config).unwrap()).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");
    if path.exists() {
        let c = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Ok(serde_json::from_str(&c).unwrap_or(AppConfig {
            keyword_groups: serde_json::json!([]),
            workspaces: serde_json::json!([]),
        }))
    } else {
        Ok(AppConfig {
            keyword_groups: serde_json::json!([]),
            workspaces: serde_json::json!([]),
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 设置全局 panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[PANIC] Application panic: {:?}", panic_info);
    }));

    // 配置 Rayon 线程池（优化多核性能）
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4); // 默认 4 线程

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus)
        .thread_name(|idx| format!("rayon-worker-{}", idx))
        .build_global()
        .expect("Failed to build Rayon thread pool");

    eprintln!(
        "[INFO] Rayon thread pool initialized with {} threads",
        num_cpus
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            temp_dir: Mutex::new(None),
            path_map: Arc::new(Mutex::new(HashMap::new())), // 使用 Arc
            file_metadata: Arc::new(Mutex::new(HashMap::new())), // 元数据
            workspace_indices: Mutex::new(HashMap::new()),
            search_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(), // 缓存 100 个搜索结果
            ))),
            // 性能统计
            last_search_duration: Arc::new(Mutex::new(0)),
            total_searches: Arc::new(Mutex::new(0)),
            cache_hits: Arc::new(Mutex::new(0)),
            // 实时监听
            watchers: Arc::new(Mutex::new(HashMap::new())),
            // 临时文件清理队列
            cleanup_queue: Arc::new(Mutex::new(Vec::new())),
        })
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            search_logs,
            import_folder,
            load_workspace,
            refresh_workspace,
            export_results,
            get_performance_metrics,
            check_rar_support,
            start_watch,
            stop_watch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ============================================================================
// 单元测试（私有函数）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_canonicalize_path() {
        let current_dir = std::env::current_dir().unwrap();
        let canonical = canonicalize_path(&current_dir);
        assert!(canonical.is_ok());

        let non_existent = Path::new("/path/that/does/not/exist/123456789");
        let result = canonicalize_path(non_existent);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_path_separator() {
        let path = "folder/subfolder/file.txt";
        let normalized = normalize_path_separator(path);

        #[cfg(target_os = "windows")]
        assert_eq!(normalized, "folder\\subfolder\\file.txt");

        #[cfg(not(target_os = "windows"))]
        assert_eq!(normalized, "folder/subfolder/file.txt");
    }

    #[test]
    fn test_remove_readonly() -> Result<(), String> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let test_file = temp_dir.path().join("readonly_test.txt");

        fs::write(&test_file, "test").map_err(|e| e.to_string())?;

        let metadata = test_file.metadata().map_err(|e| e.to_string())?;
        let mut perms = metadata.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&test_file, perms).map_err(|e| e.to_string())?;

        let result = remove_readonly(&test_file);
        assert!(result.is_ok());

        #[cfg(target_os = "windows")]
        {
            let metadata = test_file.metadata().map_err(|e| e.to_string())?;
            assert!(!metadata.permissions().readonly());
        }

        Ok(())
    }

    #[test]
    fn test_get_file_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("metadata_test.txt");

        fs::write(&test_file, "test content").unwrap();

        let metadata = get_file_metadata(&test_file);
        assert!(metadata.is_ok());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.size, 12);
        assert!(metadata.modified_time > 0);
    }

    #[test]
    fn test_parse_metadata() {
        let (ts, lvl) = parse_metadata("2024-01-01 12:00:00 ERROR Something went wrong");
        assert_eq!(lvl, "ERROR");
        assert_eq!(ts, "2024-01-01 12:00:00");

        let (_ts, lvl) = parse_metadata("2024-01-01 12:00:00 WARN Warning message");
        assert_eq!(lvl, "WARN");

        let (_ts, lvl) = parse_metadata("2024-01-01 12:00:00 INFO Info message");
        assert_eq!(lvl, "INFO");

        let (_ts, lvl) = parse_metadata("2024-01-01 12:00:00 Other message");
        assert_eq!(lvl, "DEBUG");

        let (ts, _) = parse_metadata("short");
        assert_eq!(ts, "");
    }

    #[test]
    fn test_safe_path_join() {
        let base = Path::new("/base");

        // 正常路径
        let result = safe_path_join(base, "normal/path.txt");
        assert!(result.to_string_lossy().contains("normal"));
        assert!(result.to_string_lossy().contains("path.txt"));

        // 路径穿越被清理
        let result = safe_path_join(base, "../../../etc/passwd");
        assert!(!result.to_string_lossy().contains(".."));

        // Windows 驱动器符号被清理
        let result = safe_path_join(base, "C:evil:path");
        assert!(!result.to_string_lossy().contains(":"));
    }

    #[test]
    fn test_decode_filename() {
        let utf8_bytes = "test.txt".as_bytes();
        let result = decode_filename(utf8_bytes);
        assert_eq!(result, "test.txt");

        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let result = decode_filename(&invalid_bytes);
        assert!(result.contains("�") || result.len() > 0);
    }

    #[test]
    fn test_validate_path_param() {
        // 测试空路径
        let result = validate_path_param("", "test_path");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        // 测试不存在的路径
        let result = validate_path_param("/nonexistent/path/12345", "test_path");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));

        // 测试存在的路径
        let current_dir = std::env::current_dir().unwrap();
        let result = validate_path_param(&current_dir.to_string_lossy(), "test_path");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_workspace_id() {
        // 测试空 ID
        let result = validate_workspace_id("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        // 测试包含路径穿越
        let result = validate_workspace_id("../evil");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));

        // 测试包含路径分隔符
        let result = validate_workspace_id("folder/subfolder");
        assert!(result.is_err());

        // 测试合法 ID
        let result = validate_workspace_id("workspace_123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_unrar_available() {
        // 这个测试可能会根据系统配置不同而失败
        let result = check_unrar_available();
        assert!(result.is_ok());
        // 不断言结果，因为不同系统可能有不同的unrar安装状态
    }
}
