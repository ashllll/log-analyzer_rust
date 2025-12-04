//! 日志分析器 - Rust 后端
//!
//! 提供高性能的日志分析功能，包括：
//! - 多格式压缩包递归解压
//! - 并行全文搜索
//! - 结构化查询系统
//! - 索引持久化与增量更新
//! - 实时文件监听

use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Emitter, Manager, State};
use uuid::Uuid;
use walkdir::WalkDir;

// 模块声明
mod archive;
mod commands;
mod models;
mod services;
mod utils;

// 从模块导入类型
use models::{
    AppConfig, AppState, FileChangeEvent, FileMetadata, LogEntry, PerformanceMetrics,
    SearchCacheKey, SearchFilters, SearchQuery, TaskProgress, WatcherState,
};

// 从services模块导入函数和类型
use services::{
    append_to_workspace_index, get_file_metadata, load_index, parse_log_lines, parse_metadata,
    read_file_from_offset, save_index, ExecutionPlan, QueryExecutor,
};

// 从archive模块导入函数
use archive::process_path_recursive_with_metadata;

// 从utils模块导入函数
use utils::{
    canonicalize_path, normalize_path_separator, validate_path_param, validate_workspace_id,
};

// 从commands模块导入新增的delete_workspace命令
use commands::delete_workspace;

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

    // 创建持久化的解压目录（使用应用数据目录而非临时目录）
    let extracted_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("extracted")
        .join(&workspaceId);

    // 确保目录存在
    fs::create_dir_all(&extracted_dir)
        .map_err(|e| format!("Failed to create extracted dir: {}", e))?;

    eprintln!("[DEBUG] Using extracted dir: {}", extracted_dir.display());

    // 立即发送初始状态
    eprintln!(
        "[DEBUG] Sending initial task-update event: task_id={}, workspace_id={}",
        task_id, workspaceId
    );
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
            workspace_id: Some(workspaceId.clone()), // 添加 workspace_id
        },
    );

    // 清理内存中的旧数据
    {
        let mut map_guard = state
            .path_map
            .lock()
            .map_err(|e| format!("Failed to acquire path_map lock: {}", e))?;
        let mut metadata_guard = state
            .file_metadata
            .lock()
            .map_err(|e| format!("Failed to acquire metadata lock: {}", e))?;

        map_guard.clear();
        metadata_guard.clear();
    }

    std::thread::spawn(move || {
        eprintln!(
            "[DEBUG] Processing thread started for task: {}",
            task_id_clone
        );

        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let state = app_handle.state::<AppState>();
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
                    workspace_id: Some(workspace_id_clone.clone()),
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

            // 使用持久化的解压目录
            process_path_recursive_with_metadata(
                source_path,
                &root_name,
                &extracted_dir,
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

            // 注意：不再清理解压目录，保持文件可用以便搜索
            eprintln!(
                "[DEBUG] Extracted files kept in: {}",
                extracted_dir.display()
            );

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
                    workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
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

            eprintln!(
                "[DEBUG] Sending COMPLETED task-update event: task_id={}, workspace_id={}",
                task_id_clone, workspace_id_clone
            );
            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Import".to_string(),
                    target: file_name,
                    status: "COMPLETED".to_string(),
                    message: "Done".to_string(),
                    progress: 100,
                    workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
                },
            );
            eprintln!(
                "[DEBUG] Sending import-complete event: task_id={}",
                task_id_clone
            );
            let _ = app_handle.emit("import-complete", task_id_clone);
        }
    });

    Ok(task_id)
}

// 单文件搜索函数（用于并行处理）
#[allow(dead_code)] // 保留供将来使用或测试
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
                        match_details: None, // 旧版本不包含匹配详情
                    });
                }
            }
        }
    }

    results
}

/// 单文件搜索（带匹配详情） - 使用 query_executor
fn search_single_file_with_details(
    real_path: &str,
    virtual_path: &str,
    executor: &QueryExecutor,
    plan: &ExecutionPlan,
    global_offset: usize,
) -> Vec<LogEntry> {
    let mut results = Vec::new();

    if let Ok(file) = File::open(real_path) {
        let reader = BufReader::with_capacity(8192, file);

        for (i, line_res) in reader.lines().enumerate() {
            if let Ok(line) = line_res {
                // 使用 query_executor 匹配
                if executor.matches_line(plan, &line) {
                    let (ts, lvl) = parse_metadata(&line);

                    // 获取匹配详情
                    let match_details = executor.match_with_details(plan, &line);

                    results.push(LogEntry {
                        id: global_offset + i,
                        timestamp: ts,
                        level: lvl,
                        file: virtual_path.to_string(),
                        real_path: real_path.to_string(),
                        line: i + 1,
                        content: line,
                        tags: vec![],
                        match_details,
                    });
                }
            }
        }
    }

    results
}

#[command]
async fn search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] workspaceId: Option<String>, // 工作区ID，用于缓存隔离
    max_results: Option<usize>,                           // 可配置限制
    filters: Option<SearchFilters>,                       // 高级过滤器
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

        // 将简单查询字符串转换为结构化查询
        // | 仅作为分隔符，多个关键词用 OR 逻辑组合
        let raw_terms: Vec<String> = query
            .split('|')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();

        if raw_terms.is_empty() {
            let _ = app_handle.emit("search-error", "Search query is empty after processing");
            return;
        }

        // 创建结构化查询对象
        use models::search::*;
        let search_terms: Vec<SearchTerm> = raw_terms
            .iter()
            .enumerate()
            .map(|(i, term)| SearchTerm {
                id: format!("term_{}", i),
                value: term.clone(),
                operator: QueryOperator::Or, // OR 逻辑
                source: TermSource::User,
                preset_group_id: None,
                is_regex: false,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            })
            .collect();

        let structured_query = SearchQuery {
            id: "search_logs_query".to_string(),
            terms: search_terms,
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        // 创建 QueryExecutor 并执行查询
        let mut executor = services::QueryExecutor::new(100);
        let plan = match executor.execute(&structured_query) {
            Ok(p) => p,
            Err(e) => {
                let _ = app_handle.emit("search-error", format!("Query execution error: {}", e));
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

        // 并行搜索，使用带详情的搜索函数
        let mut all_results: Vec<LogEntry> = files
            .par_iter()
            .enumerate()
            .flat_map(|(idx, (real_path, virtual_path))| {
                search_single_file_with_details(
                    real_path,
                    virtual_path,
                    &executor,
                    &plan,
                    idx * 10000,
                )
            })
            .collect();

        eprintln!(
            "[DEBUG] Found {} results before filtering",
            all_results.len()
        );

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

            eprintln!("[DEBUG] {} results after filtering", all_results.len());
        }

        // 截取结果（Rayon 不支持 .take()）
        let results_truncated = all_results.len() > max_results;
        if results_truncated {
            all_results.truncate(max_results);
            eprintln!("[WARN] Results truncated to {} (max limit)", max_results);
        }

        eprintln!("[DEBUG] Final result count: {}", all_results.len());

        // 缓存结果（仅当结果未被截断时缓存）
        if !results_truncated && !all_results.is_empty() {
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
            workspace_id: Some(workspaceId.clone()), // 添加 workspace_id
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
        eprintln!("[DEBUG] Refresh thread started for task: {}", task_id_clone);

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
                    target: file_name.clone(), // 使用文件名而不是完整路径
                    status: "RUNNING".to_string(),
                    message: "Scanning file system...".to_string(),
                    progress: 20,
                    workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
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
                    target: file_name.clone(), // 使用文件名
                    status: "RUNNING".to_string(),
                    message: "Analyzing changes...".to_string(),
                    progress: 40,
                    workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
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
                eprintln!("[DEBUG] No changes detected, no index update needed");
                // 直接跳到最后，不做任何处理，但不 return，让它继续执行到 COMPLETED 事件
            } else {
                let _ = app_handle.emit(
                    "task-update",
                    TaskProgress {
                        task_id: task_id_clone.clone(),
                        task_type: "Refresh".to_string(),
                        target: file_name.clone(), // 使用文件名
                        status: "RUNNING".to_string(),
                        message: format!("Processing {} changes...", total_changes),
                        progress: 60,
                        workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
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
                            let virtual_path = format!(
                                "{}/{}",
                                root_name,
                                relative.to_string_lossy().replace('\\', "/")
                            );
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
                            let virtual_path = format!(
                                "{}/{}",
                                root_name,
                                relative.to_string_lossy().replace('\\', "/")
                            );
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

                    eprintln!(
                        "[DEBUG] Updated index: {} total files",
                        existing_path_map.len()
                    );
                }

                let _ = app_handle.emit(
                    "task-update",
                    TaskProgress {
                        task_id: task_id_clone.clone(),
                        task_type: "Refresh".to_string(),
                        target: file_name.clone(), // 使用文件名
                        status: "RUNNING".to_string(),
                        message: "Saving index...".to_string(),
                        progress: 80,
                        workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
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
                    target: file_name.clone(), // 使用文件名
                    status: "FAILED".to_string(),
                    message: "Refresh failed".to_string(),
                    progress: 0,
                    workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
                },
            );
        } else {
            eprintln!("[DEBUG] Refresh completed for task: {}", task_id_clone);
            eprintln!(
                "[DEBUG] Sending COMPLETED event with workspace_id: {:?}",
                workspace_id_clone
            );
            let _ = app_handle.emit(
                "task-update",
                TaskProgress {
                    task_id: task_id_clone.clone(),
                    task_type: "Refresh".to_string(),
                    target: file_name, // 使用文件名
                    status: "COMPLETED".to_string(),
                    message: "Refresh complete".to_string(),
                    progress: 100,
                    workspace_id: Some(workspace_id_clone.clone()), // 添加 workspace_id
                },
            );
            eprintln!("[DEBUG] Sending import-complete event");
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
    writeln!(writer, "ID,Timestamp,Level,File,Line,Content")
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
            entry.id, entry.timestamp, entry.level, file_escaped, entry.line, content_escaped
        )
        .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    writer
        .flush()
        .map_err(|e| format!("Failed to flush CSV file: {}", e))?;

    eprintln!(
        "[DEBUG] CSV export completed: {} rows written",
        results.len()
    );
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
///
/// 由于 unrar 已经内置在应用中，此命令始终返回 available: true
#[command]
async fn check_rar_support() -> Result<serde_json::Value, String> {
    // unrar 已经打包在应用中，始终可用
    Ok(serde_json::json!({
        "available": true,
        "install_guide": null,
        "bundled": true,
    }))
}

// ============================================================================
// 结构化查询 API
// ============================================================================

/**
 * 执行结构化查询
 */
#[command]
fn execute_structured_query(query: SearchQuery, logs: Vec<String>) -> Result<Vec<String>, String> {
    let mut executor = QueryExecutor::new(1000);

    let plan = executor.execute(&query).map_err(|e| e.to_string())?;

    let filtered: Vec<String> = logs
        .iter()
        .filter(|line| executor.matches_line(&plan, line))
        .cloned()
        .collect();

    Ok(filtered)
}

/**
 * 验证查询有效性
 */
#[command]
fn validate_query(query: SearchQuery) -> Result<bool, String> {
    let mut executor = QueryExecutor::new(1000);

    executor
        .execute(&query)
        .map(|_| true)
        .map_err(|e| e.to_string())
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
    use notify::{recommended_watcher, Event, RecursiveMode, Watcher};
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
        let watchers = state
            .watchers
            .lock()
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
        let mut watchers = state
            .watchers
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        watchers.insert(workspaceId.clone(), watcher_state);
    }

    // 在后台线程中启动监听器
    let app_handle = app.clone();
    let workspace_id_clone = workspaceId.clone();
    let watch_path_clone = watch_path.clone();
    let watchers_arc = Arc::clone(&state.watchers);

    std::thread::spawn(move || {
        eprintln!(
            "[DEBUG] File watcher thread started for workspace: {}",
            workspace_id_clone
        );

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

        eprintln!(
            "[DEBUG] Successfully started watching: {}",
            watch_path_clone.display()
        );

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
                                        let offset =
                                            *watcher.file_offsets.get(&file_path_str).unwrap_or(&0);
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
                                        if let Some(watcher) = watchers.get_mut(&workspace_id_clone)
                                        {
                                            watcher
                                                .file_offsets
                                                .insert(file_path_str.clone(), new_offset);
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
                                    eprintln!("[WARNING] Failed to read file incrementally: {}", e);
                                }
                            }
                        }
                    }

                    // 检查是否还在活跃
                    let is_active = {
                        if let Ok(watchers) = watchers_arc.lock() {
                            watchers
                                .get(&workspace_id_clone)
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
                }
                Err(e) => {
                    eprintln!("[ERROR] Watch error: {}", e);
                }
            }
        }

        eprintln!(
            "[DEBUG] File watcher thread terminated for workspace: {}",
            workspace_id_clone
        );
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
    let mut watchers = state
        .watchers
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    if let Some(watcher_state) = watchers.get_mut(&workspaceId) {
        watcher_state.is_active = false;
        eprintln!("[DEBUG] Watcher deactivated for workspace: {}", workspaceId);
    } else {
        return Err("No active watcher found for this workspace".to_string());
    }

    // 从状态中移除
    watchers.remove(&workspaceId);

    eprintln!(
        "[DEBUG] Watcher removed from state for workspace: {}",
        workspaceId
    );
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
        .plugin(tauri_plugin_shell::init())
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
            execute_structured_query,
            validate_query,
            delete_workspace,
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
    use crate::utils::encoding::decode_filename;
    use crate::utils::path::{remove_readonly, safe_path_join};
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

    // 注意：get_bundled_unrar_path 需要 AppHandle，无法在单元测试中测试
    // 该功能通过集成测试（实际运行应用并导入 RAR 文件）进行验证
}
