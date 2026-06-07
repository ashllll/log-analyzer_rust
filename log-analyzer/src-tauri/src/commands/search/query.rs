//! 搜索查询解析与构建

use la_core::error::CommandError;
use la_core::models::search::{QueryMetadata, QueryOperator, SearchTerm, TermSource};
use la_core::models::SearchQuery;

use crate::services::looks_like_regex_pattern;

pub(crate) fn split_query_by_pipe(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    let mut escaped = false;
    for ch in query.chars() {
        if escaped {
            if ch == '|' {
                current.push('|');
            } else {
                current.push('\\');
                current.push(ch);
            }
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '(' || ch == '[' || ch == '{' {
            depth += 1;
            current.push(ch);
            continue;
        }
        if ch == ')' || ch == ']' || ch == '}' {
            if depth > 0 {
                depth -= 1;
            }
            current.push(ch);
            continue;
        }
        if ch == '|' && depth == 0 {
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
