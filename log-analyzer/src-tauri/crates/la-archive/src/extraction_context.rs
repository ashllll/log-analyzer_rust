//! Extraction context and stack management for iterative archive traversal
//!
//! This module provides the core data structures for managing extraction state
//! during iterative depth-first traversal of nested archives. It replaces
//! recursive calls with explicit stack management to prevent stack overflow.

use std::path::PathBuf;
use std::time::Instant;

/// Extraction context tracking state during archive processing
///
/// This structure maintains all necessary state information for a single
/// extraction operation, including depth tracking, accumulated metrics,
/// and parent archive relationships.
#[derive(Debug, Clone)]
pub struct ExtractionContext {
    /// Workspace identifier for this extraction
    pub workspace_id: String,

    /// Current nesting depth (0 = top-level archive)
    pub current_depth: usize,

    /// Path to parent archive (None for top-level)
    pub parent_archive: Option<PathBuf>,

    /// Total bytes extracted so far
    pub accumulated_size: u64,

    /// Total number of files extracted so far
    pub accumulated_files: usize,

    /// Timestamp when extraction started
    pub start_time: Instant,
}

impl ExtractionContext {
    /// Create a new extraction context for a top-level archive
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - Unique identifier for the workspace
    ///
    /// # Returns
    ///
    /// A new ExtractionContext with depth 0 and no parent
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            current_depth: 0,
            parent_archive: None,
            accumulated_size: 0,
            accumulated_files: 0,
            start_time: Instant::now(),
        }
    }

    /// Create a child context for a nested archive
    ///
    /// # Arguments
    ///
    /// * `parent_archive` - Path to the parent archive containing this nested archive
    ///
    /// # Returns
    ///
    /// A new ExtractionContext with incremented depth and parent reference
    pub fn create_child(&self, parent_archive: PathBuf) -> Self {
        Self {
            workspace_id: self.workspace_id.clone(),
            current_depth: self.current_depth + 1,
            parent_archive: Some(parent_archive),
            accumulated_size: self.accumulated_size,
            accumulated_files: self.accumulated_files,
            start_time: self.start_time,
        }
    }

    /// Update accumulated metrics after extracting files
    ///
    /// # Arguments
    ///
    /// * `bytes` - Number of bytes extracted
    /// * `files` - Number of files extracted
    pub fn update_metrics(&mut self, bytes: u64, files: usize) {
        self.accumulated_size = self.accumulated_size.saturating_add(bytes);
        self.accumulated_files = self.accumulated_files.saturating_add(files);
    }

    /// Get elapsed time since extraction started
    ///
    /// # Returns
    ///
    /// Duration since start_time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Check if depth limit is reached
    ///
    /// # Arguments
    ///
    /// * `max_depth` - Maximum allowed nesting depth
    ///
    /// # Returns
    ///
    /// true if current depth equals or exceeds max_depth
    pub fn is_depth_limit_reached(&self, max_depth: usize) -> bool {
        self.current_depth >= max_depth
    }
}

/// Item in the extraction stack representing a pending archive to process
///
/// Each item contains all information needed to extract an archive and
/// continue traversal, including the archive path, target directory,
/// current depth, and parent context.
#[derive(Debug, Clone)]
pub struct ExtractionItem {
    /// Path to the archive file to extract
    pub archive_path: PathBuf,

    /// Target directory for extraction
    pub target_dir: PathBuf,

    /// Current nesting depth for this item
    pub depth: usize,

    /// Parent extraction context
    pub parent_context: ExtractionContext,
}

impl ExtractionItem {
    /// Create a new extraction item
    ///
    /// # Arguments
    ///
    /// * `archive_path` - Path to the archive to extract
    /// * `target_dir` - Directory where files should be extracted
    /// * `depth` - Current nesting depth
    /// * `parent_context` - Parent extraction context
    ///
    /// # Returns
    ///
    /// A new ExtractionItem ready for processing
    pub fn new(
        archive_path: PathBuf,
        target_dir: PathBuf,
        depth: usize,
        parent_context: ExtractionContext,
    ) -> Self {
        Self {
            archive_path,
            target_dir,
            depth,
            parent_context,
        }
    }
}

/// Stack for managing iterative depth-first traversal of nested archives
///
/// This structure replaces recursive function calls with explicit stack
/// management, preventing stack overflow when processing deeply nested
/// archives. It enforces a maximum stack size to prevent memory exhaustion.
#[derive(Debug)]
pub struct ExtractionStack {
    /// Internal stack of extraction items
    items: Vec<ExtractionItem>,

    /// Maximum allowed stack size
    max_size: usize,
}

impl ExtractionStack {
    /// Default maximum stack size (prevents memory exhaustion)
    pub const DEFAULT_MAX_SIZE: usize = 1000;

    /// Create a new extraction stack with default maximum size
    ///
    /// # Returns
    ///
    /// A new empty ExtractionStack
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            max_size: Self::DEFAULT_MAX_SIZE,
        }
    }

    /// Create a new extraction stack with custom maximum size
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of items allowed on the stack
    ///
    /// # Returns
    ///
    /// A new empty ExtractionStack with specified max_size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            items: Vec::new(),
            max_size,
        }
    }

    /// Push an item onto the stack
    ///
    /// # Arguments
    ///
    /// * `item` - ExtractionItem to push
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Item successfully pushed
    /// * `Err(String)` - Stack size limit exceeded
    pub fn push(&mut self, item: ExtractionItem) -> Result<(), String> {
        if self.items.len() >= self.max_size {
            return Err(format!(
                "Stack size limit exceeded: {} items (max: {})",
                self.items.len(),
                self.max_size
            ));
        }

        self.items.push(item);
        Ok(())
    }

    /// Pop an item from the stack
    ///
    /// # Returns
    ///
    /// * `Some(ExtractionItem)` - Next item to process
    /// * `None` - Stack is empty
    pub fn pop(&mut self) -> Option<ExtractionItem> {
        self.items.pop()
    }

    /// Check if the stack is empty
    ///
    /// # Returns
    ///
    /// true if no items remain on the stack
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the current stack size
    ///
    /// # Returns
    ///
    /// Number of items currently on the stack
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Get the maximum allowed stack size
    ///
    /// # Returns
    ///
    /// Maximum stack size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Clear all items from the stack
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl Default for ExtractionStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_context_new() {
        let ctx = ExtractionContext::new("workspace_123".to_string());

        assert_eq!(ctx.workspace_id, "workspace_123");
        assert_eq!(ctx.current_depth, 0);
        assert!(ctx.parent_archive.is_none());
        assert_eq!(ctx.accumulated_size, 0);
        assert_eq!(ctx.accumulated_files, 0);
    }

    #[test]
    fn test_extraction_context_create_child() {
        let parent_ctx = ExtractionContext::new("workspace_123".to_string());
        let parent_archive = PathBuf::from("/path/to/parent.zip");

        let child_ctx = parent_ctx.create_child(parent_archive.clone());

        assert_eq!(child_ctx.workspace_id, "workspace_123");
        assert_eq!(child_ctx.current_depth, 1);
        assert_eq!(child_ctx.parent_archive, Some(parent_archive));
        assert_eq!(child_ctx.accumulated_size, 0);
        assert_eq!(child_ctx.accumulated_files, 0);
    }

    #[test]
    fn test_extraction_context_update_metrics() {
        let mut ctx = ExtractionContext::new("workspace_123".to_string());

        ctx.update_metrics(1024, 5);
        assert_eq!(ctx.accumulated_size, 1024);
        assert_eq!(ctx.accumulated_files, 5);

        ctx.update_metrics(2048, 3);
        assert_eq!(ctx.accumulated_size, 3072);
        assert_eq!(ctx.accumulated_files, 8);
    }

    #[test]
    fn test_extraction_context_depth_limit() {
        let mut ctx = ExtractionContext::new("workspace_123".to_string());

        assert!(!ctx.is_depth_limit_reached(10));

        ctx.current_depth = 9;
        assert!(!ctx.is_depth_limit_reached(10));

        ctx.current_depth = 10;
        assert!(ctx.is_depth_limit_reached(10));

        ctx.current_depth = 11;
        assert!(ctx.is_depth_limit_reached(10));
    }

    #[test]
    fn test_extraction_stack_push_pop() {
        let mut stack = ExtractionStack::new();
        let ctx = ExtractionContext::new("workspace_123".to_string());

        let item1 = ExtractionItem::new(
            PathBuf::from("/path/to/archive1.zip"),
            PathBuf::from("/target1"),
            0,
            ctx.clone(),
        );

        let item2 = ExtractionItem::new(
            PathBuf::from("/path/to/archive2.zip"),
            PathBuf::from("/target2"),
            1,
            ctx.clone(),
        );

        assert!(stack.push(item1).is_ok());
        assert!(stack.push(item2).is_ok());
        assert_eq!(stack.len(), 2);

        let popped = stack.pop().unwrap();
        assert_eq!(popped.archive_path, PathBuf::from("/path/to/archive2.zip"));
        assert_eq!(stack.len(), 1);

        let popped = stack.pop().unwrap();
        assert_eq!(popped.archive_path, PathBuf::from("/path/to/archive1.zip"));
        assert_eq!(stack.len(), 0);

        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_extraction_stack_max_size() {
        let mut stack = ExtractionStack::with_max_size(2);
        let ctx = ExtractionContext::new("workspace_123".to_string());

        let item1 = ExtractionItem::new(
            PathBuf::from("/path/to/archive1.zip"),
            PathBuf::from("/target1"),
            0,
            ctx.clone(),
        );

        let item2 = ExtractionItem::new(
            PathBuf::from("/path/to/archive2.zip"),
            PathBuf::from("/target2"),
            1,
            ctx.clone(),
        );

        let item3 = ExtractionItem::new(
            PathBuf::from("/path/to/archive3.zip"),
            PathBuf::from("/target3"),
            2,
            ctx.clone(),
        );

        assert!(stack.push(item1).is_ok());
        assert!(stack.push(item2).is_ok());

        // Third push should fail due to max_size limit
        let result = stack.push(item3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Stack size limit exceeded"));
    }

    #[test]
    fn test_extraction_stack_clear() {
        let mut stack = ExtractionStack::new();
        let ctx = ExtractionContext::new("workspace_123".to_string());

        let item = ExtractionItem::new(
            PathBuf::from("/path/to/archive.zip"),
            PathBuf::from("/target"),
            0,
            ctx,
        );

        stack.push(item).unwrap();
        assert_eq!(stack.len(), 1);

        stack.clear();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_extraction_stack_is_empty() {
        let mut stack = ExtractionStack::new();
        assert!(stack.is_empty());

        let ctx = ExtractionContext::new("workspace_123".to_string());
        let item = ExtractionItem::new(
            PathBuf::from("/path/to/archive.zip"),
            PathBuf::from("/target"),
            0,
            ctx,
        );

        stack.push(item).unwrap();
        assert!(!stack.is_empty());

        stack.pop();
        assert!(stack.is_empty());
    }
}
