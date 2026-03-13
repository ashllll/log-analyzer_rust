//! FFI 集成测试
//! 
//! 验证所有 FFI 修复：
//! 1. 错误类型化（无 panic）
//! 2. 全局 Runtime
//! 3. Session 存储修复

use std::sync::Once;

// 确保测试时初始化全局运行时
static INIT: Once = Once::new();

fn init_test_runtime() {
    INIT.call_once(|| {
        let config = log_analyzer::ffi::runtime::RuntimeConfig::for_constrained();
        let _ = log_analyzer::ffi::runtime::init_runtime(Some(config));
    });
}

#[test]
fn test_ffi_error_conversion() {
    use log_analyzer::ffi::error::{FfiError, FfiErrorCode};
    use log_analyzer::error::AppError;

    // 测试错误类型转换
    let app_error = AppError::NotFound("test".to_string());
    let ffi_error: FfiError = app_error.into();
    
    match ffi_error {
        FfiError::NotFound { resource, id } => {
            assert_eq!(resource, "unknown");
            assert_eq!(id, "test");
        }
        _ => panic!("Expected NotFound error"),
    }

    // 测试搜索错误转换
    let app_error = AppError::Search {
        _message: "search failed".to_string(),
        _source: None,
    };
    let ffi_error: FfiError = app_error.into();
    assert!(matches!(ffi_error, FfiError::Search { .. }));

    // 测试验证错误转换
    let app_error = AppError::Validation("invalid input".to_string());
    let ffi_error: FfiError = app_error.into();
    assert!(matches!(ffi_error, FfiError::Validation { .. }));
}

#[test]
fn test_ffi_error_creation() {
    use log_analyzer::ffi::error::{FfiError, FfiErrorCode};

    // 测试各种错误创建方式
    let err = FfiError::new(FfiErrorCode::NotFound, "resource not found");
    assert!(matches!(err, FfiError::NotFound { .. }));

    let err = FfiError::with_details(FfiErrorCode::IoError, "read failed", "permission denied");
    assert!(matches!(err, FfiError::Io { .. }));

    let err = FfiError::not_found("user", "123");
    assert!(matches!(err, FfiError::NotFound { .. }));

    let err = FfiError::invalid_argument("age", "must be positive");
    assert!(matches!(err, FfiError::InvalidArgument { .. }));

    let err = FfiError::timeout(5000);
    assert!(matches!(err, FfiError::Timeout { .. }));

    let err = FfiError::unknown("unknown error");
    assert!(matches!(err, FfiError::Internal { .. }));
}

#[test]
fn test_global_runtime_singleton() {
    use log_analyzer::ffi::runtime::{
        init_runtime, is_runtime_initialized, block_on, RuntimeConfig
    };

    // 确保测试开始时已初始化（由 init_test_runtime 调用）
    init_test_runtime();

    // 验证已初始化
    assert!(is_runtime_initialized());

    // 测试 block_on 执行异步任务
    let result = block_on(async { 42 });
    assert_eq!(result.unwrap(), 42);

    // 测试 block_on 执行异步计算
    let result = block_on(async {
        let sum: u32 = (1..=10).sum();
        sum
    });
    assert_eq!(result.unwrap(), 55);
}

#[test]
fn test_global_runtime_idempotent() {
    use log_analyzer::ffi::runtime::{
        init_runtime, is_runtime_initialized, RuntimeConfig
    };

    // 确保已初始化
    init_test_runtime();
    assert!(is_runtime_initialized());

    // 再次初始化应该幂等（不会失败）
    let config = RuntimeConfig::for_constrained();
    let result = init_runtime(Some(config));
    
    // 即使再次调用，也应该返回已存在的运行时
    assert!(result.is_ok());
    assert!(is_runtime_initialized());
}

#[test]
fn test_session_storage_basic() {
    use log_analyzer::ffi::global_state::{
        get_session, insert_session, SessionHolder, SessionState
    };

    // 清理之前的测试数据
    log_analyzer::ffi::global_state::clear_all_sessions();

    // 创建 Session
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

    // 获取（修复：不再返回 None）
    let retrieved = get_session("test-session");
    assert!(retrieved.is_some(), "Session should be retrievable after insert");

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.session_id(), "test-session");
    assert_eq!(retrieved.workspace_id(), "test-workspace");

    // 修改状态并验证
    session.set_state(SessionState::Active);
    assert!(matches!(session.state(), SessionState::Active));
    
    // 通过 get_session 获取的应该看到相同的状态变化
    // （因为共享同一个 Arc<Mutex<>>）

    // 清理
    log_analyzer::ffi::global_state::clear_all_sessions();
}

#[test]
fn test_session_storage_clone() {
    use log_analyzer::ffi::global_state::{
        get_session, insert_session, SessionHolder, SessionState
    };

    // 清理之前的测试数据
    log_analyzer::ffi::global_state::clear_all_sessions();

    let session = SessionHolder::new(
        "clone-test".to_string(),
        "workspace".to_string(),
    );

    // 克隆
    let cloned = session.clone();

    // 插入克隆的版本
    insert_session(cloned);

    // 修改原对象状态
    session.set_state(SessionState::Active);

    // 通过 get_session 获取并验证状态变化
    let retrieved = get_session("clone-test").unwrap();
    assert!(matches!(retrieved.state(), SessionState::Active), 
        "Cloned session should reflect state changes");

    // 清理
    log_analyzer::ffi::global_state::clear_all_sessions();
}

#[test]
fn test_session_storage_multiple() {
    use log_analyzer::ffi::global_state::{
        get_session, insert_session, SessionHolder, list_sessions,
        get_session_count, get_all_session_ids, remove_session
    };

    // 清理之前的测试数据
    log_analyzer::ffi::global_state::clear_all_sessions();

    // 创建多个 sessions
    for i in 0..5 {
        let session = SessionHolder::new(
            format!("session-{}", i),
            format!("workspace-{}", i % 2),
        );
        insert_session(session);
    }

    // 验证数量
    assert_eq!(get_session_count(), 5);

    // 验证列表
    let sessions = list_sessions();
    assert_eq!(sessions.len(), 5);

    // 验证所有 ID 都存在
    let ids = get_all_session_ids();
    assert_eq!(ids.len(), 5);
    for i in 0..5 {
        assert!(ids.contains(&format!("session-{}", i)));
    }

    // 验证每个都能获取
    for i in 0..5 {
        let session = get_session(&format!("session-{}", i));
        assert!(session.is_some());
        assert_eq!(session.unwrap().workspace_id(), format!("workspace-{}", i % 2));
    }

    // 移除一个
    let removed = remove_session("session-2");
    assert!(removed.is_some());
    assert_eq!(get_session_count(), 4);
    assert!(get_session("session-2").is_none());

    // 清理
    log_analyzer::ffi::global_state::clear_all_sessions();
}

#[test]
fn test_session_workspace_filtering() {
    use log_analyzer::ffi::global_state::{
        insert_session, SessionHolder, get_workspace_sessions
    };

    // 清理之前的测试数据
    log_analyzer::ffi::global_state::clear_all_sessions();

    // 创建工作区 A 的 sessions
    for i in 0..3 {
        let session = SessionHolder::new(
            format!("ws-a-session-{}", i),
            "workspace-a".to_string(),
        );
        insert_session(session);
    }

    // 创建工作区 B 的 sessions
    for i in 0..2 {
        let session = SessionHolder::new(
            format!("ws-b-session-{}", i),
            "workspace-b".to_string(),
        );
        insert_session(session);
    }

    // 验证工作区过滤
    let ws_a_sessions = get_workspace_sessions("workspace-a");
    assert_eq!(ws_a_sessions.len(), 3);

    let ws_b_sessions = get_workspace_sessions("workspace-b");
    assert_eq!(ws_b_sessions.len(), 2);

    let ws_c_sessions = get_workspace_sessions("workspace-c");
    assert!(ws_c_sessions.is_empty());

    // 清理
    log_analyzer::ffi::global_state::clear_all_sessions();
}

#[test]
fn test_session_info() {
    use log_analyzer::ffi::global_state::{
        insert_session, get_session_info, SessionHolder, SessionState
    };

    // 清理之前的测试数据
    log_analyzer::ffi::global_state::clear_all_sessions();

    let session = SessionHolder::new(
        "info-test".to_string(),
        "workspace".to_string(),
    );
    session.set_state(SessionState::Active);

    insert_session(session);

    // 获取 SessionInfo（不持有 SessionHolder）
    let info = get_session_info("info-test").unwrap();
    assert_eq!(info.session_id, "info-test");
    assert_eq!(info.workspace_id, "workspace");
    assert!(matches!(info.state, SessionState::Active));

    // 验证不存在的 session 返回 None
    assert!(get_session_info("nonexistent").is_none());

    // 清理
    log_analyzer::ffi::global_state::clear_all_sessions();
}

#[test]
fn test_ffi_error_wrapper() {
    use log_analyzer::ffi::error::{FfiError, FfiResultWrapper};

    // 测试成功结果
    let ok_result: Result<i32, log_analyzer::ffi::error::FfiError> = Ok(42);
    let wrapper = FfiResultWrapper::from_result(ok_result);
    assert!(wrapper.success);
    assert_eq!(wrapper.data, Some(42));
    assert!(wrapper.error.is_none());

    // 测试错误结果
    let err_result: Result<i32, log_analyzer::ffi::error::FfiError> = 
        Err(FfiError::unknown("test error"));
    let wrapper = FfiResultWrapper::from_result(err_result);
    assert!(!wrapper.success);
    assert!(wrapper.error.is_some());
    assert!(wrapper.data.is_none());
}

#[test]
fn test_ffi_error_code_mapping() {
    use log_analyzer::ffi::error::{FfiError, FfiErrorCode};

    // 测试各种错误变体的 code() 方法
    let err = FfiError::NotInitialized;
    assert_eq!(err.code(), FfiErrorCode::InitializationFailed);

    let err = FfiError::Io { message: "test".to_string(), path: None };
    assert_eq!(err.code(), FfiErrorCode::IoError);

    let err = FfiError::NotFound { resource: "test".to_string(), id: "123".to_string() };
    assert_eq!(err.code(), FfiErrorCode::NotFound);

    let err = FfiError::Timeout { duration_ms: 5000 };
    assert_eq!(err.code(), FfiErrorCode::Timeout);

    let err = FfiError::Internal { message: "test".to_string() };
    assert_eq!(err.code(), FfiErrorCode::Internal);
}

#[tokio::test]
async fn test_ffi_runtime_spawn() {
    use log_analyzer::ffi::runtime::{
        spawn, block_on, RuntimeConfig, init_runtime
    };

    // 确保运行时已初始化
    let _ = init_runtime(Some(RuntimeConfig::for_constrained()));

    // 测试 spawn 生成异步任务
    let handle = spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        42
    });

    let result = handle.await.unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_ffi_global_state_initialization() {
    use log_analyzer::ffi::global_state::{
        is_initialized, init_global_state, get_app_state
    };
    use log_analyzer::models::AppState;

    // 初始状态应该是未初始化
    assert!(!is_initialized());

    // 创建测试用的 AppState
    let app_state = AppState::default();
    let app_data_dir = std::path::PathBuf::from("/tmp/test");

    // 初始化全局状态
    init_global_state(app_state, app_data_dir);

    // 验证已初始化
    assert!(is_initialized());

    // 验证可以获取 AppState
    assert!(get_app_state().is_some());
}
