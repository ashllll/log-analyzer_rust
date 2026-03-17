//! 文件监听命令实现
//!
//! # 前后端集成规范
//!
//! 为保持与 JavaScript camelCase 惯例一致，Tauri 命令参数使用 camelCase 命名。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::error;

use crate::models::{AppState, FileChangeEvent};
use crate::services::file_watcher::WatcherState;
use crate::services::{append_to_workspace_index, parse_log_lines, read_file_from_offset};
use crate::utils::{validate_path_param, validate_workspace_id};

#[command]
pub async fn start_watch(
    app: AppHandle,
    #[allow(non_snake_case)] workspaceId: String,
    path: String,
    #[allow(non_snake_case)] _autoSearch: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_workspace_id(&workspaceId)?;
    validate_path_param(&path, "path")?;

    let watch_path = PathBuf::from(&path);
    if !watch_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    {
        let watchers = state.watchers.lock();
        if watchers.contains_key(&workspaceId) {
            return Err("Workspace is already being watched".to_string());
        }
    }

    // 创建通信通道
    let (tx, rx) = crossbeam::channel::unbounded::<Result<Event, notify::Error>>();

    // 创建监听器
    let mut watcher = match recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => {
            return Err(format!("Failed to create file watcher: {}", e));
        }
    };

    // 开始监听
    if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
        return Err(format!("Failed to start watching path: {}", e));
    }

    let watcher_state = WatcherState {
        workspace_id: workspaceId.clone(),
        watched_path: watch_path.clone(),
        file_offsets: HashMap::new(),
        line_counts: HashMap::new(),
        is_active: true,
        thread_handle: Arc::new(parking_lot::Mutex::new(None)),
        watcher: Arc::new(parking_lot::Mutex::new(Some(watcher))),
    };

    let thread_handle_arc = Arc::clone(&watcher_state.thread_handle);

    {
        let mut watchers = state.watchers.lock();
        watchers.insert(workspaceId.clone(), watcher_state);
    }

    let app_handle = app.clone();
    let workspace_id_clone = workspaceId.clone();
    let watchers_arc = Arc::clone(&state.watchers);

    let handle = thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                    let event_type = match event.kind {
                        EventKind::Create(_) => "created",
                        EventKind::Modify(_) => "modified",
                        EventKind::Remove(_) => "deleted",
                        _ => continue,
                    };

                    for path in event.paths {
                        let file_path_str = match path.to_str() {
                            Some(s) => s.to_string(),
                            None => {
                                tracing::warn!(
                                    path = ?path,
                                    "跳过包含非 UTF-8 字符的路径"
                                );
                                continue;
                            }
                        };

                        let file_change = FileChangeEvent {
                            event_type: event_type.to_string(),
                            file_path: file_path_str.clone(),
                            workspace_id: workspace_id_clone.clone(),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        let _ = app_handle.emit("file-changed", file_change);

                        if event_type == "modified" && path.is_file() {
                            let (
                                offset,
                                watcher_workspace_id,
                                watcher_watched_path,
                                start_line_number,
                            ) = {
                                let mut watchers = watchers_arc.lock();
                                if let Some(watcher) = watchers.get_mut(&workspace_id_clone) {
                                    let offset =
                                        *watcher.file_offsets.get(&file_path_str).unwrap_or(&0);
                                    let workspace_id = watcher.workspace_id.clone();
                                    let watched_path = watcher.watched_path.clone();
                                    let start_line = watcher
                                        .line_counts
                                        .get(&file_path_str)
                                        .copied()
                                        .unwrap_or(0)
                                        + 1;
                                    (offset, workspace_id, watched_path, start_line)
                                } else {
                                    continue;
                                }
                            };

                            match read_file_from_offset(&path, offset) {
                                Ok((new_lines, new_offset)) => {
                                    let new_line_count = new_lines.len();
                                    if !new_lines.is_empty() {
                                        // 使用 to_string_lossy 避免静默丢失非 UTF-8 路径字节（B-L5）
                                        let virtual_path_buf = path
                                            .strip_prefix(&watcher_watched_path)
                                            .unwrap_or(&path);
                                        let virtual_path_cow = virtual_path_buf.to_string_lossy();
                                        if virtual_path_cow.contains('\u{FFFD}') {
                                            tracing::warn!(
                                                path = ?path,
                                                "virtual_path 包含非 UTF-8 字节，替换字符 U+FFFD 已插入"
                                            );
                                        }
                                        let virtual_path = virtual_path_cow.as_ref();

                                        let new_entries = parse_log_lines(
                                            &new_lines,
                                            virtual_path,
                                            &file_path_str,
                                            0,
                                            start_line_number,
                                        );

                                        let state = app_handle.state::<AppState>();
                                        let _ = append_to_workspace_index(
                                            &watcher_workspace_id,
                                            &new_entries,
                                            &app_handle,
                                            &state,
                                        );
                                    }

                                    {
                                        let mut watchers = watchers_arc.lock();
                                        if let Some(watcher) = watchers.get_mut(&workspace_id_clone)
                                        {
                                            watcher
                                                .file_offsets
                                                .insert(file_path_str.clone(), new_offset);
                                            if new_line_count > 0 {
                                                watcher.line_counts.insert(
                                                    file_path_str.clone(),
                                                    start_line_number - 1 + new_line_count,
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        error = %e,
                                        file = %file_path_str,
                                        "Failed to read file incrementally"
                                    );
                                }
                            }
                        }
                    }

                    let is_active = {
                        let watchers = watchers_arc.lock();
                        watchers
                            .get(&workspace_id_clone)
                            .map(|w| w.is_active)
                            .unwrap_or(false)
                    };

                    if !is_active {
                        break;
                    }
                }
                Err(e) => {
                    error!(error = %e, "Watch error");
                }
            }
        }
    });

    // 保存线程句柄以便后续 join（parking_lot::Mutex 不会 poison，直接 lock）
    *thread_handle_arc.lock() = Some(handle);

    Ok(())
}

#[command]
pub async fn stop_watch(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut watchers = state.watchers.lock();

    let (thread_handle, watcher) = if let Some(watcher_state) = watchers.get_mut(&workspaceId) {
        watcher_state.is_active = false;
        // 获取句柄和监听器以便清理
        let h = watcher_state.thread_handle.lock().take();
        let w = watcher_state.watcher.lock().take();
        (h, w)
    } else {
        return Err("No active watcher found for this workspace".to_string());
    };

    // 从 map 中移除
    watchers.remove(&workspaceId);

    // 释放锁以便线程可以完成最后的循环并退出（如果它正在检查 is_active）
    drop(watchers);

    // 显式释放归档句柄，这会关闭 tx 通道，从而使 rx.iter() 终止
    drop(watcher);

    // 在锁外进行 join，避免死锁并确保线程资源回收
    if let Some(handle) = thread_handle {
        if let Err(e) = handle.join() {
            error!("Failed to join watcher thread: {:?}", e);
        }
    }

    Ok(())
}
