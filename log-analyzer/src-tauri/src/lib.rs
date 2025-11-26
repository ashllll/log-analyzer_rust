use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::{command, AppHandle, Emitter, Manager, State};
use serde::{Deserialize, Serialize};
use regex::Regex;
use walkdir::WalkDir;
use tempfile::TempDir;
use std::panic; // 引入 panic 模块

// --- Data Structures ---

#[derive(Serialize, Clone, Debug)]
struct LogEntry {
    id: usize,
    timestamp: String,
    level: String,
    file: String,      // Virtual path (e.g., backup.zip/server.log)
    real_path: String, // Physical path in temp dir
    line: usize,
    content: String,
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    keyword_groups: serde_json::Value,
    workspaces: serde_json::Value,
}

struct AppState {
    // TempDir is automatically deleted when dropped
    temp_dir: Mutex<Option<TempDir>>,
    // Maps physical path -> virtual path
    path_map: Mutex<HashMap<String, String>>,
}

// --- Helper Functions ---

fn decode_filename(bytes: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::UTF_8.decode(bytes);
    // 使用 Unicode 替换字符检测乱码
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
    
    // Simple heuristic for timestamp (first 19 chars)
    let timestamp = if line.len() > 19 { line[0..19].to_string() } else { "".to_string() };
    (timestamp, level.to_string())
}

// --- The "Onion Peeler" (Recursive Extractor) ---

fn extract_recursive(
    file_path: &Path, 
    target_dir: &Path, 
    virtual_prefix: &str,
    map: &mut HashMap<String, String>,
    app: &AppHandle
) {
    let mime = mime_guess::from_path(file_path).first_or_octet_stream();
    
    // 1. Handle ZIP
    if mime.subtype() == "zip" || file_path.extension().map_or(false, |e| e == "zip") {
        let _ = app.emit("task-progress", format!("Unzipping: {}", virtual_prefix));
        if let Ok(file) = File::open(file_path) {
            let reader = BufReader::new(file);
            if let Ok(mut archive) = zip::ZipArchive::new(reader) {
                for i in 0..archive.len() {
                    if let Ok(mut file) = archive.by_index(i) {
                        let name_raw = file.name_raw().to_vec();
                        let name = decode_filename(&name_raw);
                        if name.contains("..") { continue; } // Security check

                        let out_path = target_dir.join(&name);
                        let new_virtual = format!("{}/{}", virtual_prefix, name);

                        if file.is_dir() {
                            fs::create_dir_all(&out_path).ok();
                        } else {
                            if let Some(p) = out_path.parent() { fs::create_dir_all(p).ok(); }
                            if let Ok(mut outfile) = File::create(&out_path) {
                                std::io::copy(&mut file, &mut outfile).ok();
                                // Recurse!
                                extract_recursive(&out_path, target_dir, &new_virtual, map, app);
                            }
                        }
                    }
                }
            }
        }
        return;
    }

    // 2. Handle TAR / TAR.GZ
    let path_str = file_path.to_string_lossy();
    if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") || path_str.ends_with(".tar") {
        let _ = app.emit("task-progress", format!("Untarring: {}", virtual_prefix));
        if let Ok(file) = File::open(file_path) {
            let reader = BufReader::new(file);
            let tar = flate2::read::GzDecoder::new(reader);
            let mut archive = tar::Archive::new(tar);
            
            if let Ok(entries) = archive.entries() {
                for entry in entries {
                    if let Ok(mut file) = entry {
                        let path = file.path().unwrap().into_owned();
                        let name = path.to_string_lossy();
                        let out_path = target_dir.join(&*name);
                        let new_virtual = format!("{}/{}", virtual_prefix, name);
                        
                        if let Some(p) = out_path.parent() { fs::create_dir_all(p).ok(); }
                        file.unpack(&out_path).ok();
                        extract_recursive(&out_path, target_dir, &new_virtual, map, app);
                    }
                }
            }
        }
        return;
    }

    // 3. Regular File -> Register mapping
    map.insert(file_path.to_string_lossy().to_string(), virtual_prefix.to_string());
}

// --- Commands ---

#[command]
async fn import_folder(app: AppHandle, path: String, state: State<'_, AppState>) -> Result<String, String> {
    let app_handle = app.clone();
    
    // Reset State
    {
        let mut temp_guard = state.temp_dir.lock().unwrap();
        let mut map_guard = state.path_map.lock().unwrap();
        *temp_guard = None; 
        map_guard.clear();
        
        let new_temp = TempDir::new().map_err(|e| e.to_string())?;
        *temp_guard = Some(new_temp);
    }

    std::thread::spawn(move || {
        // 修复点：使用 AssertUnwindSafe 包装闭包，解决 AppHandle 不是 UnwindSafe 的问题
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let state = app_handle.state::<AppState>();
            let temp_guard = state.temp_dir.lock().unwrap();
            
            if let Some(ref temp_dir) = *temp_guard {
                let target_base = temp_dir.path();
                let source_path = Path::new(&path);
                
                for entry in WalkDir::new(source_path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        let file_path = entry.path();
                        let relative = file_path.strip_prefix(source_path).unwrap_or(file_path);
                        let virtual_prefix = relative.to_string_lossy().to_string();
                        
                        let mut map_guard = state.path_map.lock().unwrap();
                        extract_recursive(file_path, target_base, &virtual_prefix, &mut map_guard, &app_handle);
                    }
                }
            }
        }));

        if let Err(_) = result {
            let _ = app_handle.emit("import-error", "Import crashed unexpectedly");
        } else {
            let _ = app_handle.emit("import-complete", "Ready");
        }
    });

    Ok("Started".to_string())
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
                                id: global_id,
                                timestamp: ts,
                                level: lvl,
                                file: virtual_path.clone(),
                                real_path: real_path.clone(),
                                line: i + 1,
                                content: line,
                                tags: vec![],
                            });
                            global_id += 1;
                            count += 1;

                            if batch.len() >= 200 {
                                let _ = app_handle.emit("search-results", &batch);
                                batch.clear();
                                std::thread::sleep(std::time::Duration::from_millis(5)); // Yield
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
        .manage(AppState { temp_dir: Mutex::new(None), path_map: Mutex::new(HashMap::new()) })
        .invoke_handler(tauri::generate_handler![save_config, load_config, search_logs, import_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}