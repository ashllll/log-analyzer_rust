//! Infrastructure adapters — implement domain traits for concrete types.

pub mod event_publisher;
pub mod log_file_repo;
pub mod result_store;
pub mod searcher;
pub mod workspace_repo;

pub use event_publisher::TauriEventPublisher;
pub use log_file_repo::CasLogFileRepository;
pub use result_store::DiskResultStoreRepo;
pub use searcher::QueryEngineLogSearcher;
pub use workspace_repo::RuntimeWorkspaceRepository;
