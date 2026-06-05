//! Services Layer Trait Abstractions
//!
//! Re-exports domain traits from la-core for convenience.

pub use la_core::traits::{
    ContentStorage, MetadataStorage, PlanResult, QueryExecutor, QueryValidation, ValidationResult,
};
