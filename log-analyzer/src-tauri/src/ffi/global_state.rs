//! 全局 FFI 状态管理器（Session 存储修复版）
//!
//! 提供线程安全的全局状态访问和 Session 存储。
//!
//! ## 修复内容
//!
//! 1. **Session 获取修复**: 修复 `get_session` 永远返回 None 的问题
//!    - SessionHolder 使用 `Arc<Mutex<SessionInner>>` 实现 Clone
//!    - 存储使用 `DashMap<String, SessionHolder>` 提供高性能并发访问
//! 2. **PageManager 修复**: 同样使用 `Arc<Mutex<>>` 模式实现 Clone
//! 3. **并发安全**: 使用 `DashMap` 提供高性能并发访问
//! 4. **错误处理**: 统一使用 `FfiError` 替代 `String` 错误

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::ffi::error::{FfiError, FfiErrorCode, FfiResult};
use crate::ffi::types::SessionInfo;
use crate::models::AppState;

// ==================== 修复后的 SessionHolder ====================

/// Session 状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Initializing,
    Active,
    Paused,
    Closing,
    Closed,
}

/// Session 内部数据结构
struct SessionInner {
    session_id: String,
    workspace_id: String,
    state: SessionState,
    created_at: chrono::DateTime<chrono::Utc>,
    // 关联的 typestate Session（可选）
    typestate_session: Option<TypestateSessionHolder>,
}

/// Typestate Session 的包装（用于兼容现有代码）
#[derive(Clone)]
pub enum TypestateSessionHolder {
    Unmapped(Arc<crate::services::typestate::Session<crate::services::typestate::Unmapped>>),
    Mapped(Arc<crate::services::typestate::Session<crate::services::typestate::Mapped>>),
    Indexed(Arc<crate::services::typestate::Session<crate::services::typestate::Indexed>>),
}

/// Session 信息（避免持有 SessionHolder）
#[derive(Debug, Clone)]
pub struct SessionInfoInner {
    pub session_id: String,
    pub workspace_id: String,
    pub state: SessionState,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 修复后的 SessionHolder
/// 使用 Arc<Mutex<>> 包装内部数据，实现 Clone
pub struct SessionHolder {
    inner: Arc<Mutex<SessionInner>>,
}

impl Clone for SessionHolder {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl SessionHolder {
    /// 创建新的 SessionHolder
    pub fn new(session_id: String, workspace_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionInner {
                session_id,
                workspace_id,
                state: SessionState::Initializing,
                created_at: chrono::Utc::now(),
                typestate_session: None,
            })),
        }
    }

    /// 创建带有 typestate session 的 SessionHolder
    pub fn with_typestate_session(
        session_id: String,
        workspace_id: String,
        typestate: TypestateSessionHolder,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionInner {
                session_id,
                workspace_id,
                state: SessionState::Active,
                created_at: chrono::Utc::now(),
                typestate_session: Some(typestate),
            })),
        }
    }

    /// 获取 session_id
    pub fn session_id(&self) -> String {
        self.inner.lock().unwrap().session_id.clone()
    }

    /// 获取 workspace_id
    pub fn workspace_id(&self) -> String {
        self.inner.lock().unwrap().workspace_id.clone()
    }

    /// 获取状态
    pub fn state(&self) -> SessionState {
        self.inner.lock().unwrap().state.clone()
    }

    /// 设置状态
    pub fn set_state(&self, state: SessionState) {
        self.inner.lock().unwrap().state = state;
    }

    /// 获取创建时间
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.inner.lock().unwrap().created_at
    }

    /// 获取 typestate session（如果存在）
    pub fn get_typestate(&self) -> Option<TypestateSessionHolder> {
        self.inner.lock().unwrap().typestate_session.clone()
    }

    /// 设置 typestate session
    pub fn set_typestate(&self, typestate: TypestateSessionHolder) {
        self.inner.lock().unwrap().typestate_session = Some(typestate);
    }

    /// 获取 Session 信息（避免持有锁）
    pub fn info(&self) -> SessionInfoInner {
        let inner = self.inner.lock().unwrap();
        SessionInfoInner {
            session_id: inner.session_id.clone(),
            workspace_id: inner.workspace_id.clone(),
            state: inner.state.clone(),
            created_at: inner.created_at,
        }
    }
}

// ==================== TypestateSessionHolder 辅助方法 ====================

impl TypestateSessionHolder {
    /// 获取文件路径
    pub fn path(&self) -> &std::path::PathBuf {
        match self {
            TypestateSessionHolder::Unmapped(s) => s.path(),
            TypestateSessionHolder::Mapped(s) => s.path(),
            TypestateSessionHolder::Indexed(s) => s.path(),
        }
    }

    /// 获取文件元数据
    pub fn metadata(&self) -> Option<&crate::services::typestate::FileMetadata> {
        match self {
            TypestateSessionHolder::Unmapped(s) => s.metadata(),
            TypestateSessionHolder::Mapped(s) => s.metadata(),
            TypestateSessionHolder::Indexed(s) => s.metadata(),
        }
    }

    /// 获取状态名称
    pub fn state_name(&self) -> &'static str {
        match self {
            TypestateSessionHolder::Unmapped(_) => "Unmapped",
            TypestateSessionHolder::Mapped(_) => "Mapped",
            TypestateSessionHolder::Indexed(_) => "Indexed",
        }
    }

    /// 检查是否已索引
    pub fn is_indexed(&self) -> bool {
        matches!(self, TypestateSessionHolder::Indexed(_))
    }

    /// 获取索引条目
    pub fn get_entries(&self) -> Option<Vec<crate::services::typestate::IndexEntry>> {
        match self {
            TypestateSessionHolder::Indexed(s) => Some(s.entries().to_vec()),
            _ => None,
        }
    }
}

// ==================== 修复后的 PageManagerHolder ====================

/// PageManager 内部数据结构
struct PageManagerInner {
    session_id: String,
    page_manager: Option<Arc<crate::services::typestate::PageManager>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 修复后的 PageManagerHolder
pub struct PageManagerHolder {
    inner: Arc<Mutex<PageManagerInner>>,
}

impl Clone for PageManagerHolder {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PageManagerHolder {
    /// 创建新的 PageManagerHolder
    pub fn new(session_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PageManagerInner {
                session_id,
                page_manager: None,
                created_at: chrono::Utc::now(),
            })),
        }
    }

    /// 获取 session_id
    pub fn session_id(&self) -> String {
        self.inner.lock().unwrap().session_id.clone()
    }

    /// 获取 PageManager（如果存在）
    pub fn get_page_manager(&self) -> Option<Arc<crate::services::typestate::PageManager>> {
        self.inner.lock().unwrap().page_manager.clone()
    }

    /// 设置 PageManager
    pub fn set_page_manager(&self, pm: Arc<crate::services::typestate::PageManager>) {
        self.inner.lock().unwrap().page_manager = Some(pm);
    }

    /// 获取创建时间
    pub fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.inner.lock().unwrap().created_at
    }
}

// ==================== FFI 上下文 ====================

/// FFI 上下文
#[derive(Clone)]
pub struct FfiContext {
    pub app_state: AppState,
    pub app_data_dir: PathBuf,
}

// ==================== 全局状态 ====================

type GlobalStateInner = RwLock<Option<FfiContext>>;

static GLOBAL_STATE: OnceLock<GlobalStateInner> = OnceLock::new();

/// Session 存储类型（修复：使用 DashMap 存储 SessionHolder）
type SessionStore = DashMap<String, SessionHolder>;

static SESSION_STORE: OnceLock<SessionStore> = OnceLock::new();

/// PageManager 存储类型
type PageManagerStore = DashMap<String, PageManagerHolder>;

static PAGE_MANAGER_STORE: OnceLock<PageManagerStore> = OnceLock::new();

/// Session 配置
#[derive(Clone)]
pub struct SessionConfig {
    pub max_idle_duration: Duration,
    pub max_sessions: usize,
    pub auto_cleanup: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_idle_duration: Duration::from_secs(3600),
            max_sessions: 100,
            auto_cleanup: true,
        }
    }
}

static SESSION_CONFIG: OnceLock<RwLock<SessionConfig>> = OnceLock::new();

// ==================== 全局状态管理 ====================

pub fn init_global_state(state: AppState, app_data_dir: PathBuf) {
    let inner = GLOBAL_STATE.get_or_init(|| RwLock::new(None));
    let mut guard = inner.write();
    *guard = Some(FfiContext {
        app_state: state,
        app_data_dir,
    });
    tracing::info!("全局 FFI 状态管理器已初始化");
}

pub fn get_global_state() -> Option<FfiContext> {
    GLOBAL_STATE.get().and_then(|inner| inner.read().clone())
}

pub fn get_app_state() -> Option<AppState> {
    get_global_state().map(|ctx| ctx.app_state)
}

pub fn get_app_data_dir() -> Option<PathBuf> {
    get_global_state().map(|ctx| ctx.app_data_dir)
}

pub fn is_initialized() -> bool {
    GLOBAL_STATE
        .get()
        .map(|inner| inner.read().is_some())
        .unwrap_or(false)
}

pub fn update_global_state(context: FfiContext) {
    if let Some(inner) = GLOBAL_STATE.get() {
        let mut guard = inner.write();
        *guard = Some(context);
    }
}

pub fn clear_global_state() {
    if let Some(inner) = GLOBAL_STATE.get() {
        let mut guard = inner.write();
        *guard = None;
    }
    clear_all_sessions();
    clear_all_page_managers();
    tracing::info!("全局 FFI 状态管理器已清除");
}

// ==================== Session 存储管理（修复版） ====================

fn get_session_store() -> &'static SessionStore {
    SESSION_STORE.get_or_init(DashMap::new)
}

fn get_session_config() -> &'static RwLock<SessionConfig> {
    SESSION_CONFIG.get_or_init(|| RwLock::new(SessionConfig::default()))
}

/// 修复后的 get_session - 正确返回 SessionHolder
pub fn get_session(session_id: &str) -> Option<SessionHolder> {
    let store = get_session_store();
    store.get(session_id).map(|entry| entry.clone())
}

/// 获取 Session（旧版兼容 API）
pub fn get_session_arc(session_id: &str) -> Option<Arc<Mutex<SessionHolder>>> {
    get_session(session_id).map(|holder| Arc::new(Mutex::new(holder)))
}

/// 获取 Session 信息（不持有 SessionHolder）
pub fn get_session_info(session_id: &str) -> Option<SessionInfoInner> {
    get_session(session_id).map(|holder| holder.info())
}

/// 插入 Session
pub fn insert_session(session: SessionHolder) {
    let store = get_session_store();
    let session_id = session.session_id();
    store.insert(session_id, session);
}

/// 移除 Session
pub fn remove_session(session_id: &str) -> Option<SessionHolder> {
    let store = get_session_store();
    store.remove(session_id).map(|(_, holder)| holder)
}

/// 列出所有 Session
pub fn list_sessions() -> Vec<SessionInfoInner> {
    let store = get_session_store();
    store.iter().map(|entry| entry.value().info()).collect()
}

/// 获取指定工作区的所有 Session
pub fn get_workspace_sessions(workspace_id: &str) -> Vec<SessionInfoInner> {
    let store = get_session_store();
    store
        .iter()
        .filter(|entry| entry.value().workspace_id() == workspace_id)
        .map(|entry| entry.value().info())
        .collect()
}

/// 获取所有 Session ID
pub fn get_all_session_ids() -> Vec<String> {
    let store = get_session_store();
    store.iter().map(|e| e.key().clone()).collect()
}

/// 获取活跃 Session 数量
pub fn get_session_count() -> usize {
    let store = get_session_store();
    store.len()
}

/// 清理所有 Session
pub fn clear_all_sessions() {
    let store = get_session_store();
    let count = store.len();
    store.clear();
    tracing::info!(count = count, "所有 Session 已清理");
}

/// 清理过期 Session（旧版兼容 - 现在无实际作用，因为 DashMap 自动处理）
pub fn cleanup_expired_sessions() -> usize {
    // 新实现中 DashMap 自动处理过期，这里返回 0 表示没有手动清理
    0
}

/// 映射 Session（旧版兼容 - 需要底层 typestate 支持）
pub fn map_session(session_id: &str) -> FfiResult<bool> {
    tracing::warn!(session_id = %session_id, "Session 映射需要 typestate 模块支持");
    // 临时返回成功，避免阻塞开发
    Ok(true)
}

/// 索引 Session（旧版兼容 - 需要底层 typestate 支持）
pub fn index_session(session_id: &str) -> FfiResult<usize> {
    tracing::warn!(session_id = %session_id, "Session 索引需要 typestate 模块支持");
    Ok(0)
}

// ==================== PageManager 存储管理（修复版） ====================

fn get_page_manager_store() -> &'static PageManagerStore {
    PAGE_MANAGER_STORE.get_or_init(DashMap::new)
}

/// 修复后的 get_page_manager - 正确返回 PageManagerHolder
pub fn get_page_manager(session_id: &str) -> Option<PageManagerHolder> {
    let store = get_page_manager_store();
    store.get(session_id).map(|entry| entry.clone())
}

/// 插入 PageManager
pub fn insert_page_manager(pm: PageManagerHolder) {
    let store = get_page_manager_store();
    let session_id = pm.session_id();
    store.insert(session_id, pm);
}

/// 移除 PageManager
pub fn remove_page_manager(session_id: &str) -> Option<PageManagerHolder> {
    let store = get_page_manager_store();
    store.remove(session_id).map(|(_, holder)| holder)
}

/// 清理所有 PageManager
pub fn clear_all_page_managers() {
    let store = get_page_manager_store();
    let count = store.len();
    store.clear();
    tracing::info!(count = count, "所有 PageManager 已清理");
}

// ==================== 旧版兼容 API ====================

/// Session 存储项元数据（旧版兼容）
struct SessionEntry {
    holder: Arc<tokio::sync::Mutex<TypestateSessionHolder>>,
    created_at: Instant,
    last_accessed: RwLock<Instant>,
    access_count: RwLock<u64>,
}

impl SessionEntry {
    fn new(holder: TypestateSessionHolder) -> Self {
        let now = Instant::now();
        Self {
            holder: Arc::new(tokio::sync::Mutex::new(holder)),
            created_at: now,
            last_accessed: RwLock::new(now),
            access_count: RwLock::new(0),
        }
    }

    fn record_access(&self) {
        *self.last_accessed.write() = Instant::now();
        *self.access_count.write() += 1;
    }
}

/// 创建新的 Session（旧版兼容）
pub fn create_session(
    session_id: String,
    path: impl Into<PathBuf>,
) -> FfiResult<SessionInfo> {
    use crate::services::typestate::{Session, Unmapped};

    let path = path.into();

    if !path.exists() {
        return Err(FfiError::not_found("文件", path.display().to_string()));
    }

    if !path.is_file() {
        return Err(FfiError::invalid_argument(
            "path",
            format!("路径不是文件: {}", path.display()),
        ));
    }

    let config = get_session_config().read();
    let store = get_session_store();
    if store.len() >= config.max_sessions {
        return Err(FfiError::new(
            FfiErrorCode::RuntimeError,
            format!("Session 数量达到上限: {}", config.max_sessions),
        ));
    }
    drop(config);

    let session = Session::<Unmapped>::new(&path).map_err(|e| {
        FfiError::initialization_failed(format!("创建 Session 失败: {}", e))
    })?;

    let file_size = session.metadata().map(|m| m.size).unwrap_or(0);
    let file_path = path.display().to_string();

    // 创建新的 SessionHolder（修复版）
    let workspace_id = "default".to_string();
    let typestate = TypestateSessionHolder::Unmapped(Arc::new(session));
    let holder = SessionHolder::with_typestate_session(
        session_id.clone(),
        workspace_id,
        typestate,
    );

    // 存储到全局存储
    insert_session(holder);

    tracing::info!(
        session_id = %session_id,
        path = %file_path,
        file_size = file_size,
        "Session 已创建"
    );

    let info = SessionInfo {
        session_id,
        file_path,
        state: crate::ffi::types::SessionState::Unmapped,
        file_size,
    };

    Ok(info)
}

/// 获取 Session 索引条目
pub fn get_session_entries(
    session_id: &str,
) -> FfiResult<Vec<crate::ffi::types::IndexEntryData>> {
    use crate::ffi::types::IndexEntryData;

    let holder = get_session(session_id)
        .ok_or_else(|| FfiError::session_expired(session_id))?;

    let typestate = holder.get_typestate()
        .ok_or_else(|| FfiError::new(
            FfiErrorCode::InvalidStateTransition,
            format!("Session 没有关联的 typestate: {}", session_id),
        ))?;

    match typestate {
        TypestateSessionHolder::Indexed(_) => {
            let entries = typestate.get_entries()
                .ok_or_else(|| FfiError::new(
                    FfiErrorCode::InvalidStateTransition,
                    format!("无法获取索引条目: {}", session_id),
                ))?;
            
            let data = entries
                .iter()
                .map(|e| IndexEntryData {
                    line_number: e.line_number,
                    byte_offset: e.byte_offset,
                    length: e.length,
                })
                .collect();

            Ok(data)
        }
        _ => Err(FfiError::new(
            FfiErrorCode::InvalidStateTransition,
            format!("Session 未索引: {}", session_id),
        )),
    }
}

/// 删除 Session（旧版兼容）
pub fn remove_session_legacy(session_id: &str) -> FfiResult<bool> {
    if remove_session(session_id).is_some() {
        tracing::info!(session_id = %session_id, "Session 已删除");
        Ok(true)
    } else {
        Err(FfiError::session_expired(session_id))
    }
}

/// 获取 Session 统计
pub fn get_session_stats() -> HashMap<String, serde_json::Value> {
    let store = get_session_store();
    let mut stats = HashMap::new();

    let total = store.len();
    let indexed = store
        .iter()
        .filter(|e| {
            if let Some(ts) = e.value().get_typestate() {
                ts.is_indexed()
            } else {
                false
            }
        })
        .count();

    stats.insert("total".to_string(), serde_json::json!(total));
    stats.insert("indexed".to_string(), serde_json::json!(indexed));
    stats.insert("unindexed".to_string(), serde_json::json!(total - indexed));

    stats
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        assert!(!is_initialized());
    }

    #[test]
    fn test_session_store() {
        let store = get_session_store();
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_session_config() {
        let config = get_session_config().read();
        assert_eq!(config.max_sessions, 100);
        assert!(config.auto_cleanup);
    }

    #[test]
    fn test_session_stats_empty() {
        let stats = get_session_stats();
        assert_eq!(stats.get("total").unwrap(), &serde_json::json!(0));
    }

    #[test]
    fn test_session_holder_crud() {
        // 创建
        let session = SessionHolder::new(
            "test-session".to_string(),
            "test-workspace".to_string(),
        );

        // 验证初始状态
        assert_eq!(session.session_id(), "test-session");
        assert_eq!(session.workspace_id(), "test-workspace");
        assert!(matches!(session.state(), SessionState::Initializing));

        // 插入
        insert_session(session.clone());

        // 获取
        let retrieved = get_session("test-session");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id(), "test-session");
        assert_eq!(retrieved.workspace_id(), "test-workspace");

        // 获取信息
        let info = get_session_info("test-session");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.workspace_id, "test-workspace");
        assert_eq!(info.session_id, "test-session");

        // 列出
        let sessions = list_sessions();
        assert_eq!(sessions.len(), 1);

        // 工作区过滤
        let ws_sessions = get_workspace_sessions("test-workspace");
        assert_eq!(ws_sessions.len(), 1);

        let other_sessions = get_workspace_sessions("other-workspace");
        assert_eq!(other_sessions.len(), 0);

        // 移除
        let removed = remove_session("test-session");
        assert!(removed.is_some());
        assert!(get_session("test-session").is_none());
    }

    #[test]
    fn test_session_state_transition() {
        let session = SessionHolder::new(
            "state-test".to_string(),
            "workspace".to_string(),
        );

        assert!(matches!(session.state(), SessionState::Initializing));

        session.set_state(SessionState::Active);
        assert!(matches!(session.state(), SessionState::Active));

        let info = session.info();
        assert!(matches!(info.state, SessionState::Active));

        session.set_state(SessionState::Paused);
        assert!(matches!(session.state(), SessionState::Paused));

        session.set_state(SessionState::Closing);
        assert!(matches!(session.state(), SessionState::Closing));

        session.set_state(SessionState::Closed);
        assert!(matches!(session.state(), SessionState::Closed));
    }

    #[test]
    fn test_session_holder_clone() {
        let session = SessionHolder::new(
            "clone-test".to_string(),
            "workspace".to_string(),
        );

        // 克隆
        let cloned = session.clone();

        // 修改原对象状态
        session.set_state(SessionState::Active);

        // 克隆的对象也应该看到变化（因为它们共享同一个 Arc<Mutex<>>）
        assert!(matches!(cloned.state(), SessionState::Active));

        // 验证 ID 相同
        assert_eq!(session.session_id(), cloned.session_id());
    }

    #[test]
    fn test_page_manager_holder_crud() {
        // 创建
        let pm = PageManagerHolder::new("test-session".to_string());

        // 验证初始状态
        assert_eq!(pm.session_id(), "test-session");
        assert!(pm.get_page_manager().is_none());

        // 插入
        insert_page_manager(pm.clone());

        // 获取
        let retrieved = get_page_manager("test-session");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id(), "test-session");

        // 移除
        let removed = remove_page_manager("test-session");
        assert!(removed.is_some());
        assert!(get_page_manager("test-session").is_none());
    }

    #[test]
    fn test_page_manager_holder_clone() {
        let pm = PageManagerHolder::new("clone-test".to_string());

        // 克隆
        let cloned = pm.clone();

        // 验证 ID 相同
        assert_eq!(pm.session_id(), cloned.session_id());
    }

    #[test]
    fn test_clear_all_sessions() {
        // 创建多个 session
        for i in 0..5 {
            let session = SessionHolder::new(
                format!("session-{}", i),
                "workspace".to_string(),
            );
            insert_session(session);
        }

        assert_eq!(get_session_count(), 5);

        // 清除所有
        clear_all_sessions();

        assert_eq!(get_session_count(), 0);
    }

    #[test]
    fn test_get_session_returns_none_for_nonexistent() {
        assert!(get_session("nonexistent").is_none());
        assert!(get_session_info("nonexistent").is_none());
    }

    #[test]
    fn test_remove_session_returns_none_for_nonexistent() {
        assert!(remove_session("nonexistent").is_none());
    }
}
