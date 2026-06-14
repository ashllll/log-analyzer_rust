//! WorkspaceServiceImpl — WorkspaceService trait 的具体实现。
//!
//! 每个实例对应一个工作区，持有该工作区的全部运行时依赖。
//! 在导入完成时由命令层创建，存入 AppState.workspace_services。
//!
//! # 架构位置
//!
//! - **接口层**：`WorkspaceService` trait（application/workspace_service.rs）
//! - **实现层**：`WorkspaceServiceImpl`（本文件，infrastructure 层）
//! - **组装点**：导入命令完成时，由命令层调用 `WorkspaceServiceImpl::new` 创建
//!
//! # 当前实现状态
//!
//! - [x] SearchService（search / cancel_search / fetch_search_page）
//! - [x] ImportService（P4 完整实现）
//! - [x] WatchService（P5 完整实现，watcher 状态内嵌于实例中）

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tokio_util::sync::CancellationToken;

use crate::application::watch::{WatchEvent, WatchEventKind};
use la_core::domain::event::EventPublisher;
// use la_core::domain::SearchResultRepository; // 不需要直接导入，SearchUseCase 内部使用
use la_core::error::{AppError, Result};
use la_core::models::{SearchFilters, SearchQuery};
use la_core::traits::AppConfigProvider;

use notify::Watcher;
use tracing::error;

use la_archive::processor::process_path_with_cas;
use std::time::Duration;

use crate::application::workspace_service::{
    ImportOptions, ImportResult, ImportService, SearchService, WatchService, WorkspaceService,
};
use crate::application::SearchUseCase;
use crate::infrastructure::watcher_runner::WatcherRunner;
use crate::infrastructure::{
    CasLogFileRepository, DiskResultStoreRepo, QueryEngineLogSearcher, WorkspaceRepo,
};
use crate::services::file_watcher::WatcherState;
use crate::utils::encoding::decode_log_content;

// ============================================================================
// WorkspaceServiceImpl
// ============================================================================

/// 工作区服务的具体实现。
///
/// 每个实例持有单个工作区的全部运行时依赖：
/// - CAS（内容寻址存储）
/// - MetadataStore（SQLite 元数据索引）
/// - SearchEngineManager（Tantivy 搜索索引）
/// - DiskResultStore（磁盘结果缓存）
/// - EventPublisher（事件发射器，trait 对象）
/// - ThreadPool（Rayon 线程池，共享引用）
///
/// 内部管理搜索会话的生命周期（CancellationToken 存储与清理）。
pub struct WorkspaceServiceImpl {
    workspace_id: String,
    workspace_dir: PathBuf,
    /// P9: 存储层聚合（cas + metadata_store + search_engine + disk_result_store）
    repo: WorkspaceRepo,
    event_publisher: Arc<dyn EventPublisher>,
    thread_pool: Arc<rayon::ThreadPool>,
    /// FIX(P1-03): 缓存 QueryEngineLogSearcher，避免每次搜索都新建实例导致正则缓存失效
    searcher: Arc<QueryEngineLogSearcher>,
    /// 活跃的搜索会话 —— search_id → CancellationToken
    search_sessions: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// 文件监听器状态（P5：从 AppState::watchers 移入实例）
    watcher_state: Arc<Mutex<Option<WatcherState>>>,
}

impl WorkspaceServiceImpl {
    /// 创建新的 WorkspaceServiceImpl 实例。
    ///
    /// # 参数
    /// - `workspace_id`: 工作区唯一标识
    /// - `workspace_dir`: 工作区目录路径
    /// - `repo`: 预组装的存储层聚合（CAS + MetadataStore + SearchEngine + DiskResultStore）
    /// - `event_publisher`: 事件发射器 trait 对象
    /// - `thread_pool`: 全局共享的 Rayon 线程池
    /// - `regex_cache_size`: 正则缓存大小（传递给 QueryEngineLogSearcher）
    pub fn new(
        workspace_id: String,
        workspace_dir: PathBuf,
        repo: WorkspaceRepo,
        event_publisher: Arc<dyn EventPublisher>,
        thread_pool: Arc<rayon::ThreadPool>,
        regex_cache_size: usize,
    ) -> Self {
        Self {
            workspace_id,
            workspace_dir,
            repo,
            event_publisher,
            thread_pool,
            searcher: Arc::new(QueryEngineLogSearcher::new(regex_cache_size)),
            search_sessions: Arc::new(Mutex::new(HashMap::new())),
            watcher_state: Arc::new(Mutex::new(None)),
        }
    }
}

// ============================================================================
// WorkspaceService 实现
// ============================================================================

impl WorkspaceService for WorkspaceServiceImpl {
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    fn workspace_dir(&self) -> &PathBuf {
        &self.workspace_dir
    }

    fn cas(&self) -> &Arc<la_storage::ContentAddressableStorage> {
        self.repo.cas()
    }

    fn metadata_store(&self) -> &Arc<la_storage::MetadataStore> {
        self.repo.metadata_store()
    }

    fn search_engine(&self) -> &Arc<la_search::SearchEngineManager> {
        self.repo.search_engine()
    }
}

// ============================================================================
// SearchService 实现
// ============================================================================

#[async_trait]
impl SearchService for WorkspaceServiceImpl {
    async fn search(
        &self,
        query: SearchQuery,
        _raw_terms: Vec<String>,
        filters: SearchFilters,
        max_results: usize,
    ) -> Result<String> {
        // 1. 生成搜索会话 ID 和 CancellationToken（实现层内部管理完整生命周期）
        let search_id = uuid::Uuid::new_v4().to_string();
        let cancellation_token = CancellationToken::new();

        // 2. 注册 CancellationToken 到会话表（供 cancel_search 使用）
        {
            self.search_sessions
                .lock()
                .insert(search_id.clone(), cancellation_token.clone());
        }

        // 3. 组装 SearchUseCase 的依赖
        let log_files = Arc::new(CasLogFileRepository {
            metadata: self.repo.metadata_store().clone(),
            cas: self.repo.cas().clone(),
        });
        let results = Arc::new(DiskResultStoreRepo {
            store: self.repo.disk_result_store().clone(),
        });
        let searcher = Arc::clone(&self.searcher);

        // 4. 创建 SearchUseCase（泛型参数在此具体化）
        let use_case = SearchUseCase::new(
            log_files,
            results,
            self.event_publisher.clone(),
            searcher,
            self.thread_pool.clone(),
        );

        // 5. 执行搜索（spawn_blocking 内运行，立即返回）
        let workspace_id = self.workspace_id.clone();
        let search_id_clone = search_id.clone();
        let sessions = Arc::clone(&self.search_sessions);

        // 在后台执行搜索，完成后清理会话
        tokio::spawn(async move {
            let result = use_case
                .execute(
                    &workspace_id,
                    &query,
                    &filters,
                    max_results,
                    search_id_clone.clone(),
                    cancellation_token,
                )
                .await;

            // 搜索完成后（成功/失败/取消），从会话表中移除
            sessions.lock().remove(&search_id_clone);

            if let Err(e) = result {
                tracing::warn!(
                    search_id = %search_id_clone,
                    error = %e,
                    "Search execution failed"
                );
            }
        });

        Ok(search_id)
    }

    async fn fetch_search_page(
        &self,
        search_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<la_search::SearchPageResult> {
        let limit = limit.min(10_000);

        if !self.repo.disk_result_store().has_session(search_id) {
            return Err(AppError::not_found(format!(
                "Search session '{search_id}' not found"
            )));
        }

        self.repo
            .disk_result_store()
            .read_page(search_id, offset, limit)
            .map_err(|e| AppError::io_error(format!("Failed to read search page: {e}"), None))
    }

    async fn cancel_search(&self, search_id: &str) -> Result<()> {
        let token = {
            let sessions = self.search_sessions.lock();
            sessions.get(search_id).cloned()
        };
        match token {
            Some(t) => {
                t.cancel();
                Ok(())
            }
            None => Err(AppError::not_found(format!(
                "Search session '{search_id}' not found"
            ))),
        }
    }
}

// ============================================================================
// ImportService 实现
// ============================================================================

/// Tantivy 索引每 N 个文件提交一次，平衡 I/O 与内存使用。
const SEARCH_INDEX_COMMIT_EVERY_FILES: usize = 25;
/// 回退统计延迟（秒），等待逐文件分析先完成。
const FALLBACK_STATS_DELAY_SECS: u64 = 5;

// ============================================================================
// 辅助函数
// ============================================================================

/// 解析日志文件内容，计算时间范围和级别掩码。
///
/// 从 `commands/import.rs` 移动至此，供 ImportService 内部使用。
///
/// # Arguments
/// * `content` — 文件原始内容（字节）
///
/// # Returns
/// `(min_timestamp, max_timestamp, level_mask)`
pub(super) fn compute_file_stats(content: &[u8]) -> (Option<i64>, Option<i64>, Option<u8>) {
    crate::utils::log_stats::compute_file_stats(content)
}

/// 重建搜索索引内部实现。
///
/// 仅在索引为空时执行全量重建（首次导入）。
/// 后续导入的新文件通过 CAS 回退路径搜索，避免每次 O(n) 全量 I/O。
async fn rebuild_search_index_inner(
    metadata_store: Arc<la_storage::MetadataStore>,
    cas: Arc<la_storage::ContentAddressableStorage>,
    search_manager: Arc<la_search::SearchEngineManager>,
) -> std::result::Result<usize, String> {
    let index_empty = match search_manager.get_time_range() {
        Ok((_, _, count)) => count == 0,
        Err(_) => true,
    };

    if !index_empty {
        tracing::info!("Skipping index rebuild: Tantivy index already has documents");
        return Ok(0);
    }

    let files = metadata_store
        .get_all_files()
        .await
        .map_err(|e| format!("Failed to enumerate imported files for indexing: {e}"))?;

    tokio::task::spawn_blocking(move || -> std::result::Result<usize, String> {
        search_manager
            .clear_index()
            .map_err(|e| format!("Failed to clear search index before rebuild: {e}"))?;

        let mut indexed_lines = 0usize;

        for (file_index, file) in files.into_iter().enumerate() {
            let content = cas.read_content_sync(&file.sha256_hash).map_err(|e| {
                format!("Failed to read CAS content for {}: {e}", file.virtual_path)
            })?;
            let (content_str, _) = decode_log_content(&content);
            let real_path = format!("cas://{}", file.sha256_hash);

            let mut line_buffer = Vec::with_capacity(1024);
            let mut start_line_number = 1usize;

            for line in content_str.lines() {
                line_buffer.push(line.to_string());

                if line_buffer.len() >= 1024 {
                    let entries = la_core::utils::parse_log_lines(
                        &line_buffer,
                        &file.virtual_path,
                        &real_path,
                        indexed_lines,
                        start_line_number,
                    );
                    for entry in &entries {
                        search_manager
                            .add_document(entry)
                            .map_err(|e| format!("Failed to add indexed document: {e}"))?;
                    }
                    indexed_lines += entries.len();
                    start_line_number += line_buffer.len();
                    line_buffer.clear();
                }
            }

            if !line_buffer.is_empty() {
                let entries = la_core::utils::parse_log_lines(
                    &line_buffer,
                    &file.virtual_path,
                    &real_path,
                    indexed_lines,
                    start_line_number,
                );
                for entry in &entries {
                    search_manager
                        .add_document(entry)
                        .map_err(|e| format!("Failed to add indexed document: {e}"))?;
                }
                indexed_lines += entries.len();
            }

            if (file_index + 1) % SEARCH_INDEX_COMMIT_EVERY_FILES == 0 {
                search_manager
                    .commit()
                    .map_err(|e| format!("Failed to commit rebuilt search index: {e}"))?;
            }
        }

        search_manager
            .commit()
            .map_err(|e| format!("Failed to finalize rebuilt search index: {e}"))?;

        Ok(indexed_lines)
    })
    .await
    .map_err(|e| format!("Search index rebuild task panicked: {e}"))?
}

// ============================================================================
// ImportService trait 实现
// ============================================================================

#[async_trait]
impl ImportService for WorkspaceServiceImpl {
    async fn import_file(
        &self,
        source_path: &std::path::Path,
        _options: ImportOptions,
        config_provider: &dyn AppConfigProvider,
        task_id: &str,
        cancellation_token: CancellationToken,
    ) -> la_core::error::Result<ImportResult> {
        let root_name = source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // ── 1. 核心导入：调用 la_archive::processor::process_path_with_cas ──
        process_path_with_cas(
            source_path,
            &root_name,
            &self.workspace_dir,
            self.repo.cas(),
            self.repo.metadata_store().clone(),
            config_provider,
            task_id,
            &self.workspace_id,
            None, // parent_archive_id
            0,    // depth_level
        )
        .await
        .map_err(|e| {
            AppError::archive_error(
                format!("Import failed: {e}"),
                Some(source_path.to_path_buf()),
            )
        })?;

        // ── 2. 回退文件统计（处理 PENDING 文件，后台执行）──
        let metadata_store = self.repo.metadata_store().clone();
        let cas = Arc::clone(self.repo.cas());
        let workspace_id = self.workspace_id.clone();
        let ct = cancellation_token.clone();
        tokio::spawn(async move {
            if ct.is_cancelled() {
                return;
            }
            tokio::time::sleep(Duration::from_secs(FALLBACK_STATS_DELAY_SECS)).await;
            if ct.is_cancelled() {
                return;
            }

            match metadata_store.get_all_files().await {
                Ok(files) => {
                    let mut updated = 0usize;
                    let mut failed = 0usize;

                    for file in files {
                        if file.analysis_status == la_core::storage_types::AnalysisStatus::Ready {
                            continue;
                        }

                        match cas.read_content(&file.sha256_hash).await {
                            Ok(content) => {
                                let (min_ts, max_ts, level_mask) = compute_file_stats(&content);
                                if let Err(e) = metadata_store
                                    .update_file_ready(
                                        &file.virtual_path,
                                        min_ts,
                                        max_ts,
                                        level_mask,
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        virtual_path = %file.virtual_path,
                                        error = %e,
                                        "Failed to update file ready status in fallback"
                                    );
                                    failed += 1;
                                } else {
                                    updated += 1;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    virtual_path = %file.virtual_path,
                                    hash = %file.sha256_hash,
                                    error = %e,
                                    "Failed to read file content for fallback stats"
                                );
                                failed += 1;
                            }
                        }
                    }

                    if updated > 0 || failed > 0 {
                        tracing::info!(
                            workspace_id = %workspace_id,
                            stats_updated = updated,
                            stats_failed = failed,
                            "Fallback file stats computation completed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        "Failed to get files for fallback stats computation"
                    );
                }
            }
        });

        // ── 3. 重建搜索索引（仅首次导入，后台执行）──
        let metadata_store = self.repo.metadata_store().clone();
        let cas = Arc::clone(self.repo.cas());
        let search_engine = Arc::clone(self.repo.search_engine());
        let workspace_id_bg = self.workspace_id.clone();
        let ct_bg = cancellation_token.clone();
        tokio::spawn(async move {
            if ct_bg.is_cancelled() {
                return;
            }
            if let Err(e) = rebuild_search_index_inner(metadata_store, cas, search_engine).await {
                tracing::warn!(
                    workspace_id = %workspace_id_bg,
                    error = %e,
                    "Background Tantivy index rebuild failed; get_time_range may be stale"
                );
            } else {
                tracing::info!(
                    workspace_id = %workspace_id_bg,
                    "Background Tantivy index rebuild completed"
                );
            }
        });

        // ── 4. 统计已导入文件数 ──
        let files_imported = self.repo.metadata_store().count_files().await.unwrap_or(0) as usize;

        Ok(ImportResult {
            root_name,
            files_imported,
        })
    }
}

// ============================================================================
// WatchService 实现（P5 完整实现）
// ============================================================================

#[async_trait]
impl WatchService for WorkspaceServiceImpl {
    async fn start_watch(&self, watch_path: &str) -> Result<()> {
        // ── 1. Validate ──
        let watch_path_buf = PathBuf::from(watch_path);
        if !watch_path_buf.exists() {
            return Err(AppError::validation_error(format!(
                "Path does not exist: {watch_path}"
            )));
        }

        // ── 2. Check not already watching ──
        {
            let state = self.watcher_state.lock();
            if state.as_ref().map(|w| w.is_active).unwrap_or(false) {
                return Err(AppError::validation_error(
                    "Workspace is already being watched".to_string(),
                ));
            }
        }

        // ── 3. Create notify watcher, convert to WatchEvent ──
        let (tx, notify_rx) =
            crossbeam::channel::unbounded::<std::result::Result<notify::Event, notify::Error>>();
        let (watch_tx, rx) = crossbeam::channel::unbounded::<WatchEvent>();

        let mut watcher = notify::recommended_watcher(tx)
            .map_err(|e| AppError::io_error(format!("Failed to create file watcher: {e}"), None))?;

        watcher
            .watch(&watch_path_buf, notify::RecursiveMode::Recursive)
            .map_err(|e| AppError::io_error(format!("Failed to start watching path: {e}"), None))?;

        // Conversion thread: notify events → WatchEvent
        std::thread::spawn(move || {
            for res in notify_rx {
                let event = match res {
                    Ok(e) => {
                        let kind = match e.kind {
                            notify::EventKind::Create(_) => WatchEventKind::Create,
                            notify::EventKind::Modify(_) => WatchEventKind::Modify,
                            notify::EventKind::Remove(_) => WatchEventKind::Remove,
                            _ => WatchEventKind::Other,
                        };
                        WatchEvent {
                            kind,
                            paths: e.paths,
                        }
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "NotifyWatcher: event error, skipping");
                        continue;
                    }
                };
                if watch_tx.send(event).is_err() {
                    break;
                }
            }
        });

        // ── 4. Create WatcherState ──
        let watcher_state = WatcherState {
            workspace_id: self.workspace_id.clone(),
            watched_path: watch_path_buf.clone(),
            file_offsets: HashMap::new(),
            line_counts: HashMap::new(),
            is_active: true,
            thread_handle: Arc::new(parking_lot::Mutex::new(None)),
            watcher: Arc::new(parking_lot::Mutex::new(Some(watcher))),
        };

        let thread_handle_arc = Arc::clone(&watcher_state.thread_handle);

        // ── 5. Store in self ──
        *self.watcher_state.lock() = Some(watcher_state);

        // ── 6. Spawn background thread via WatcherRunner (P7) ──
        let runner = WatcherRunner::new(
            Arc::clone(&self.event_publisher),
            self.repo.cas().clone(),
            self.repo.metadata_store().clone(),
            Arc::clone(self.repo.search_engine()),
            Arc::clone(&self.watcher_state),
            self.workspace_id.clone(),
        );
        let handle = thread::spawn(move || runner.run(rx));

        // ── 8. Store thread handle for later join ──
        *thread_handle_arc.lock() = Some(handle);

        Ok(())
    }

    async fn stop_watch(&self) -> Result<()> {
        let mut state = self.watcher_state.lock();

        let Some(ref mut ws) = *state else {
            return Err(AppError::validation_error(
                "No active watcher found for this workspace".to_string(),
            ));
        };

        // Set inactive flag so the background thread exits
        ws.is_active = false;
        let thread_handle = ws.thread_handle.lock().take();
        let watcher_opt = ws.watcher.lock().take();

        // Remove from state
        *state = None;
        drop(state);

        // Drop the watcher — this closes the tx channel, causing rx iteration to end
        drop(watcher_opt);

        // Join the background thread outside the lock to avoid deadlock
        if let Some(handle) = thread_handle {
            if handle.join().is_err() {
                error!("Failed to join watcher thread");
            }
        }

        Ok(())
    }

    async fn is_watching(&self) -> Result<bool> {
        let guard = self.watcher_state.lock();
        Ok(guard.as_ref().map(|w| w.is_active).unwrap_or(false))
    }
}
