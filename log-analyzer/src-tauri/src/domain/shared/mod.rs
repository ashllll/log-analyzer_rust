//! 共享领域模型

pub mod events;
pub mod specifications;
pub mod value_objects;

pub use events::{DomainEvent, DomainEventBus, EventHandler, LogAnalysisEvent};
pub use specifications::{
    AndSpec, KeywordSpecification, LogLevelSpecification, NotSpec, OrSpec,
    SearchQuerySpecification, SourceFileSpecification, Specification, TagSpecification,
    TimeRangeSpecification, WorkspaceNameSpecification, WorkspacePathFilterSpecification,
    WorkspacePathValidationSpecification, WorkspaceStatusSpecification,
};
pub use value_objects::{
    BoundedString, Email, FilePath, NonEmptyString, PositiveInteger, Url, ValueError,
};
