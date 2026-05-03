use crate::services::traits::{QueryValidation, ValidationResult};
use la_core::error::{AppError, Result};
use la_core::models::search::*;
use regex::Regex;

/**
 * 查询验证器
 *
 * 负责验证搜索查询的有效性和正确性
 */
pub struct QueryValidator;

impl Default for QueryValidator {
    fn default() -> Self {
        Self
    }
}

impl QueryValidation for QueryValidator {
    fn validate(&self, query: &SearchQuery) -> ValidationResult {
        match Self::validate(query) {
            Ok(()) => ValidationResult::valid(),
            Err(e) => ValidationResult::error(format!("{}", e)),
        }
    }
}

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

        // 使用字符数（而非字节数）检查长度，避免多字节 UTF-8（如中文）被提前截断
        if term.value.chars().count() > 100 {
            return Err(AppError::validation_error(format!(
                "Term {} value is too long",
                term.id
            )));
        }

        if term.is_regex {
            // Note: look-around assertions are supported by FancyEngine (fancy-regex),
            // so we no longer reject them here. RegexEngine::new automatically routes
            // look-around patterns to FancyEngine.

            if contains_backreference(&term.value) {
                return Err(AppError::validation_error(format!(
                    "Term {} uses regex syntax not supported by Rust regex: backreferences are not supported",
                    term.id
                )));
            }

            // Validate with the appropriate regex engine:
            // - look-around patterns => fancy-regex
            // - others => regex crate
            let has_lookaround = term.value.contains("(?=")
                || term.value.contains("(?!")
                || term.value.contains("(?<=")
                || term.value.contains("(?<!");
            if has_lookaround {
                // Check for ReDoS patterns before compiling with fancy-regex
                // fancy-regex uses a backtracking engine for look-around patterns,
                // which can suffer from catastrophic backtracking
                check_redos_risk(&term.value)?;
                fancy_regex::Regex::new(&term.value).map_err(|e| {
                    AppError::validation_error(format!("Term {} has invalid regex: {}", term.id, e))
                })?;
            } else {
                Regex::new(&term.value).map_err(|e| {
                    AppError::validation_error(format!("Term {} has invalid regex: {}", term.id, e))
                })?;
            }
        }

        Ok(())
    }
}

fn contains_backreference(pattern: &str) -> bool {
    let chars: Vec<char> = pattern.chars().collect();

    for (index, ch) in chars.iter().enumerate() {
        if *ch != '\\' {
            continue;
        }

        let preceding_backslashes = chars[..=index]
            .iter()
            .rev()
            .take_while(|candidate| **candidate == '\\')
            .count();

        if preceding_backslashes % 2 == 0 {
            continue;
        }

        match chars.get(index + 1) {
            Some(next) if *next >= '1' && *next <= '9' => return true,
            Some('k') if matches!(chars.get(index + 2), Some('<') | Some('\'')) => return true,
            _ => {}
        }
    }

    pattern.contains("(?P=")
}

/// Check for ReDoS (Regular Expression Denial of Service) risk patterns.
///
/// fancy_regex uses a backtracking engine for look-around assertions,
/// which can suffer from catastrophic backtracking with exponential runtime.
/// This function detects common dangerous patterns:
/// - Nested quantifiers: (a+)+, (a*)*, (a+)*, (a*)+
/// - Overlapping alternation with quantifiers: (a|aa)+
fn check_redos_risk(pattern: &str) -> Result<()> {
    // Check for nested quantifiers which are the most common ReDoS vector.
    // Pattern: a group ending with +, *, or {n,} followed by another quantifier.
    // We use a simple structural check to catch the common cases.
    if has_nested_quantifiers(pattern) {
        return Err(AppError::validation_error(
            "Regex contains nested quantifiers (e.g., (a+)+) which may cause excessive backtracking"
        ));
    }

    // Check for alternation with overlapping branches under quantifier.
    // Pattern like (a|aa)+ can cause exponential backtracking.
    if has_overlapping_alternation(pattern) {
        return Err(AppError::validation_error(
            "Regex contains overlapping alternation under quantifier which may cause excessive backtracking"
        ));
    }

    Ok(())
}

/// Detect nested quantifiers like (X+)+, (X*)*, (X+)*, (X*)+
/// This is a conservative heuristic that checks for the common structural patterns.
fn has_nested_quantifiers(pattern: &str) -> bool {
    // Use a simple regex to detect patterns like:
    // )+, )*, )}+, )}* — a closing group followed immediately by a quantifier
    let nested_pattern = regex::Regex::new(r"\)[\+\*\?]|\}[\,\d]*[\+\*\?]").unwrap();
    if !nested_pattern.is_match(pattern) {
        return false;
    }

    // More targeted check: look for things like (.+)+ specifically
    // where immediate re-application of a quantifier creates exponential state
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ')' {
            // Check if this closing paren is followed by a quantifier
            if i + 1 < chars.len() && matches!(chars[i + 1], '+' | '*' | '{') {
                // Walk backward to find matching opening paren
                let mut depth = 1;
                let mut j = i as isize - 1;
                while j >= 0 && depth > 0 {
                    if chars[j as usize] == ')' {
                        depth += 1;
                    } else if chars[j as usize] == '(' {
                        depth -= 1;
                        if depth == 0 {
                            // Check if the group itself contains quantifiers
                            // Look for + or * inside the group's body
                            let body = &chars[j as usize + 1..i];
                            let body_str: String = body.iter().collect();
                            if body_str.contains('+') || body_str.contains('*') {
                                return true;
                            }
                        }
                    }
                    j -= 1;
                }
            }
        }
        i += 1;
    }
    false
}

/// Detect overlapping alternation patterns like (a|aa)+
fn has_overlapping_alternation(pattern: &str) -> bool {
    // Simple check: find alternation groups with quantifiers where branches
    // could overlap (one branch is a substring/superset of another)
    // This is a simplified heuristic; full analysis requires DFA construction
    let alternation_re = regex::Regex::new(r"\([^()]*\|[^()]*\)[\+\*]").unwrap();
    alternation_re.is_match(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_core::models::search::{QueryMetadata, TermSource};

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

    #[test]
    fn test_validate_accepts_lookaround() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: "(?=foo)bar".to_string(),
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

        // Look-around assertions are now accepted because FancyEngine supports them
        assert!(QueryValidator::validate(&query).is_ok());
    }

    #[test]
    fn test_validate_rejects_backreference() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: r"(foo)\1".to_string(),
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

        let error = QueryValidator::validate(&query).unwrap_err();
        assert!(error.to_string().contains("backreferences"));
    }

    /// ReDoS protection: nested quantifiers should be rejected
    #[test]
    fn test_redos_rejects_nested_quantifiers() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: "(?=a)(a+)+b".to_string(),
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
        assert!(result.is_err(), "Nested quantifiers should be rejected");
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("nested quantifiers"),
            "Error should mention nested quantifiers, got: {}",
            error
        );
    }

    /// ReDoS protection: safe look-around patterns should still be accepted
    #[test]
    fn test_redos_accepts_safe_lookaround() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: r"(?<=error)\s+\d+".to_string(),
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

        assert!(
            QueryValidator::validate(&query).is_ok(),
            "Safe look-around should be accepted"
        );
    }

    /// ReDoS protection: overlapping alternation should be rejected
    #[test]
    fn test_redos_rejects_overlapping_alternation() {
        let term = SearchTerm {
            id: "term1".to_string(),
            value: "(?=x)(a|aa)+".to_string(),
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
            "Overlapping alternation should be rejected"
        );
    }
}
