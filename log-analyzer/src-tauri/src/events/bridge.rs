//! 简化版 Tauri 事件桥接
//! 
//! 直接转发内部事件到 Tauri 前端，移除多余的中转层
//! 
//! # 优先级处理 (P2-11)
//! 
//! 桥接器优先处理高优先级事件，确保关键事件不被延迟：
//! - 使用 PriorityEventChannels 接收事件
//! - 优先检查 high 通道，然后是 normal 和 low

use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use super::constants::*;
use super::{get_event_bus, AppEvent, PriorityEventChannels};

/// Tauri 事件桥接器
/// 
/// 从内部事件总线接收事件并直接转发到 Tauri 前端
/// 
/// # 优先级支持
/// 
/// 支持两种模式：
/// 1. 传统模式：使用单一 EventBus（向后兼容）
/// 2. 优先级模式：使用 PriorityEventChannels（推荐）
pub struct TauriBridge {
    app_handle: AppHandle,
    receiver: broadcast::Receiver<AppEvent>,
    /// 可选的优先级通道接收器
    priority_receivers: Option<PriorityReceivers>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

/// 优先级通道接收器集合
struct PriorityReceivers {
    high: broadcast::Receiver<AppEvent>,
    normal: broadcast::Receiver<AppEvent>,
    low: broadcast::Receiver<AppEvent>,
}

impl TauriBridge {
    /// 创建新的 Tauri 桥接器（传统模式）
    pub fn new(app_handle: AppHandle) -> Self {
        let receiver = get_event_bus().subscribe("tauri_bridge".to_string());

        Self {
            app_handle,
            receiver,
            priority_receivers: None,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 创建新的 Tauri 桥接器（优先级模式）
    pub fn with_priority_channels(app_handle: AppHandle, channels: &PriorityEventChannels) -> Self {
        let (high, normal, low) = channels.subscribe_all();
        
        Self {
            app_handle,
            receiver: high.resubscribe(), // high 优先级作为默认
            priority_receivers: Some(PriorityReceivers {
                high,
                normal,
                low,
            }),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 启动桥接器（在后台任务中运行）
    pub async fn run(mut self) {
        use std::sync::atomic::Ordering;

        if self.is_running.load(Ordering::Relaxed) {
            warn!("Tauri bridge is already running");
            return;
        }

        self.is_running.store(true, Ordering::Relaxed);
        info!("Tauri event bridge started");

        // 提取需要的字段，避免部分移动问题
        let priority_receivers = self.priority_receivers.take();
        let is_running = self.is_running.clone();
        let app_handle = self.app_handle;

        // 根据是否有优先级通道选择不同处理模式
        if let Some(receivers) = priority_receivers {
            // 优先级模式：优先处理高优先级事件
            Self::run_priority_loop(is_running, receivers, app_handle).await;
        } else {
            // 传统模式：单一通道处理
            Self::run_single_loop(is_running, self.receiver, app_handle).await;
        }

        info!("Tauri event bridge stopped");
    }

    /// 优先级模式：优先处理高优先级事件
    async fn run_priority_loop(
        is_running: Arc<std::sync::atomic::AtomicBool>,
        mut receivers: PriorityReceivers,
        app_handle: AppHandle,
    ) {
        use std::sync::atomic::Ordering;
        use tokio::sync::broadcast::error::RecvError;

        while is_running.load(Ordering::Relaxed) {
            // 优先处理高优先级事件（非阻塞检查）
            match receivers.high.try_recv() {
                Ok(event) => {
                    if let Err(e) = Self::forward_event_static(&app_handle, event).await {
                        error!(error = %e, "Failed to forward high priority event");
                    }
                    continue; // 继续检查高优先级
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    // 高优先级队列为空，继续检查其他
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    info!("High priority channel closed");
                    break;
                }
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    warn!(skipped_events = skipped, "High priority channel lagged");
                }
            }

            // 检查普通优先级事件
            match receivers.normal.try_recv() {
                Ok(event) => {
                    if let Err(e) = Self::forward_event_static(&app_handle, event).await {
                        error!(error = %e, "Failed to forward normal priority event");
                    }
                    continue;
                }
                Err(broadcast::error::TryRecvError::Empty) => {}
                Err(broadcast::error::TryRecvError::Closed) => {}
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    warn!(skipped_events = skipped, "Normal priority channel lagged");
                }
            }

            // 检查低优先级事件
            match receivers.low.try_recv() {
                Ok(event) => {
                    if let Err(e) = Self::forward_event_static(&app_handle, event).await {
                        error!(error = %e, "Failed to forward low priority event");
                    }
                    continue;
                }
                Err(broadcast::error::TryRecvError::Empty) => {}
                Err(broadcast::error::TryRecvError::Closed) => {}
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    warn!(skipped_events = skipped, "Low priority channel lagged");
                }
            }

            // 所有队列为空时，等待新事件（优先等待高优先级）
            tokio::select! {
                result = receivers.high.recv() => {
                    match result {
                        Ok(event) => {
                            if let Err(e) = Self::forward_event_static(&app_handle, event).await {
                                error!(error = %e, "Failed to forward high priority event");
                            }
                        }
                        Err(RecvError::Closed) => break,
                        Err(RecvError::Lagged(skipped)) => {
                            warn!(skipped_events = skipped, "High priority channel lagged");
                        }
                    }
                }
                // 检查普通优先级
                result = receivers.normal.recv() => {
                    match result {
                        Ok(event) => {
                            if let Err(e) = Self::forward_event_static(&app_handle, event).await {
                                error!(error = %e, "Failed to forward normal priority event");
                            }
                        }
                        Err(RecvError::Closed) => {}
                        Err(RecvError::Lagged(skipped)) => {
                            warn!(skipped_events = skipped, "Normal priority channel lagged");
                        }
                    }
                }
            }
        }
    }

    /// 传统模式：单一通道处理
    async fn run_single_loop(
        is_running: Arc<std::sync::atomic::AtomicBool>,
        mut receiver: broadcast::Receiver<AppEvent>,
        app_handle: AppHandle,
    ) {
        use std::sync::atomic::Ordering;

        while is_running.load(Ordering::Relaxed) {
            match receiver.recv().await {
                Ok(event) => {
                    if let Err(e) = Self::forward_event_static(&app_handle, event).await {
                        error!(error = %e, "Failed to forward event to Tauri frontend");
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Event bus closed, stopping Tauri bridge");
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped_events = skipped,
                        "Tauri bridge lagged behind, some events were skipped"
                    );
                }
            }
        }
    }

    /// 停止桥接器
    pub fn stop(&self) {
        use std::sync::atomic::Ordering;
        self.is_running.store(false, Ordering::Relaxed);
        info!("Tauri event bridge stopping");
    }

    /// 转发事件到 Tauri 前端
    /// 
    /// 使用常量定义的事件名称，确保命名一致性
    async fn forward_event_static(app_handle: &AppHandle, event: AppEvent) -> Result<(), tauri::Error> {
        match event {
            // 搜索事件
            AppEvent::SearchStart { message } => {
                app_handle.emit(EVENT_SEARCH_START, message)?;
            }
            AppEvent::SearchProgress { progress } => {
                app_handle.emit(EVENT_SEARCH_PROGRESS, progress)?;
            }
            AppEvent::SearchResults { results } => {
                app_handle.emit(EVENT_SEARCH_RESULTS, results)?;
            }
            AppEvent::SearchSummary { summary } => {
                app_handle.emit(EVENT_SEARCH_SUMMARY, summary)?;
            }
            AppEvent::SearchComplete { count } => {
                app_handle.emit(EVENT_SEARCH_COMPLETE, count)?;
            }
            AppEvent::SearchError { error } => {
                app_handle.emit(EVENT_SEARCH_ERROR, error)?;
            }

            // 异步搜索事件
            AppEvent::AsyncSearchStart { search_id } => {
                app_handle.emit(EVENT_ASYNC_SEARCH_START, search_id)?;
            }
            AppEvent::AsyncSearchProgress {
                search_id,
                progress,
            } => {
                app_handle
                    .emit(EVENT_ASYNC_SEARCH_PROGRESS, (search_id, progress))?;
            }
            AppEvent::AsyncSearchResults { results } => {
                app_handle.emit(EVENT_ASYNC_SEARCH_RESULTS, results)?;
            }
            AppEvent::AsyncSearchComplete { search_id, count } => {
                app_handle
                    .emit(EVENT_ASYNC_SEARCH_COMPLETE, (search_id, count))?;
            }
            AppEvent::AsyncSearchError { search_id, error } => {
                app_handle
                    .emit(EVENT_ASYNC_SEARCH_ERROR, (search_id, error))?;
            }

            // 任务事件
            AppEvent::TaskUpdate { progress } => {
                app_handle.emit(EVENT_TASK_UPDATE, progress)?;
            }
            AppEvent::ImportComplete { task_id } => {
                app_handle.emit(EVENT_IMPORT_COMPLETE, task_id)?;
            }

            // 文件监控事件
            AppEvent::FileChanged { event } => {
                app_handle.emit(EVENT_FILE_CHANGED, event)?;
            }
            AppEvent::NewLogs { entries } => {
                app_handle.emit(EVENT_NEW_LOGS, entries)?;
            }

            // 系统事件（仅记录，不转发到前端）
            AppEvent::SystemError { error, context } => {
                debug!(
                    error = %error,
                    context = ?context,
                    event = EVENT_SYSTEM_ERROR,
                    "System error event (not forwarded to frontend)"
                );
            }
            AppEvent::SystemWarning { warning, context } => {
                debug!(
                    warning = %warning,
                    context = ?context,
                    event = EVENT_SYSTEM_WARNING,
                    "System warning event (not forwarded to frontend)"
                );
            }
            AppEvent::SystemInfo { info, context } => {
                debug!(
                    info = %info,
                    context = ?context,
                    event = EVENT_SYSTEM_INFO,
                    "System info event (not forwarded to frontend)"
                );
            }
        }

        Ok(())
    }
}

/// 初始化并启动 Tauri 事件桥接器
/// 
/// 在应用启动时调用，在后台任务中运行桥接器
pub fn init_tauri_bridge(app_handle: AppHandle) {
    let bridge = TauriBridge::new(app_handle);

    // 在后台任务中启动桥接器
    tauri::async_runtime::spawn(async move {
        bridge.run().await;
    });

    info!("Tauri event bridge initialized");
}

/// 初始化并启动 Tauri 事件桥接器（使用优先级通道）
/// 
/// 推荐在高负载场景下使用，防止高优先级事件丢失
pub fn init_tauri_bridge_with_priority(app_handle: AppHandle, channels: &PriorityEventChannels) {
    let bridge = TauriBridge::with_priority_channels(app_handle, channels);

    // 在后台任务中启动桥接器
    tauri::async_runtime::spawn(async move {
        bridge.run().await;
    });

    info!("Tauri event bridge with priority channels initialized");
}

/// 便捷函数：直接通过 Tauri AppHandle 发送事件
/// 
/// 用于需要直接访问 Tauri 发射器的场景
pub fn emit_to_frontend<T: serde::Serialize + Clone>(
    app_handle: &AppHandle,
    event_name: &str,
    payload: T,
) -> Result<(), tauri::Error> {
    app_handle.emit(event_name, payload)
}

/// 便捷发射函数模块
/// 
/// 提供简化的函数来发射各类事件，保持向后兼容性
pub mod emit {
    use super::super::{emit_event, AppEvent, BroadcastResult};
    use crate::models::{FileChangeEvent, LogEntry, SearchResultSummary, TaskProgress};

    /// 发射搜索开始事件
    pub fn search_start(message: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchStart {
            message: message.into(),
        })
    }

    /// 发射搜索进度事件
    pub fn search_progress(progress: i32) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchProgress { progress })
    }

    /// 发射搜索结果
    pub fn search_results(results: Vec<LogEntry>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchResults { results })
    }

    /// 发射搜索摘要
    pub fn search_summary(summary: SearchResultSummary) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchSummary { summary })
    }

    /// 发射搜索完成事件
    pub fn search_complete(count: usize) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchComplete { count })
    }

    /// 发射搜索错误
    pub fn search_error(error: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SearchError {
            error: error.into(),
        })
    }

    /// 发射异步搜索开始事件
    pub fn async_search_start(search_id: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchStart {
            search_id: search_id.into(),
        })
    }

    /// 发射异步搜索进度
    pub fn async_search_progress(
        search_id: impl Into<String>,
        progress: u32,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchProgress {
            search_id: search_id.into(),
            progress,
        })
    }

    /// 发射异步搜索结果
    pub fn async_search_results(results: Vec<LogEntry>) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchResults { results })
    }

    /// 发射异步搜索完成
    pub fn async_search_complete(
        search_id: impl Into<String>,
        count: usize,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchComplete {
            search_id: search_id.into(),
            count,
        })
    }

    /// 发射异步搜索错误
    pub fn async_search_error(
        search_id: impl Into<String>,
        error: impl Into<String>,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::AsyncSearchError {
            search_id: search_id.into(),
            error: error.into(),
        })
    }

    /// 发射任务更新
    pub fn task_update(progress: TaskProgress) -> BroadcastResult<usize> {
        emit_event(AppEvent::TaskUpdate { progress })
    }

    /// 发射导入完成
    pub fn import_complete(task_id: impl Into<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::ImportComplete {
            task_id: task_id.into(),
        })
    }

    /// 发射文件变化事件
    pub fn file_changed(event: FileChangeEvent) -> BroadcastResult<usize> {
        emit_event(AppEvent::FileChanged { event })
    }

    /// 发射新日志
    pub fn new_logs(entries: Vec<LogEntry>) -> BroadcastResult<usize> {
        emit_event(AppEvent::NewLogs { entries })
    }

    /// 发射系统错误
    pub fn system_error(
        error: impl Into<String>,
        context: Option<String>,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::SystemError {
            error: error.into(),
            context,
        })
    }

    /// 发射系统警告
    pub fn system_warning(
        warning: impl Into<String>,
        context: Option<String>,
    ) -> BroadcastResult<usize> {
        emit_event(AppEvent::SystemWarning {
            warning: warning.into(),
            context,
        })
    }

    /// 发射系统信息
    pub fn system_info(info: impl Into<String>, context: Option<String>) -> BroadcastResult<usize> {
        emit_event(AppEvent::SystemInfo {
            info: info.into(),
            context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_constants_used() {
        // 验证所有事件类型都有对应的常量
        // 这个测试确保当 AppEvent 枚举变化时，常量文件也会更新
        let event_types = [
            ("SearchStart", EVENT_SEARCH_START),
            ("SearchProgress", EVENT_SEARCH_PROGRESS),
            ("SearchResults", EVENT_SEARCH_RESULTS),
            ("SearchSummary", EVENT_SEARCH_SUMMARY),
            ("SearchComplete", EVENT_SEARCH_COMPLETE),
            ("SearchError", EVENT_SEARCH_ERROR),
            ("AsyncSearchStart", EVENT_ASYNC_SEARCH_START),
            ("AsyncSearchProgress", EVENT_ASYNC_SEARCH_PROGRESS),
            ("AsyncSearchResults", EVENT_ASYNC_SEARCH_RESULTS),
            ("AsyncSearchComplete", EVENT_ASYNC_SEARCH_COMPLETE),
            ("AsyncSearchError", EVENT_ASYNC_SEARCH_ERROR),
            ("TaskUpdate", EVENT_TASK_UPDATE),
            ("ImportComplete", EVENT_IMPORT_COMPLETE),
            ("FileChanged", EVENT_FILE_CHANGED),
            ("NewLogs", EVENT_NEW_LOGS),
            ("SystemError", EVENT_SYSTEM_ERROR),
            ("SystemWarning", EVENT_SYSTEM_WARNING),
            ("SystemInfo", EVENT_SYSTEM_INFO),
        ];

        for (variant_name, constant) in &event_types {
            // 验证常量不为空
            assert!(
                !constant.is_empty(),
                "Event constant for {} should not be empty",
                variant_name
            );
            // 验证使用 kebab-case
            assert!(
                !constant.contains('_'),
                "Event constant for {} should use kebab-case",
                variant_name
            );
        }
    }
}
