//! Services Layer Trait Abstractions
//!
//! This module defines the core trait abstractions for the services layer,
//! implementing the Dependency Inversion Principle (DIP) from SOLID principles.
//!
//! ## Design Rationale
//!
//! Traits allow us to:
//! - Decouple concrete implementations from their consumers
//! - Enable easy testing through mock implementations
//! - Support dependency injection for better architecture
//! - Allow different implementations to be swapped at runtime

use crate::error::Result;
use crate::models::search::SearchQuery;
use crate::storage::metadata_store::FileMetadata;
use async_trait::async_trait;

/// Query validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the query is valid
    pub is_valid: bool,
    /// List of validation errors (empty if valid)
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
        }
    }

    /// Create a failed validation result with a single error
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            errors: vec![msg.into()],
        }
    }
}

/// Query validation trait
///
/// Implementors can validate search queries for correctness and constraints.
/// This trait is object-safe and can be used for dependency injection.
pub trait QueryValidation: Send + Sync {
    /// Validate a search query
    ///
    /// # Arguments
    /// * `query` - The search query to validate
    ///
    /// # Returns
    /// A `ValidationResult` containing the validation status and any errors
    fn validate(&self, query: &SearchQuery) -> ValidationResult;
}

/// Execution plan for query planning
///
/// This is a simplified representation of the planning result.
/// The actual plan details are implementation-specific.
#[derive(Debug, Clone)]
pub struct PlanResult {
    /// Execution steps description
    pub steps: Vec<String>,
    /// Estimated cost (arbitrary units, lower is better)
    pub estimated_cost: u32,
}

impl PlanResult {
    /// Create a new plan result
    pub fn new(steps: Vec<String>, estimated_cost: u32) -> Self {
        Self {
            steps,
            estimated_cost,
        }
    }
}

/// Query planning trait
///
/// Implementors can create execution plans from search queries.
/// This enables different planning strategies to be used interchangeably.
pub trait QueryPlanning: Send + Sync {
    /// Create an execution plan for a search query
    ///
    /// # Arguments
    /// * `query` - The search query to plan
    ///
    /// # Returns
    /// A `Result` containing the plan result or an error
    fn plan(&self, query: &SearchQuery) -> Result<PlanResult>;

    /// Build an execution plan for a search query
    ///
    /// This method returns the actual ExecutionPlan used by QueryExecutor.
    /// Subclasses should override this to provide the actual execution plan.
    ///
    /// # Arguments
    /// * `query` - The search query to plan
    ///
    /// # Returns
    /// A `Result` containing the execution plan or an error
    fn build_execution_plan(
        &self,
        query: &SearchQuery,
    ) -> Result<crate::services::query_planner::ExecutionPlan> {
        // Default implementation: call plan() and return a minimal execution plan
        // This is a fallback for backward compatibility
        let plan_result = self.plan(query)?;
        Ok(crate::services::query_planner::ExecutionPlan::new(
            crate::services::query_planner::SearchStrategy::And,
            Vec::new(),
            plan_result.steps.len(),
            Vec::new(),
        ))
    }
}

/// Content storage trait (CAS abstraction)
///
/// This trait abstracts content-addressable storage operations,
/// allowing different storage backends to be used interchangeably.
#[async_trait]
pub trait ContentStorage: Send + Sync {
    /// Store content and return its hash
    ///
    /// # Arguments
    /// * `content` - The content bytes to store
    ///
    /// # Returns
    /// The content hash (typically SHA-256) as a string
    async fn store(&self, content: &[u8]) -> Result<String>;

    /// Retrieve content by its hash
    ///
    /// # Arguments
    /// * `hash` - The content hash
    ///
    /// # Returns
    /// The content bytes, or an error if not found
    async fn retrieve(&self, hash: &str) -> Result<Vec<u8>>;

    /// Check if content exists by its hash
    ///
    /// # Arguments
    /// * `hash` - The content hash
    ///
    /// # Returns
    /// `true` if the content exists, `false` otherwise
    async fn exists(&self, hash: &str) -> bool;
}

/// Metadata storage trait
///
/// This trait abstracts metadata storage operations,
/// enabling different database backends or testing mocks.
#[async_trait]
pub trait MetadataStorage: Send + Sync {
    /// Insert file metadata
    ///
    /// # Arguments
    /// * `metadata` - The file metadata to insert
    ///
    /// # Returns
    /// The auto-generated file ID
    async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64>;

    /// Get all files
    ///
    /// # Returns
    /// A vector of all file metadata
    async fn get_all_files(&self) -> Result<Vec<FileMetadata>>;

    /// Get file by its hash
    ///
    /// # Arguments
    /// * `hash` - The file hash
    ///
    /// # Returns
    /// The file metadata if found, or `None`
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>>;
}

/// Generic query executor trait
///
/// This trait abstracts query execution, allowing for different
/// execution strategies and easy testing.
pub trait QueryExecutor: Send + Sync {
    /// Execute a search query and return results
    ///
    /// # Type Parameters
    /// * `T` - The result type
    ///
    /// # Arguments
    /// * `query` - The search query to execute
    ///
    /// # Returns
    /// Query results or an error
    fn execute<T>(&self, query: &SearchQuery) -> Result<T>
    where
        T: Send + Sync + 'static;
}

// ============== Trait Implementations for Existing Types ==============

// Note: The actual implementations of these traits for the concrete types
// (QueryValidator, QueryPlanner, ContentAddressableStorage, MetadataStore)
// are in their respective source files to maintain modularity.
