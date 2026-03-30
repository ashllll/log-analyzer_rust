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

// 从 la-core 重新导出的 trait 和类型
pub use la_core::traits::{
    ContentStorage, MetadataStorage, PlanResult, QueryExecutor, QueryValidation, ValidationResult,
};

use crate::error::Result;
use crate::models::search::SearchQuery;

/// 查询计划 trait
///
/// QueryPlanning 保留在主 crate 中，因为它引用了 ExecutionPlan（运行时类型）。
/// 实现者可以从搜索查询创建执行计划，支持不同的计划策略互换使用。
pub trait QueryPlanning: Send + Sync {
    /// 为搜索查询创建执行计划
    ///
    /// # Arguments
    /// * `query` - 待计划的搜索查询
    ///
    /// # Returns
    /// 包含计划结果或错误的 `Result`
    fn plan(&self, query: &SearchQuery) -> Result<PlanResult>;

    /// 为搜索查询构建执行计划
    ///
    /// 该方法返回 QueryExecutor 使用的实际 ExecutionPlan。
    /// 子类应重写此方法以提供实际的执行计划。
    ///
    /// # Arguments
    /// * `query` - 待计划的搜索查询
    ///
    /// # Returns
    /// 包含执行计划或错误的 `Result`
    fn build_execution_plan(
        &self,
        query: &SearchQuery,
    ) -> Result<crate::services::query_planner::ExecutionPlan> {
        // 默认实现：调用 plan() 并返回最小执行计划
        // 这是向后兼容的降级方案
        let plan_result = self.plan(query)?;
        Ok(crate::services::query_planner::ExecutionPlan::new(
            crate::services::query_planner::SearchStrategy::And,
            Vec::new(),
            plan_result.steps.len(),
            Vec::new(),
        ))
    }
}

// ============== Trait Implementations for Existing Types ==============

// 注意：这些 trait 的具体实现（QueryValidator, QueryPlanner, ContentAddressableStorage, MetadataStore）
// 位于各自的源文件中，以保持模块化。
