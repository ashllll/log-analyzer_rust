//! Infrastructure adapters — implement domain traits for concrete types.

pub mod archive_extractor;
pub mod event_publisher;
pub mod log_file_repo;
pub mod result_store;
pub mod searcher;
pub mod task_scheduler;
pub mod workspace_service_impl;

pub use archive_extractor::ArchiveManagerAdapter;
pub use event_publisher::TauriEventPublisher;
pub use log_file_repo::CasLogFileRepository;
pub use result_store::DiskResultStoreRepo;
pub use searcher::QueryEngineLogSearcher;
pub use task_scheduler::TaskManagerAdapter;
pub use workspace_service_impl::WorkspaceServiceImpl;
