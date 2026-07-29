//! 搜索查询解析与构建

use la_core::error::CommandError;
use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use la_core::models::SearchQuery;

use crate::services::looks_like_regex_pattern;

#[derive(Clone, Copy)]
enum QueryScope {
    Group(char),
    CharacterClass {
        allows_literal_closing_bracket: bool,
    },
}

pub(crate) fn split_query_by_pipe(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut scopes = Vec::new();
    let mut escaped = false;
    for ch in query.chars() {
        if escaped {
            if ch == '|' {
                current.push('|');
            } else {
                current.push('\\');
                current.push(ch);
            }
            if let Some(QueryScope::CharacterClass {
                allows_literal_closing_bracket,
            }) = scopes.last_mut()
            {
                *allows_literal_closing_bracket = false;
            }
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }

        if let Some(QueryScope::CharacterClass {
            allows_literal_closing_bracket,
        }) = scopes.last_mut()
        {
            current.push(ch);
            match ch {
                '[' => {
                    *allows_literal_closing_bracket = false;
                    scopes.push(QueryScope::CharacterClass {
                        allows_literal_closing_bracket: true,
                    });
                }
                ']' if *allows_literal_closing_bracket => {
                    *allows_literal_closing_bracket = false;
                }
                ']' => {
                    scopes.pop();
                }
                '^' if *allows_literal_closing_bracket => {}
                _ => *allows_literal_closing_bracket = false,
            }
            continue;
        }

        if ch == '[' {
            scopes.push(QueryScope::CharacterClass {
                allows_literal_closing_bracket: true,
            });
            current.push(ch);
            continue;
        }
        if let Some(expected_closer) = match ch {
            '(' => Some(')'),
            '{' => Some('}'),
            _ => None,
        } {
            scopes.push(QueryScope::Group(expected_closer));
            current.push(ch);
            continue;
        }
        if matches!(ch, ')' | ']' | '}') {
            if matches!(scopes.last(), Some(QueryScope::Group(closer)) if *closer == ch) {
                scopes.pop();
            }
            current.push(ch);
            continue;
        }
        if ch == '|' && scopes.is_empty() {
            let t = current.trim();
            if !t.is_empty() {
                terms.push(t.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }
    if escaped {
        current.push('\\');
    }
    let t = current.trim();
    if !t.is_empty() {
        terms.push(t.to_string());
    }
    terms
}

pub(crate) fn build_structured_search_query(
    query: &str,
    case_sensitive: bool,
    query_id: &str,
) -> Result<(Vec<String>, SearchQuery), CommandError> {
    let raw_terms = split_query_by_pipe(query);
    if raw_terms.is_empty() {
        return Err(
            CommandError::new("VALIDATION_ERROR", "Search query cannot be empty")
                .with_help("Please enter at least one search term"),
        );
    }
    let terms = raw_terms
        .iter()
        .enumerate()
        .map(|(i, v)| SearchTerm {
            id: format!("term_{i}"),
            value: v.clone(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            is_regex: looks_like_regex_pattern(v),
            priority: 1,
            enabled: true,
            case_sensitive,
        })
        .collect();
    Ok((
        raw_terms,
        SearchQuery {
            id: query_id.to_string(),
            terms,
            global_operator: QueryOperator::Or,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        },
    ))
}

pub(crate) fn resolve_search_query(
    query: &str,
    structured_query: Option<SearchQuery>,
    case_sensitive: bool,
    query_id: &str,
) -> Result<(Vec<String>, SearchQuery), CommandError> {
    if let Some(mut sq) = structured_query {
        let raw: Vec<String> = sq
            .terms
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.value.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if raw.is_empty() {
            return Err(
                CommandError::new("VALIDATION_ERROR", "Search query cannot be empty")
                    .with_help("Please enter at least one search term"),
            );
        }
        sq.id = query_id.to_string();
        sq.metadata = QueryMetadata {
            created_at: 0,
            last_modified: 0,
            execution_count: 0,
            label: None,
        };
        return Ok((raw, sq));
    }
    build_structured_search_query(query, case_sensitive, query_id)
}

#[cfg(test)]
mod tests {
    use super::build_structured_search_query;

    #[test]
    fn character_class_pipe_is_not_treated_as_a_query_separator() {
        let (raw_terms, structured_query) =
            build_structured_search_query("[)|] | timeout", false, "search-1")
                .expect("valid search query should parse");

        assert_eq!(raw_terms, vec!["[)|]", "timeout"]);
        assert_eq!(
            structured_query
                .terms
                .iter()
                .map(|term| term.value.as_str())
                .collect::<Vec<_>>(),
            vec!["[)|]", "timeout"]
        );
    }

    #[test]
    fn nested_character_class_pipe_is_not_treated_as_a_query_separator() {
        let (raw_terms, _) =
            build_structured_search_query("[[:alpha:]|_] | timeout", false, "search-2")
                .expect("valid search query should parse");

        assert_eq!(raw_terms, vec!["[[:alpha:]|_]", "timeout"]);
    }

    #[test]
    fn leading_closing_bracket_in_character_class_is_treated_as_a_literal() {
        let (raw_terms, _) = build_structured_search_query("[]|] | timeout", false, "search-3")
            .expect("valid search query should parse");

        assert_eq!(raw_terms, vec!["[]|]", "timeout"]);
    }

    #[test]
    fn leading_closing_bracket_after_negation_is_treated_as_a_literal() {
        let (raw_terms, _) = build_structured_search_query("[^]|] | timeout", false, "search-4")
            .expect("valid search query should parse");

        assert_eq!(raw_terms, vec!["[^]|]", "timeout"]);
    }
}
