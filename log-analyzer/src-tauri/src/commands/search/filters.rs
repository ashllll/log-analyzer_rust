//! 搜索过滤器类型与匹配逻辑

use crate::commands::level_to_mask;
use crate::services::file_watcher::TimestampParser;
use crate::services::parse_metadata;
use la_core::error::CommandError;
use la_core::models::SearchFilters;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(crate) struct CompiledSearchFilters {
    pub(crate) levels: Option<HashSet<String>>,
    pub(crate) level_mask: Option<u8>,
    pub(crate) time_start: Option<chrono::NaiveDateTime>,
    pub(crate) time_end: Option<chrono::NaiveDateTime>,
    pub(crate) file_matcher: Option<FilePatternMatcher>,
    pub(crate) database_file_pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedLineMetadata {
    pub(crate) timestamp: String,
    pub(crate) level: &'static str,
    pub(crate) level_normalized: &'static str,
    pub(crate) datetime: Option<chrono::NaiveDateTime>,
    pub(crate) level_mask: u8,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SearchSegmentSummary {
    pub(crate) min_datetime: Option<chrono::NaiveDateTime>,
    pub(crate) max_datetime: Option<chrono::NaiveDateTime>,
    pub(crate) level_mask: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchLineCandidate<'a> {
    pub(crate) index: usize,
    pub(crate) line: std::borrow::Cow<'a, str>,
    pub(crate) metadata: ParsedLineMetadata,
}

#[derive(Debug, Clone)]
pub(crate) enum FilePatternMatcher {
    Substring(String),
    Wildcard(Regex),
}

fn escape_sqlite_glob_literal(value: &str) -> String {
    let mut e = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '*' => e.push_str("[*]"),
            '?' => e.push_str("[?]"),
            '[' => e.push_str("[[]"),
            ']' => e.push_str("[]]"),
            _ => e.push(ch),
        }
    }
    e
}

impl ParsedLineMetadata {
    pub(crate) fn parse(line: &str, needs_datetime: bool) -> Self {
        let (timestamp, level) = parse_metadata(line);
        let datetime = if needs_datetime {
            TimestampParser::parse_naive_datetime(&timestamp)
        } else {
            None
        };
        Self {
            timestamp,
            level,
            level_normalized: level,
            datetime,
            level_mask: level_to_mask(level),
        }
    }
}

impl SearchSegmentSummary {
    pub(crate) fn record(&mut self, m: &ParsedLineMetadata) {
        self.level_mask |= m.level_mask;
        if let Some(dt) = m.datetime {
            self.min_datetime = Some(self.min_datetime.map_or(dt, |c| c.min(dt)));
            self.max_datetime = Some(self.max_datetime.map_or(dt, |c| c.max(dt)));
        }
    }
}

impl FilePatternMatcher {
    pub(crate) fn compile(raw: &str) -> Result<Self, CommandError> {
        let t = raw.trim();
        if t.contains('*') || t.contains('?') {
            let escaped = regex::escape(t);
            let re = format!("^{}$", escaped.replace(r"\*", ".*").replace(r"\?", "."));
            Ok(Self::Wildcard(Regex::new(&re).map_err(|e| {
                CommandError::new(
                    "VALIDATION_ERROR",
                    format!("Invalid file pattern '{}': {}", t, e),
                )
                .with_help("Use '*.log' or 'service-error.log'")
            })?))
        } else {
            Ok(Self::Substring(t.to_string()))
        }
    }
    pub(crate) fn matches(&self, value: &str) -> bool {
        match self {
            Self::Substring(p) => value.contains(p),
            Self::Wildcard(r) => r.is_match(value),
        }
    }
}

impl CompiledSearchFilters {
    pub(crate) fn compile(filters: &SearchFilters) -> Result<Self, CommandError> {
        let levels = if filters.levels.is_empty() {
            None
        } else {
            Some(
                filters
                    .levels
                    .iter()
                    .map(|l| l.trim().to_ascii_lowercase())
                    .filter(|l| !l.is_empty())
                    .collect::<HashSet<_>>(),
            )
        }
        .filter(|l| !l.is_empty());
        let level_mask = levels
            .as_ref()
            .map(|l| l.iter().fold(0u8, |m, l| m | level_to_mask(l)));
        let time_start = Self::parse_dt(filters.time_start.as_deref(), "start time")?;
        let time_end = Self::parse_dt(filters.time_end.as_deref(), "end time")?;
        if let (Some(s), Some(e)) = (time_start, time_end) {
            if s > e {
                return Err(CommandError::new(
                    "VALIDATION_ERROR",
                    "Start time cannot be later than end time",
                )
                .with_help("Adjust the time range"));
            }
        }
        let file_matcher = filters
            .file_pattern
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(FilePatternMatcher::compile)
            .transpose()?;
        let db_pattern = filters
            .file_pattern
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(Self::build_db_pattern);
        Ok(Self {
            levels,
            level_mask,
            time_start,
            time_end,
            file_matcher,
            database_file_pattern: db_pattern,
        })
    }
    fn build_db_pattern(p: &str) -> String {
        if p.contains('*') || p.contains('?') {
            escape_sqlite_glob_literal(p)
                .replace("[*]", "*")
                .replace("[?]", "?")
        } else {
            format!("*{}*", escape_sqlite_glob_literal(p))
        }
    }
    fn parse_dt(
        v: Option<&str>,
        label: &str,
    ) -> Result<Option<chrono::NaiveDateTime>, CommandError> {
        let Some(v) = v.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(None);
        };
        TimestampParser::parse_naive_datetime(v)
            .ok_or_else(|| {
                CommandError::new("VALIDATION_ERROR", format!("Invalid {} '{}'", label, v))
                    .with_help("Use '2024-01-15T10:30' or '2024-01-15 10:30:45'")
            })
            .map(Some)
    }
    pub(crate) fn matches_file(&self, vpath: &str, rpath: Option<&str>) -> bool {
        let Some(m) = &self.file_matcher else {
            return true;
        };
        m.matches(vpath) || rpath.is_some_and(|p| m.matches(p))
    }
    pub(crate) fn database_file_pattern(&self) -> Option<String> {
        self.database_file_pattern.clone()
    }
    pub(crate) fn matches_parsed_line_metadata(&self, m: &ParsedLineMetadata) -> bool {
        if let Some(lv) = &self.levels {
            if !lv.contains(m.level_normalized) {
                return false;
            }
        }
        if !self.has_time_filter() {
            return true;
        }
        let Some(dt) = m.datetime else { return false };
        if let Some(s) = self.time_start {
            if dt < s {
                return false;
            }
        }
        if let Some(e) = self.time_end {
            if dt > e {
                return false;
            }
        }
        true
    }
    pub(crate) fn has_time_filter(&self) -> bool {
        self.time_start.is_some() || self.time_end.is_some()
    }
    pub(crate) fn needs_segment_pruning(&self) -> bool {
        self.levels.is_some() || self.has_time_filter()
    }
    #[allow(clippy::if_same_then_else)]
    pub(crate) fn segment_may_match(&self, s: &SearchSegmentSummary) -> bool {
        if let Some(lv) = &self.levels {
            if self.level_mask.unwrap_or(0) == 0 && !lv.is_empty() {
                return false;
            } else if s.level_mask & self.level_mask.unwrap_or(0) == 0 {
                return false;
            }
        }
        if !self.has_time_filter() {
            return true;
        }
        let (Some(mn), Some(mx)) = (s.min_datetime, s.max_datetime) else {
            return true;
        };
        if let Some(st) = self.time_start {
            if mx < st {
                return false;
            }
        }
        if let Some(ed) = self.time_end {
            if mn > ed {
                return false;
            }
        }
        true
    }
}
