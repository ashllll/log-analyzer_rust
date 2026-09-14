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

pub(crate) fn split_query_by_pipe(query: &str) -> Vec<(String, bool)> {
    let mut terms: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut current_has_escaped_pipe = false;
    let mut scopes = Vec::new();
    let mut escaped = false;
    for ch in query.chars() {
        if escaped {
            if ch == '|' {
                // 转义管道：保留字面量 `|` 并记录标志，供 is_regex
                // 判定把该词项视为字面量（而非正则交替）。
                current.push('|');
                current_has_escaped_pipe = true;
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
                terms.push((t.to_string(), current_has_escaped_pipe));
            }
            current.clear();
            current_has_escaped_pipe = false;
            continue;
        }
        current.push(ch);
    }
    if escaped {
        current.push('\\');
    }
    let t = current.trim();
    if !t.is_empty() {
        terms.push((t.to_string(), current_has_escaped_pipe));
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
        .map(|(i, (v, contains_escaped_pipe))| SearchTerm {
            id: format!("term_{i}"),
            value: v.clone(),
            operator: QueryOperator::Or,
            source: TermSource::User,
            preset_group_id: None,
            // 转义管道（\\|）是"字面量管道"的明确意图：词项按字面量
            // 处理，绝不进入正则交替语义（否则 `foo\\|` 的空分支会匹配
            // 所有行）。
            is_regex: !*contains_escaped_pipe && looks_like_regex_pattern(v),
            priority: 1,
            enabled: true,
            case_sensitive,
        })
        .collect();
    let raw_term_strings: Vec<String> = raw_terms.iter().map(|(v, _)| v.clone()).collect();
    Ok((
        raw_term_strings,
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
    use la_core::models::QueryOperator;

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
    // ── P2#4 regression: escaped pipe must stay a literal term ──

    #[test]
    fn escaped_pipe_term_is_never_treated_as_regex() {
        let (_raw, sq) = build_structured_search_query(r"timeout\|retry", false, "search-5")
            .expect("escaped pipe query should parse");

        assert_eq!(sq.terms.len(), 1);
        assert_eq!(sq.terms[0].value, "timeout|retry");
        assert!(
            !sq.terms[0].is_regex,
            "escaped pipe is a literal, not a regex alternation"
        );
    }

    #[test]
    fn trailing_escaped_pipe_never_matches_every_line() {
        // `timeout\|` 若被当正则 `timeout|`，空分支会匹配所有行。
        let (_raw, sq) = build_structured_search_query(r"timeout\|", false, "search-6")
            .expect("escaped pipe query should parse");

        assert_eq!(sq.terms.len(), 1);
        assert_eq!(sq.terms[0].value, "timeout|");
        assert!(!sq.terms[0].is_regex);
    }

    #[test]
    fn unescaped_pipe_still_splits_into_or_terms() {
        // 未转义管道是词项分隔符：两个词项交给 query_planner 的
        // fast-OR（Aho-Corasick 多模式）组合，而非词项内正则交替。
        let (_raw, sq) = build_structured_search_query("timeout|retry", false, "search-7")
            .expect("pipe query should parse");

        assert_eq!(sq.terms.len(), 2);
        assert_eq!(sq.global_operator, QueryOperator::Or);
    }

    #[test]
    fn escaped_pipe_does_not_affect_plain_regex_terms() {
        let (_raw, sq) = build_structured_search_query(r"error.*timeout|warn", false, "search-8")
            .expect("mixed query should parse");

        assert_eq!(sq.terms.len(), 2);
        // `error.*timeout` 含 `*` → 正则；`warn` 是字面量词项。
        assert!(sq.terms[0].is_regex);
        assert!(!sq.terms[1].is_regex);
    }
}
