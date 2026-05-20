//! Infrastructure adapters — implement domain traits for concrete types.

pub mod event_publisher;
pub mod log_file_repo;
pub mod result_store;

pub use event_publisher::TauriEventPublisher;
pub use log_file_repo::CasLogFileRepository;
pub use result_store::DiskResultStoreRepo;
