//! Tantivy Schema Definition for Log Entries
//!
//! Defines the search index schema optimized for log analysis with:
//! - Full-text searchable content
//! - Fast range queries on timestamps
//! - Faceted search on log levels
//! - Hierarchical filtering on file paths

use tantivy::schema::{
    Field, IndexRecordOption, NumericOptions, Schema, TextFieldIndexing, TextOptions,
};
use tantivy::tokenizer::TextAnalyzer;

/// Schema definition for log entries in the search index
#[derive(Clone, Debug)]
pub struct LogSchema {
    pub schema: Schema,
    pub content: Field,     // Full-text searchable content
    pub timestamp: Field,   // Fast range queries (i64 unix timestamp)
    pub level: Field,       // Faceted search (stored as string)
    pub file_path: Field,   // Hierarchical filtering (virtual path)
    pub real_path: Field,   // Real file system path
    pub line_number: Field, // Precise location (u64)
}

impl LogSchema {
    /// Build the optimized schema for log entries
    pub fn build() -> Self {
        let mut schema_builder = Schema::builder();

        // Content field: Full-text searchable with positions for highlighting
        let content = schema_builder.add_text_field(
            "content",
            TextOptions::default()
                .set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer("en_stem")
                        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                )
                .set_stored(),
        );

        // Timestamp field: Fast range queries and sorting
        let timestamp = schema_builder.add_i64_field(
            "timestamp",
            NumericOptions::default()
                .set_fast() // Enable fast field for range queries
                .set_stored(),
        );

        // Level field: Faceted search (ERROR, WARN, INFO, DEBUG, etc.)
        let level = schema_builder.add_text_field(
            "level",
            TextOptions::default()
                .set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer("raw") // No tokenization for exact matching
                        .set_index_option(IndexRecordOption::Basic),
                )
                .set_fast(Some("raw")) // Enable fast field for faceting
                .set_stored(),
        );

        // File path field: Hierarchical filtering on virtual paths
        let file_path = schema_builder.add_text_field(
            "file_path",
            TextOptions::default()
                .set_indexing_options(
                    TextFieldIndexing::default()
                        .set_tokenizer("raw")
                        .set_index_option(IndexRecordOption::Basic),
                )
                .set_fast(Some("raw"))
                .set_stored(),
        );

        // Real path field: For result display and file operations
        let real_path = schema_builder.add_text_field(
            "real_path",
            TextOptions::default().set_stored(), // Only stored, not indexed
        );

        // Line number field: Precise location within file
        let line_number = schema_builder.add_u64_field(
            "line_number",
            NumericOptions::default().set_fast().set_stored(),
        );

        Self {
            schema: schema_builder.build(),
            content,
            timestamp,
            level,
            file_path,
            real_path,
            line_number,
        }
    }

    /// Get the schema reference
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Configure custom tokenizers for the schema
    pub fn configure_tokenizers(&self, index: &tantivy::Index) -> tantivy::Result<()> {
        let tokenizer_manager = index.tokenizers();

        // Register stemming tokenizer for content analysis
        tokenizer_manager.register(
            "en_stem",
            TextAnalyzer::builder(tantivy::tokenizer::SimpleTokenizer::default())
                .filter(tantivy::tokenizer::RemoveLongFilter::limit(40))
                .filter(tantivy::tokenizer::LowerCaser)
                .filter(tantivy::tokenizer::Stemmer::default())
                .build(),
        );

        // Register raw tokenizer for exact matching
        tokenizer_manager.register(
            "raw",
            TextAnalyzer::builder(tantivy::tokenizer::RawTokenizer::default()).build(),
        );

        Ok(())
    }
}

impl Default for LogSchema {
    fn default() -> Self {
        Self::build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::Index;

    #[test]
    fn test_schema_creation() {
        let log_schema = LogSchema::build();

        // Verify all fields are present
        assert!(log_schema.schema.get_field("content").is_ok());
        assert!(log_schema.schema.get_field("timestamp").is_ok());
        assert!(log_schema.schema.get_field("level").is_ok());
        assert!(log_schema.schema.get_field("file_path").is_ok());
        assert!(log_schema.schema.get_field("real_path").is_ok());
        assert!(log_schema.schema.get_field("line_number").is_ok());
    }

    #[test]
    fn test_tokenizer_configuration() {
        let log_schema = LogSchema::build();
        let index = Index::create_in_ram(log_schema.schema.clone());

        // Should not panic when configuring tokenizers
        assert!(log_schema.configure_tokenizers(&index).is_ok());
    }
}
