use crate::error::{AppError, Result};
use crate::models::search::*;
use regex::Regex;

/**
 * 查询验证器
 *
 * 负责验证搜索查询的有效性和正确性
 */
pub struct QueryValidator;

impl QueryValidator {
    /**
     * 验证查询
     *
     * # 参数
     * * `query` - 要验证的搜索查询
     *
     * # 返回
     * * `Ok(())` - 如果查询有效
     * * `Err(AppError)` - 如果查询无效
     */
    pub fn validate(query: &SearchQuery) -> Result<()> {
        if query.terms.is_empty() {
            return Err(AppError::validation_error("Query is empty"));
        }

        let enabled_terms: Vec<_> = query.terms.iter().filter(|t| t.enabled).collect();

        if enabled_terms.is_empty() {
            return Err(AppError::validation_error("No enabled terms"));
        }

        // 验证每个项
        for term in &enabled_terms {
            Self::validate_term(term)?;
        }

        Ok(())
    }

    /**
     * 验证单个搜索项
     *
     * # 参数
     * * `term` - 要验证的搜索项
     *
     * # 返回
     * * `Ok(())` - 如果项有效
     * * `Err(AppError)` - 如果项无效
     */
    fn validate_term(term: &SearchTerm) -> Result<()> {
        if term.value.is_empty() {
            return Err(AppError::validation_error(format!(
                "Term {} has empty value",
                term.id
            )));
        }

        if term.value.len() > 100 {
            return Err(AppError::validation_error(format!(
                "Term {} value is too long",
                term.id
            )));
        }

        if term.is_regex {
            Regex::new(&term.value).map_err(|e| {
                AppError::validation_error(format!("Term {} has invalid regex: {}", term.id, e))
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::search::{QueryMetadata, TermSource};

    fn create_test_term(value: &str, enabled: bool) -> SearchTerm {
        SearchTerm {
            id: "test".to_string(),
            value: value.to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled,
            case_sensitive: false,
        }
    }

    #[test]
    fn test_validate_empty_query() {
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(result.is_err(), "Expected validation error for empty query");

        let error = result.unwrap_err();
        if let AppError::Validation(msg) = &error {
            assert!(
                msg.contains("empty"),
                "Expected error message containing 'empty', got: {}",
                msg
            );
        } else {
            panic!("Expected Validation error, got: {:?}", error);
        }
    }

    #[test]
    fn test_validate_no_enabled_terms() {
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![create_test_term("error", false)],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(
            result.is_err(),
            "Expected validation error for no enabled terms"
        );

        let error = result.unwrap_err();
        if let AppError::Validation(msg) = &error {
            assert!(
                msg.contains("No enabled terms"),
                "Expected error message containing 'No enabled terms', got: {}",
                msg
            );
        } else {
            panic!("Expected Validation error, got: {:?}", error);
        }
    }

    #[test]
    fn test_validate_empty_term_value() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: "".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: false,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(
            result.is_err(),
            "Expected validation error for empty term value"
        );

        let error = result.unwrap_err();
        if let AppError::Validation(msg) = &error {
            assert!(
                msg.contains("empty value"),
                "Expected error message containing 'empty value', got: {}",
                msg
            );
        } else {
            panic!("Expected Validation error, got: {:?}", error);
        }
    }

    #[test]
    fn test_validate_term_value_too_long() {
        let long_value = "a".repeat(101);
        let term = create_test_term(&long_value, true);

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(
            result.is_err(),
            "Expected validation error for term value too long"
        );

        let error = result.unwrap_err();
        if let AppError::Validation(msg) = &error {
            assert!(
                msg.contains("too long"),
                "Expected error message containing 'too long', got: {}",
                msg
            );
        } else {
            panic!("Expected Validation error, got: {:?}", error);
        }
    }

    #[test]
    fn test_validate_invalid_regex() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: "[invalid".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: true,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(
            result.is_err(),
            "Expected validation error for invalid regex"
        );

        let error = result.unwrap_err();
        if let AppError::Validation(msg) = &error {
            assert!(
                msg.contains("invalid regex"),
                "Expected error message containing 'invalid regex', got: {}",
                msg
            );
        } else {
            panic!("Expected Validation error, got: {:?}", error);
        }
    }

    #[test]
    fn test_validate_valid_query() {
        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![
                create_test_term("error", true),
                create_test_term("timeout", true),
            ],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_valid_regex() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: r"\d+".to_string(),
            operator: QueryOperator::And,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: true,
            priority: 1,
            enabled: true,
            case_sensitive: false,
        };

        let query = SearchQuery {
            id: "test".to_string(),
            terms: vec![term],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let result = QueryValidator::validate(&query);
        assert!(result.is_ok());
    }
}
