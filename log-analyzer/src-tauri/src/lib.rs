use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::{command, AppHandle, Emitter, Manager, State};
use serde::{Deserialize, Serialize};
use regex::Regex;
use walkdir::WalkDir;
use tempfile::TempDir;
use uuid::Uuid;
use std::panic;

// --- Data Structures ---

#[derive(Serialize, Clone, Debug)]
struct LogEntry {
    id: usize, timestamp: String, level: String, file: String, 
    real_path: String, line: usize, content: String, tags: Vec<String>,
}

#[derive(Serialize, Clone)]
struct TaskProgress {
    task_id: String, status: String, message: String, progress: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    keyword_groups: serde_json::Value, 
    workspaces: serde_json::Value,
}

// 新增：索引持久化结构
#[derive(Serialize, Deserialize, Debug, Clone)]
struct IndexData {
    path_map: HashMap<String, String>,
    workspace_id: String,
    created_at: i64,
}

struct AppState {
    temp_dir: Mutex<Option<TempDir>>,
    path_map: Mutex<HashMap<String, String>>,
    workspace_indices: Mutex<HashMap<String, PathBuf>>,  // workspace_id -> index_file_path
}

// --- Helpers ---

fn decode_filename(bytes: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
    if cow.contains('\u{FFFD}') {
        let (cow_gbk, _, _) = encoding_rs::GBK.decode(bytes);
        return cow_gbk.into_owned();
    }
    cow.into_owned()
}

fn parse_metadata(line: &str) -> (String, String) {
    let level = if line.contains("ERROR") { "ERROR" }
    else if line.contains("WARN") { "WARN" }
    else if line.contains("INFO") { "INFO" }
    else { "DEBUG" };
    let timestamp = if line.len() > 19 { line[0..19].to_string() } else { "".to_string() };
    (timestamp, level.to_string())
}

// 新增：保存索引到磁盘
fn save_index(app: &AppHandle, workspace_id: &str, path_map: &HashMap<String, String>) -> Result<PathBuf, String> {
    let index_dir = app.path().app_data_dir().map_err(|e| e.to_string())?
        .join("indices");
    fs::create_dir_all(&index_dir).map_err(|e| e.to_string())?;
    
    let index_path = index_dir.join(format!("{}.idx", workspace_id));
    let index_data = IndexData {
        path_map: path_map.clone(),
        workspace_id: workspace_id.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    
    let encoded = bincode::serialize(&index_data).map_err(|e| e.to_string())?;
    let mut file = File::create(&index_path).map_err(|e| e.to_string())?;
    file.write_all(&encoded).map_err(|e| e.to_string())?;
    
    eprintln!("[DEBUG] Index saved: {} ({} entries)", index_path.display(), path_map.len());
    Ok(index_path)
}

// 新增：从磁盘加载索引
fn load_index(index_path: &Path) -> Result<HashMap<String, String>, String> {
    if !index_path.exists() {
        return Err("Index file not found".to_string());
    }
    
    let data = fs::read(index_path).map_err(|e| e.to_string())?;
    let index_data: IndexData = bincode::deserialize(&data).map_err(|e| e.to_string())?;
    
    eprintln!("[DEBUG] Index loaded: {} ({} entries)", index_path.display(), index_data.path_map.len());
    Ok(index_data.path_map)
}

// --- Generic Tar Processor ---
fn process_tar_archive<R: std::io::Read>(
    archive: &mut tar::Archive<R>,
    _path: &Path,
    file_name: &str,
    target_root: &Path,
    virtual_path: &str,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str
) -> Result<(), String> {
    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path).map_err(|e| e.to_string())?;

    let entries = archive.entries().map_err(|e| e.to_string())?;
    for entry in entries {
        if let Ok(mut file) = entry {
            let entry_name = file.path().unwrap().to_string_lossy().to_string();
            if entry_name.contains("..") { continue; }  // 安全检查
            
            let out_path = extract_path.join(&entry_name);
            
            if let Some(p) = out_path.parent() { 
                let _ = fs::create_dir_all(p); 
            }
            
            if file.unpack(&out_path).is_ok() {
                let new_virtual = format!("{}/{}", virtual_path, entry_name);
                
                // 关键：递归处理解压后的文件（可能是嵌套的压缩包）
                if out_path.is_file() {
                    process_path_recursive(&out_path, &new_virtual, target_root, map, app, task_id);
                } else if out_path.is_dir() {
                    process_path_recursive(&out_path, &new_virtual, target_root, map, app, task_id);
                }
            }
        }
    }
    Ok(())
}

// --- ZIP Processor ---
fn process_zip_archive(
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
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;
    
    let extract_folder_name = format!("{}_extracted_{}", file_name, Uuid::new_v4());
    let extract_path = target_root.join(&extract_folder_name);
    fs::create_dir_all(&extract_path).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        if let Ok(mut file) = archive.by_index(i) {
            let name_raw = file.name_raw().to_vec();
            let name = decode_filename(&name_raw);
            if name.contains("..") { continue; }

            let out_path = extract_path.join(&name);
            if file.is_dir() {
                let _ = fs::create_dir_all(&out_path);
            } else {
                if let Some(p) = out_path.parent() { 
                    let _ = fs::create_dir_all(p); 
                }
                if let Ok(mut outfile) = File::create(&out_path) {
                    if std::io::copy(&mut file, &mut outfile).is_ok() {
                        let new_virtual = format!("{}/{}", virtual_path, name);
                        // 关键：递归处理（支持嵌套压缩包）
                        process_path_recursive(&out_path, &new_virtual, target_root, map, app, task_id);
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
    let unique_name = format!("{}_{}", base_name, Uuid::new_v4().to_string().split('-').next().unwrap_or("tmp"));
    let out_path = target_root.join(&unique_name);
    
    let mut out_file = File::create(&out_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut gz, &mut out_file).map_err(|e| e.to_string())?;
    
    let decompressed_virtual = if virtual_path.ends_with(".gz") {
        virtual_path.trim_end_matches(".gz").to_string()
    } else {
        virtual_path.to_string()
    };
    
    // 关键：递归处理（解压后可能是 tar 或其他压缩格式）
    process_path_recursive(&out_path, &decompressed_virtual, target_root, map, app, task_id);
    Ok(())
}

// --- RAR Processor ---
fn process_rar_archive(
    path: &Path,
    _file_name: &str,
    _virtual_path: &str,
    _target_root: &Path,
    _map: &mut HashMap<String, String>,
    _app: &AppHandle,
    _task_id: &str,
) -> Result<(), String> {
    // RAR 支持需要外部库，这里提供基本框架
    // 实际使用时可能需要系统安装 unrar
    eprintln!("[WARNING] RAR format is not fully supported yet: {}", path.display());
    
    // TODO: 实现 RAR 解压逻辑
    // 可以使用 unrar crate 或调用系统 unrar 命令
    
    Ok(())
}

// --- Deep Recursive Processor (增强版) ---

fn process_path_recursive(
    path: &Path,
    virtual_path: &str,
    target_root: &Path,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) {
    // 错误处理：如果处理失败，不中断整个流程
    if let Err(e) = process_path_recursive_inner(path, virtual_path, target_root, map, app, task_id) {
        eprintln!("[WARNING] Failed to process {}: {}", path.display(), e);
        let _ = app.emit("task-update", TaskProgress {
            task_id: task_id.to_string(), 
            status: "RUNNING".to_string(),
            message: format!("Warning: {}", e), 
            progress: 50,
        });
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
        for entry in WalkDir::new(path).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let entry_name = entry.file_name().to_string_lossy();
            let new_virtual = format!("{}/{}", virtual_path, entry_name);
            process_path_recursive(entry.path(), &new_virtual, target_root, map, app, task_id);
        }
        return Ok(());
    }

    let path_str = path.to_string_lossy();
    let file_name = path.file_name().ok_or("Invalid filename")?.to_string_lossy();
    let lower_path = path_str.to_lowercase();

    let _ = app.emit("task-update", TaskProgress {
        task_id: task_id.to_string(), status: "RUNNING".to_string(),
        message: format!("Processing: {}", file_name), progress: 50,
    });

    // 判断文件类型
    let is_zip = lower_path.ends_with(".zip");
    let is_rar = lower_path.ends_with(".rar");
    let is_tar = lower_path.ends_with(".tar");
    let is_tar_gz = lower_path.ends_with(".tar.gz") || lower_path.ends_with(".tgz");
    let is_plain_gz = lower_path.ends_with(".gz") && !is_tar_gz;

    // --- 处理 ZIP ---
    if is_zip {
        return process_zip_archive(path, &file_name, virtual_path, target_root, map, app, task_id);
    }

    // --- 处理 RAR ---
    if is_rar {
        return process_rar_archive(path, &file_name, virtual_path, target_root, map, app, task_id);
    }

    // --- 处理 TAR / TAR.GZ ---
    if is_tar || is_tar_gz {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        if is_tar_gz {
            let tar = flate2::read::GzDecoder::new(reader);
            let mut archive = tar::Archive::new(tar);
            return process_tar_archive(&mut archive, path, &file_name, target_root, virtual_path, map, app, task_id);
        } else {
            let mut archive = tar::Archive::new(reader);
            return process_tar_archive(&mut archive, path, &file_name, target_root, virtual_path, map, app, task_id);
        }
    }

    // --- 处理纯 GZ ---
    if is_plain_gz {
        return process_gz_file(path, &file_name, virtual_path, target_root, map, app, task_id);
    }

    // --- 普通文件 ---
    map.insert(path.to_string_lossy().to_string(), virtual_path.to_string());
    eprintln!("[DEBUG] regular file indexed: real_path={}, virtual_path={}", 
             path.display(), virtual_path);
    Ok(())
}

// --- Commands ---

#[command]
async fn import_folder(
    app: AppHandle, 
    path: String, 
    #[allow(non_snake_case)]
    workspaceId: String, 
    state: State<'_, AppState>
) -> Result<String, String> {
    let app_handle = app.clone();
    let task_id = Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    let workspace_id_clone = workspaceId.clone();

    eprintln!("[DEBUG] import_folder called: path={}, workspace_id={}, task_id={}", path, workspaceId, task_id);

    // 立即发送初始状态
    let _ = app.emit("task-update", TaskProgress {
        task_id: task_id.clone(),
        status: "RUNNING".to_string(),
        message: "Starting import...".to_string(),
        progress: 0,
    });

    {
        let mut temp_guard = state.temp_dir.lock().unwrap();
        let mut map_guard = state.path_map.lock().unwrap();
        *temp_guard = None; 
        map_guard.clear();
        let new_temp = TempDir::new().map_err(|e| e.to_string())?;
        *temp_guard = Some(new_temp);
    }

    std::thread::spawn(move || {
        eprintln!("[DEBUG] Processing thread started for task: {}", task_id_clone);
        
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let state = app_handle.state::<AppState>();
            let temp_guard = state.temp_dir.lock().unwrap();
            
            if let Some(ref temp_dir) = *temp_guard {
                let target_base = temp_dir.path();
                let source_path = Path::new(&path);
                let root_name = source_path.file_name().unwrap_or_default().to_string_lossy();

                let _ = app_handle.emit("task-update", TaskProgress {
                    task_id: task_id_clone.clone(), status: "RUNNING".to_string(), message: "Scanning...".to_string(), progress: 10
                });

                let mut map_guard = state.path_map.lock().unwrap();
                process_path_recursive(source_path, &root_name, target_base, &mut map_guard, &app_handle, &task_id_clone);
                
                // 输出 path_map 的内容供调试
                eprintln!("[DEBUG] Total files indexed: {}", map_guard.len());
                
                // 关键：保存索引到磁盘
                match save_index(&app_handle, &workspace_id_clone, &map_guard) {
                    Ok(index_path) => {
                        eprintln!("[DEBUG] Index persisted to: {}", index_path.display());
                        // 保存索引路径
                        let mut indices_guard = state.workspace_indices.lock().unwrap();
                        indices_guard.insert(workspace_id_clone.clone(), index_path);
                    },
                    Err(e) => {
                        eprintln!("[WARNING] Failed to save index: {}", e);
                    }
                }
            }
        }));

        if let Err(e) = result {
            eprintln!("[ERROR] Thread panicked: {:?}", e);
            let _ = app_handle.emit("task-update", TaskProgress {
                task_id: task_id_clone, status: "FAILED".to_string(), message: "Crashed".to_string(), progress: 0
            });
            let _ = app_handle.emit("import-error", "Backend process crashed");
        } else {
            eprintln!("[DEBUG] Processing completed successfully for task: {}", task_id_clone);
            let _ = app_handle.emit("task-update", TaskProgress {
                task_id: task_id_clone.clone(), status: "COMPLETED".to_string(), message: "Done".to_string(), progress: 100
            });
            let _ = app_handle.emit("import-complete", task_id_clone); 
        }
    });

    Ok(task_id)
}

#[command]
async fn search_logs(app: AppHandle, query: String, state: State<'_, AppState>) -> Result<(), String> {
    let app_handle = app.clone();
    let state_guard = state.path_map.lock().unwrap().clone();
    
    std::thread::spawn(move || {
        if query.is_empty() { return; }
        let re = match Regex::new(&format!("(?i){}", query)) {
            Ok(r) => r,
            Err(_) => { let _ = app_handle.emit("search-error", "Invalid Regex"); return; }
        };

        let mut batch = Vec::with_capacity(200);
        let mut global_id = 0;
        let mut count = 0;

        for (real_path, virtual_path) in state_guard {
            if let Ok(file) = File::open(&real_path) {
                let reader = BufReader::new(file);
                for (i, line_res) in reader.lines().enumerate() {
                    if let Ok(line) = line_res {
                        if re.is_match(&line) {
                            let (ts, lvl) = parse_metadata(&line);
                            batch.push(LogEntry {
                                id: global_id, timestamp: ts, level: lvl,
                                file: virtual_path.clone(), real_path: real_path.clone(),
                                line: i + 1, content: line, tags: vec![],
                            });
                            global_id += 1;
                            count += 1;

                            if batch.len() >= 500 {
                                let _ = app_handle.emit("search-results", &batch);
                                batch.clear();
                                std::thread::sleep(std::time::Duration::from_millis(2));
                            }
                            if count >= 50000 { break; }
                        }
                    }
                }
            }
            if count >= 50000 { break; }
        }
        if !batch.is_empty() { let _ = app_handle.emit("search-results", &batch); }
        let _ = app_handle.emit("search-complete", count);
    });
    Ok(())
}

// 新增：加载工作区索引
#[command]
async fn load_workspace(
    app: AppHandle, 
    #[allow(non_snake_case)]
    workspaceId: String, 
    state: State<'_, AppState>
) -> Result<(), String> {
    let index_dir = app.path().app_data_dir().map_err(|e| e.to_string())?
        .join("indices");
    let index_path = index_dir.join(format!("{}.idx", workspaceId));
    
    if !index_path.exists() {
        return Err(format!("Index not found for workspace: {}", workspaceId));
    }
    
    let path_map = load_index(&index_path)?;
    
    // 更新内存中的 path_map
    let mut map_guard = state.path_map.lock().unwrap();
    *map_guard = path_map;
    
    // 保存索引路径
    let mut indices_guard = state.workspace_indices.lock().unwrap();
    indices_guard.insert(workspaceId, index_path);
    
    Ok(())
}

#[command]
fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    if !config_dir.exists() { fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?; }
    let path = config_dir.join("config.json");
    fs::write(path, serde_json::to_string_pretty(&config).unwrap()).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = app.path().app_config_dir().map_err(|e| e.to_string())?.join("config.json");
    if path.exists() {
        let c = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Ok(serde_json::from_str(&c).unwrap_or(AppConfig { keyword_groups: serde_json::json!([]), workspaces: serde_json::json!([]) }))
    } else {
        Ok(AppConfig { keyword_groups: serde_json::json!([]), workspaces: serde_json::json!([]) })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { 
            temp_dir: Mutex::new(None), 
            path_map: Mutex::new(HashMap::new()),
            workspace_indices: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            save_config, 
            load_config, 
            search_logs, 
            import_folder,
            load_workspace,  // 新增
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}