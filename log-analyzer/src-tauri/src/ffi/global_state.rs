//! 全局 FFI 状态管理器
//!
//! 由于 FFI 函数无法直接使用 Tauri State，需要创建独立的全局状态管理。
//! 这个模块提供全局状态访问接口，在 Tauri 应用启动时初始化。
//!
//! ## 架构
//!
//! ```
//! Tauri App → init_global_state(AppState, PathBuf) → GLOBAL_STATE
//! FFI Call → get_global_state() → FfiContext → Business Logic
//! ```
//!
//! ## 线程安全
//!
//! - 使用 `std::sync::OnceLock` 确保线程安全的单次初始化
//! - 使用 `parking_lot::RwLock` 提供高性能读写锁

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use parking_lot::RwLock;

use crate::models::AppState;

/// Typestate Session 的运行时表示
///
/// 由于 Typestate 模式在编译期固定类型，我们需要在运行时
/// 使用枚举来擦除类型状态，以便在 FFI 层动态管理
pub enum SessionHolder {
    /// 未映射状态
    Unmapped(crate::services::typestate::Session<crate::services::typestate::Unmapped>),
    /// 已映射状态
    Mapped(crate::services::typestate::Session<crate::services::typestate::Mapped>),
    /// 已索引状态
    Indexed(crate::services::typestate::Session<crate::services::typestate::Indexed>),
}

impl SessionHolder {
    /// 获取文件路径
    pub fn path(&self) -> &std::path::PathBuf {
        match self {
            SessionHolder::Unmapped(s) => s.path(),
            SessionHolder::Mapped(s) => s.path(),
            SessionHolder::Indexed(s) => s.path(),
        }
    }

    /// 获取文件元数据
    pub fn metadata(&self) -> Option<&crate::services::typestate::FileMetadata> {
        match self {
            SessionHolder::Unmapped(s) => s.metadata(),
            SessionHolder::Mapped(s) => s.metadata(),
            SessionHolder::Indexed(s) => s.metadata(),
        }
    }

    /// 获取状态名称
    pub fn state_name(&self) -> &'static str {
        match self {
            SessionHolder::Unmapped(_) => "Unmapped",
            SessionHolder::Mapped(_) => "Mapped",
            SessionHolder::Indexed(_) => "Indexed",
        }
    }

    /// 检查是否已索引
    pub fn is_indexed(&self) -> bool {
        matches!(self, SessionHolder::Indexed(_))
    }
}

/// FFI 上下文
///
/// 包含 FFI 函数所需的全部上下文信息
#[derive(Clone)]
pub struct FfiContext {
    /// 应用状态（包含所有共享数据）
    pub app_state: AppState,
    /// 应用数据目录路径（用于工作区目录解析）
    pub app_data_dir: PathBuf,
}

/// 全局状态包装器
///
/// 使用 RwLock 提供读写访问
type GlobalStateInner = RwLock<Option<FfiContext>>;

/// 全局状态单例
static GLOBAL_STATE: OnceLock<GlobalStateInner> = OnceLock::new();

/// 初始化全局状态
///
/// 在 Tauri 应用启动时调用，设置全局 AppState 和应用数据目录
///
/// # 参数
///
/// * `state` - Tauri 应用的 AppState
/// * `app_data_dir` - 应用数据目录路径
///
/// # 示例
///
/// ```rust,ignore
/// // 在 main.rs 的 setup 函数中调用
/// let app_data_dir = app.path().app_data_dir().unwrap();
/// init_global_state(app_state.clone(), app_data_dir);
/// ```
pub fn init_global_state(state: AppState, app_data_dir: PathBuf) {
    let inner = GLOBAL_STATE.get_or_init(|| RwLock::new(None));
    let mut guard = inner.write();
    *guard = Some(FfiContext {
        app_state: state,
        app_data_dir,
    });
    tracing::info!("全局 FFI 状态管理器已初始化");
}

/// 获取全局 FFI 上下文
///
/// FFI 函数通过此方法访问完整上下文
///
/// # 返回
///
/// 返回 FfiContext 的克隆，如果未初始化则返回 None
///
/// # 示例
///
/// ```rust,ignore
/// if let Some(ctx) = get_global_state() {
///     // 访问 app_state 和 app_data_dir...
/// }
/// ```
pub fn get_global_state() -> Option<FfiContext> {
    GLOBAL_STATE.get().and_then(|inner| inner.read().clone())
}

/// 获取全局 AppState
///
/// 便捷方法，只获取 AppState 部分
pub fn get_app_state() -> Option<AppState> {
    get_global_state().map(|ctx| ctx.app_state)
}

/// 获取应用数据目录
///
/// 便捷方法，只获取应用数据目录路径
pub fn get_app_data_dir() -> Option<PathBuf> {
    get_global_state().map(|ctx| ctx.app_data_dir)
}

/// 检查全局状态是否已初始化
pub fn is_initialized() -> bool {
    GLOBAL_STATE
        .get()
        .map(|inner| inner.read().is_some())
        .unwrap_or(false)
}

/// 更新全局状态
///
/// 用于状态变更后同步更新
pub fn update_global_state(context: FfiContext) {
    if let Some(inner) = GLOBAL_STATE.get() {
        let mut guard = inner.write();
        *guard = Some(context);
    }
}

/// 清除全局状态
///
/// 在应用关闭时调用
pub fn clear_global_state() {
    if let Some(inner) = GLOBAL_STATE.get() {
        let mut guard = inner.write();
        *guard = None;
    }
    tracing::info!("全局 FFI 状态管理器已清除");
}

// ==================== Session 管理 ====================

/// 全局 Session 存储
///
/// 存储所有活跃的 Typestate Session 实例
type SessionStore = RwLock<HashMap<String, SessionHolder>>;

/// 全局 Session 存储单例
static SESSION_STORE: OnceLock<SessionStore> = OnceLock::new();

/// 获取 Session 存储
fn get_session_store() -> &'static SessionStore {
    SESSION_STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 创建新的 Session
///
/// 创建一个未映射状态的 Session 并存储
///
/// # 参数
///
/// * `session_id` - 唯一的会话 ID
/// * `path` - 文件路径
///
/// # 返回
///
/// 成功返回 SessionInfo，失败返回错误信息
pub fn create_session(
    session_id: String,
    path: impl Into<PathBuf>,
) -> Result<crate::ffi::types::SessionInfo, String> {
    use crate::ffi::types::{SessionInfo, SessionState};
    use crate::services::typestate::{Session, Unmapped};

    let path = path.into();

    // 创建未映射状态的 Session
    let session =
        Session::<Unmapped>::new(&path).map_err(|e| format!("创建 Session 失败: {}", e))?;

    let file_size = session.metadata().map(|m| m.size).unwrap_or(0);
    let file_path = path.display().to_string();

    // 存储到全局存储
    {
        let store = get_session_store();
        let mut guard = store.write();
        guard.insert(session_id.clone(), SessionHolder::Unmapped(session));
    }

    tracing::info!(
        session_id = %session_id,
        path = %file_path,
        "Session 已创建"
    );

    Ok(SessionInfo {
        session_id,
        file_path,
        state: SessionState::Unmapped,
        file_size,
    })
}

/// 获取 Session
///
/// 从全局存储中获取 Session
pub fn get_session(session_id: &str) -> Option<SessionHolder> {
    let store = get_session_store();
    let guard = store.read();
    // 返回一个克隆的引用（需要重建）
    // 注意：Session 不支持 Clone，需要通过 get_session_info 获取信息
    let _ = guard.get(session_id);
    None
}

/// 获取 Session 信息
///
/// 获取指定 Session 的信息
pub fn get_session_info(session_id: &str) -> Option<crate::ffi::types::SessionInfo> {
    use crate::ffi::types::{SessionInfo, SessionState};

    let store = get_session_store();
    let guard = store.read();

    guard.get(session_id).map(|holder| {
        let state = match holder.state_name() {
            "Unmapped" => SessionState::Unmapped,
            "Mapped" => SessionState::Mapped,
            "Indexed" => SessionState::Indexed,
            _ => SessionState::Unmapped,
        };

        SessionInfo {
            session_id: session_id.to_string(),
            file_path: holder.path().display().to_string(),
            state,
            file_size: holder.metadata().map(|m| m.size).unwrap_or(0),
        }
    })
}

/// 映射 Session
///
/// 将 Session 从 Unmapped 状态转换为 Mapped 状态
pub fn map_session(session_id: &str) -> Result<bool, String> {
    let store = get_session_store();
    let mut guard = store.write();

    // 取出 Unmapped 状态的 Session
    let holder = guard
        .remove(session_id)
        .ok_or_else(|| format!("Session 不存在: {}", session_id))?;

    if let SessionHolder::Unmapped(session) = holder {
        // 转换为 Mapped 状态
        let mapped = session
            .map()
            .map_err(|e| format!("映射 Session 失败: {}", e))?;

        guard.insert(session_id.to_string(), SessionHolder::Mapped(mapped));

        tracing::info!(session_id = %session_id, "Session 已映射");
        Ok(true)
    } else {
        // 恢复原状态
        guard.insert(session_id.to_string(), holder);
        Err(format!("Session 状态不是 Unmapped: {}", session_id))
    }
}

/// 索引 Session
///
/// 将 Session 从 Mapped 状态转换为 Indexed 状态
pub fn index_session(session_id: &str) -> Result<usize, String> {
    let store = get_session_store();
    let mut guard = store.write();

    // 取出 Mapped 状态的 Session
    let holder = guard
        .remove(session_id)
        .ok_or_else(|| format!("Session 不存在: {}", session_id))?;

    if let SessionHolder::Mapped(session) = holder {
        // 转换为 Indexed 状态
        let indexed = session
            .index()
            .map_err(|e| format!("索引 Session 失败: {}", e))?;

        let entry_count = indexed.entry_count();

        guard.insert(session_id.to_string(), SessionHolder::Indexed(indexed));

        tracing::info!(
            session_id = %session_id,
            entry_count = entry_count,
            "Session 已索引"
        );
        Ok(entry_count)
    } else {
        // 恢复原状态
        guard.insert(session_id.to_string(), holder);
        Err(format!("Session 状态不是 Mapped: {}", session_id))
    }
}

/// 获取 Session 索引条目
///
/// 从已索引的 Session 中获取索引条目
pub fn get_session_entries(
    session_id: &str,
) -> Result<Vec<crate::ffi::types::IndexEntryData>, String> {
    use crate::ffi::types::IndexEntryData;

    let store = get_session_store();
    let guard = store.read();

    let holder = guard
        .get(session_id)
        .ok_or_else(|| format!("Session 不存在: {}", session_id))?;

    if let SessionHolder::Indexed(session) = holder {
        let entries = session
            .entries()
            .iter()
            .map(|e| IndexEntryData {
                line_number: e.line_number,
                byte_offset: e.byte_offset,
                length: e.length,
            })
            .collect();

        Ok(entries)
    } else {
        Err(format!("Session 未索引: {}", session_id))
    }
}

/// 删除 Session
///
/// 从全局存储中删除 Session
pub fn remove_session(session_id: &str) -> Result<bool, String> {
    let store = get_session_store();
    let mut guard = store.write();

    if guard.remove(session_id).is_some() {
        tracing::info!(session_id = %session_id, "Session 已删除");
        Ok(true)
    } else {
        Err(format!("Session 不存在: {}", session_id))
    }
}

/// 获取所有 Session ID
///
/// 返回所有活跃 Session 的 ID 列表
pub fn get_all_session_ids() -> Vec<String> {
    let store = get_session_store();
    let guard = store.read();
    guard.keys().cloned().collect()
}

/// 获取活跃 Session 数量
pub fn get_session_count() -> usize {
    let store = get_session_store();
    let guard = store.read();
    guard.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        // 初始状态应该是未初始化
        // 注意：由于是全局状态，这个测试可能会受到其他测试的影响
        assert!(GLOBAL_STATE.get().is_none() || !is_initialized());
    }

    #[test]
    fn test_session_store() {
        // 测试 Session 存储的基本功能
        let _store = get_session_store();
        assert_eq!(get_session_count(), 0);
    }
}
