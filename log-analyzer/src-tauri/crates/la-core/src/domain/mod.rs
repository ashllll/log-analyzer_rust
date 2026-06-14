//! Domain layer — pure trait interfaces for Clean Architecture.
//!
//! These traits define the contracts between application use cases and
//! infrastructure implementations. They have zero dependencies on Tauri,
//! file system, or any external framework.
//!
//! # Dependency Rule
//!
//! - **Application layer** depends on these traits (calls them).
//! - **Infrastructure layer** implements these traits (provides them).
//! - **Interfaces layer** wires them together (dependency injection).

pub mod event;
pub mod extract;
pub mod filter;
pub mod log_file;
pub mod result_store;
pub mod search;
pub mod task;
pub mod workspace;

// Re-export all traits + types
pub use event::EventPublisher;
pub use extract::{ArchiveEntry, ArchiveExtractor, ExtractionPolicy, ExtractionSummary};
pub use filter::{Filter, LineMetadata};
pub use log_file::LogFileRepository;
pub use result_store::{SearchResultPage, SearchResultRepository};
pub use search::{ExecutionPlan, LogSearcher, MatchPlan};
pub use task::{TaskHandle, TaskScheduler};
pub use workspace::{WorkspaceInfo, WorkspaceRepository, WorkspaceStatus};
