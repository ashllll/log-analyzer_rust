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
type SearchCache = Arc<Mutex<lru::LruCache<(String, String), Vec<LogEntry>>>>;
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
}

// --- Helpers ---

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

    for mut file in entries.flatten() {
        let entry_path = file
            .path()
            .map_err(|e| format!("Invalid tar entry path: {}", e))?;
        let entry_name = entry_path.to_string_lossy().to_string();

        // 安全检查：阻止路径穿越
        if entry_name.contains("..") {
            eprintln!("[WARNING] Skipping unsafe path: {}", entry_name);
            continue;
        }

        // Windows 兼容：处理路径分隔符
        let normalized_name = entry_name.replace('\\', "/");
        let out_path = safe_path_join(&extract_path, &normalized_name);

        if let Some(p) = out_path.parent() {
            let _ = fs::create_dir_all(p);
        }

        // Windows 兼容：在解压前移除只读属性
        if out_path.exists() {
            let _ = remove_readonly(&out_path);
        }

        if file.unpack(&out_path).is_ok() {
            let new_virtual = format!("{}/{}", ctx.virtual_path, normalized_name);

            // 递归处理解压后的文件（文件和目录都需要处理）
            process_path_recursive(
                &out_path,
                &new_virtual,
                ctx.target_root,
                ctx.map,
                ctx.app,
                ctx.task_id,
            );
        }
    }
    Ok(())
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

    for i in 0..archive.len() {
        if let Ok(mut file) = archive.by_index(i) {
            let name_raw = file.name_raw().to_vec();
            let name = decode_filename(&name_raw); // Windows 编码处理

            // 安全检查
            if name.contains("..") {
                eprintln!("[WARNING] Skipping unsafe zip entry: {}", name);
                continue;
            }

            // Windows 兼容：路径分隔符规范化
            let normalized_name = name.replace('\\', "/");
            let out_path = safe_path_join(&extract_path, &normalized_name);

            if file.is_dir() {
                let _ = fs::create_dir_all(&out_path);
            } else {
                if let Some(p) = out_path.parent() {
                    let _ = fs::create_dir_all(p);
                }

                // Windows 兼容：在解压前移除只读属性
                if out_path.exists() {
                    let _ = remove_readonly(&out_path);
                }

                if let Ok(mut outfile) = File::create(&out_path) {
                    if std::io::copy(&mut file, &mut outfile).is_ok() {
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
                }
            }
        }
    }
    Ok(())
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

    // 检查系统是否安装 unrar
    let unrar_cmd = if cfg!(target_os = "windows") {
        "unrar.exe"
    } else {
        "unrar"
    };

    // 检查 unrar 命令是否可用
    let check = Command::new(unrar_cmd).arg("-?").output();

    if check.is_err() {
        eprintln!(
            "[WARNING] unrar command not found, skipping RAR: {}",
            path.display()
        );
        return Err(
            "RAR support requires 'unrar' to be installed. Please install it:\n\
            - macOS: brew install unrar\n\
            - Ubuntu/Debian: sudo apt install unrar\n\
            - Windows: Download from https://www.rarlab.com/rar_add.htm"
                .to_string(),
        );
    }

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
    let app_handle = app.clone();
    let task_id = Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let workspace_id_clone = workspaceId.clone();

    eprintln!(
        "[DEBUG] import_folder called: path={}, workspace_id={}, task_id={}",
        path, workspaceId, task_id
    );

    // ✅ 验证路径存在性和安全性
    let source_path = Path::new(&path);
    if !source_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // ✅ 使用 canonicalize_path 处理 Windows UNC 路径和长路径
    let canonical_path = canonicalize_path(source_path).unwrap_or_else(|e| {
        eprintln!(
            "[WARNING] Path canonicalization failed: {}, using original path",
            e
        );
        source_path.to_path_buf()
    });

    eprintln!("[DEBUG] Canonical path: {}", canonical_path.display());

    // 立即发送初始状态
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

            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: file_name,
                    status: "COMPLETED".to_string(),
                    message: "Done".to_string(),
                    progress: 100,
                },
            );
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
    max_results: Option<usize>, // 可配置限制
    filters: Option<SearchFilters>, // 高级过滤器
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_handle = app.clone();
    let path_map = Arc::clone(&state.path_map); // Arc clone
    let _search_cache = Arc::clone(&state.search_cache);
    let max_results = max_results.unwrap_or(50000);
    let filters = filters.unwrap_or_default();

    // 获取当前工作区 ID（从前端传入，暂时使用 "default"）
    let _workspace_id = "default".to_string(); // TODO: 从前端传入
    let _cache_key = (
        query.clone(),
        _workspace_id.clone(),
        filters.time_start.clone(),
        filters.time_end.clone(),
        filters.levels.clone(),
        filters.file_pattern.clone(),
    );

    // 尝试从缓存获取（暂时禁用缓存，因为 cache_key 类型不匹配）
    /*
    {
        let mut cache_guard = search_cache.lock().expect("Failed to lock search_cache");

        if let Some(cached_results) = cache_guard.get(&cache_key) {
            eprintln!("[DEBUG] Cache HIT for query: {}", query);

            // 发送缓存结果
            let _ = app_handle.emit("search-results", cached_results.as_slice());
            let _ = app_handle.emit("search-complete", cached_results.len());
            return Ok(());
        } else {
            eprintln!("[DEBUG] Cache MISS for query: {}", query);
        }
    }
    */

    std::thread::spawn(move || {
        let start_time = std::time::Instant::now();
        
        if query.is_empty() {
            return;
        }

        let re = match Regex::new(&format!("(?i){}", query)) {
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
        if all_results.len() > max_results {
            all_results.truncate(max_results);
        }

        eprintln!("[DEBUG] Final result count: {}", all_results.len());

        // 缓存结果（暂时禁用）
        /*
        {
            let mut cache_guard = search_cache.lock().expect("Failed to lock search_cache");
            cache_guard.put(cache_key.clone(), all_results.clone());
            eprintln!("[DEBUG] Cached results for query: {}", query);
        }
        */

        // 分批发送结果
        for chunk in all_results.chunks(500) {
            let _ = app_handle.emit("search-results", chunk);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // 记录搜索耗时
        let duration = start_time.elapsed().as_millis() as u64;
        eprintln!("[DEBUG] Search completed in {}ms", duration);

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

    let (path_map, file_metadata) = load_index(&index_path)?;

    // 更新内存中的 path_map 和 metadata
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

    // 保存索引路径
    let mut indices_guard = state
        .workspace_indices
        .lock()
        .map_err(|e| format!("Failed to acquire indices lock: {}", e))?;
    indices_guard.insert(workspaceId, index_path);

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
                    target: path.clone(),
                    status: "RUNNING".to_string(),
                    message: "Scanning file system...".to_string(),
                    progress: 20,
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
                    target: path.clone(),
                    status: "RUNNING".to_string(),
                    message: "Analyzing changes...".to_string(),
                    progress: 40,
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
                    target: path.clone(),
                    status: "RUNNING".to_string(),
                    message: format!(
                        "Processing {} changes...",
                        total_changes
                    ),
                    progress: 60,
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
                    target: path.clone(),
                    status: "RUNNING".to_string(),
                    message: "Saving index...".to_string(),
                    progress: 80,
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
                    target: path.clone(),
                    status: "FAILED".to_string(),
                    message: "Refresh failed".to_string(),
                    progress: 0,
                },
            );
        } else {
            eprintln!("[DEBUG] Refresh completed for task: {}", task_id_clone);
            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: path.clone(),
                    status: "COMPLETED".to_string(),
                    message: "Refresh complete".to_string(),
                    progress: 100,
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
    // 获取内存使用情况（简单估算）
    let memory_used_mb = 0.0; // TODO: 实际应使用 sysinfo 或 procfs 库

    // 获取 path_map 大小
    let path_map_size = state
        .path_map
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .len();

    // 获取缓存大小
    let cache_size = state
        .search_cache
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .len();

    // 获取性能统计
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

    // 计算缓存命中率
    let cache_hit_rate = if total_searches > 0 {
        (cache_hits as f64 / total_searches as f64) * 100.0
    } else {
        0.0
    };

    // 获取索引文件大小
    let index_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("indices");

    let mut total_index_size = 0u64;
    if index_dir.exists() {
        for entry in fs::read_dir(&index_dir).map_err(|e| format!("Read dir error: {}", e))? {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    total_index_size += metadata.len();
                }
            }
        }
    }

    let index_file_size_mb = total_index_size as f64 / 1024.0 / 1024.0;

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
}
