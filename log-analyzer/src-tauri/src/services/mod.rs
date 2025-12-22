pub mod event_bus;
pub mod file_watcher;
pub mod index_store;
pub mod metadata_db;
pub mod pattern_matcher;
pub mod query_executor;
pub mod query_planner;
pub mod query_validator;
pub mod search_statistics;
pub mod service_config;
pub mod service_container;
pub mod service_lifecycle;

#[cfg(test)]
mod dependency_management_tests;

#[cfg(test)]
mod error_handling_property_tests;

#[cfg(test)]
mod concurrency_property_tests;

#[cfg(test)]
mod integration_tests;

// 重新导出所有公共类型和函数
pub use event_bus::{get_event_bus, AppEvent, EventBus, EventSubscriber};
pub use file_watcher::{
    append_to_workspace_index, get_file_metadata, parse_log_lines, parse_metadata,
    read_file_from_offset,
};
pub use index_store::{load_index, save_index};
pub use metadata_db::MetadataDB;
pub use query_executor::{MatchDetail, QueryExecutor};
pub use query_planner::ExecutionPlan;
pub use search_statistics::calculate_keyword_statistics;
pub use service_config::ServiceConfiguration;
pub use service_container::{AppServices, AppServicesBuilder};
pub use service_lifecycle::{
    HealthStatus, OverallHealth, Service, ServiceHealth, ServiceLifecycleManager,
};
