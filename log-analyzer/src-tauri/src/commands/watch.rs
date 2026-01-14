//! 文件监听命令实现

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

    let watcher_state = WatcherState {
        workspace_id: workspaceId.clone(),
        watched_path: watch_path.clone(),
        file_offsets: HashMap::new(),
        is_active: true,
    };

    {
        let mut watchers = state.watchers.lock();
        watchers.insert(workspaceId.clone(), watcher_state);
    }

    let app_handle = app.clone();
    let workspace_id_clone = workspaceId.clone();
    let watch_path_clone = watch_path.clone();
    let watchers_arc = Arc::clone(&state.watchers);

    thread::spawn(move || {
        // 使用 crossbeam 的无界通道以获得更高的吞吐量
        let (tx, rx) = crossbeam::channel::unbounded::<Result<Event, notify::Error>>();

        let mut watcher = match recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                error!(error = %e, "Failed to create file watcher");
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_path_clone, RecursiveMode::Recursive) {
            error!(error = %e, "Failed to start watching path");
            return;
        }

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
                        let file_path_str = path.to_string_lossy().to_string();

                        let file_change = FileChangeEvent {
                            event_type: event_type.to_string(),
                            file_path: file_path_str.clone(),
                            workspace_id: workspace_id_clone.clone(),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        let _ = app_handle.emit("file-changed", file_change);

                        if event_type == "modified" && path.is_file() {
                            let (offset, watcher_workspace_id, watcher_watched_path) = {
                                let mut watchers = watchers_arc.lock();
                                if let Some(watcher) = watchers.get_mut(&workspace_id_clone) {
                                    let offset =
                                        *watcher.file_offsets.get(&file_path_str).unwrap_or(&0);
                                    let workspace_id = watcher.workspace_id.clone();
                                    let watched_path = watcher.watched_path.clone();
                                    (offset, workspace_id, watched_path)
                                } else {
                                    continue;
                                }
                            };

                            match read_file_from_offset(&path, offset) {
                                Ok((new_lines, new_offset)) => {
                                    if !new_lines.is_empty() {
                                        let start_line_number = if offset == 0 {
                                            1
                                        } else {
                                            (offset / 100) as usize + 1
                                        };

                                        let virtual_path = path
                                            .strip_prefix(&watcher_watched_path)
                                            .ok()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or(path.to_str().unwrap_or("unknown"));

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

    Ok(())
}

#[command]
pub async fn stop_watch(
    #[allow(non_snake_case)] workspaceId: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut watchers = state.watchers.lock();

    if let Some(watcher_state) = watchers.get_mut(&workspaceId) {
        watcher_state.is_active = false;
    } else {
        return Err("No active watcher found for this workspace".to_string());
    }

    watchers.remove(&workspaceId);
    Ok(())
}
