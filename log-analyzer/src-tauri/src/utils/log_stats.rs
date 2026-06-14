//! Log file statistics computation — pure parsing, no infrastructure dependencies.
//!
//! Extracted from `infrastructure/workspace_service_impl.rs` since this
//! function performs only string parsing and aggregation (charset detection,
//! timestamp extraction, log level mask computation) with zero calls to
//! filesystem, database, or network.

/// Compute file-level statistics from raw log content.
///
/// Parses the content to extract:
/// - `min_timestamp`: earliest Unix timestamp found
/// - `max_timestamp`: latest Unix timestamp found
/// - `level_mask`: bitmask of log levels present
///
/// For files > 10MB, only the first and last 1000 lines are sampled
/// to avoid excessive CPU usage on very large files.
///
/// # Arguments
/// * `content` — raw file bytes
///
/// # Returns
/// `(min_timestamp, max_timestamp, level_mask)`
pub fn compute_file_stats(content: &[u8]) -> (Option<i64>, Option<i64>, Option<u8>) {
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => return (None, None, None),
    };

    // 大文件优化：超过 10MB 只解析前 1000 行和后 1000 行
    const MAX_FULL_PARSE_BYTES: usize = 10 * 1024 * 1024;
    const MAX_LINES_SAMPLE: usize = 1000;

    let mut min_ts: Option<i64> = None;
    let mut max_ts: Option<i64> = None;
    let mut level_mask: u8 = 0;
    let mut has_any_level = false;

    let mut process_line = |line: &str| {
        if line.is_empty() {
            return;
        }
        let (timestamp_str, level) = la_core::utils::parse_metadata(line);
        if !level.is_empty() {
            has_any_level = true;
            level_mask |= la_core::utils::level_to_mask(level);
        }
        if !timestamp_str.is_empty() {
            if let Some(ts) = la_search::parse_log_timestamp_to_unix(&timestamp_str) {
                min_ts = Some(min_ts.map_or(ts, |m| m.min(ts)));
                max_ts = Some(max_ts.map_or(ts, |m| m.max(ts)));
            }
        }
    };

    if text.len() > MAX_FULL_PARSE_BYTES {
        let all_lines: Vec<&str> = text.lines().collect();
        let total = all_lines.len();
        if total > MAX_LINES_SAMPLE * 2 {
            for line in &all_lines[..MAX_LINES_SAMPLE] {
                process_line(line);
            }
            for line in &all_lines[total - MAX_LINES_SAMPLE..] {
                process_line(line);
            }
        } else {
            for line in &all_lines {
                process_line(line);
            }
        }
    } else {
        for line in text.lines() {
            process_line(line);
        }
    }

    (
        min_ts,
        max_ts,
        if has_any_level {
            Some(level_mask)
        } else {
            None
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content_returns_none() {
        let (min_ts, max_ts, levels) = compute_file_stats(b"");
        assert_eq!(min_ts, None);
        assert_eq!(max_ts, None);
        assert_eq!(levels, None);
    }

    #[test]
    fn non_utf8_content_returns_none() {
        let content = vec![0xFF, 0xFE, 0x00, 0x01];
        let (min_ts, max_ts, levels) = compute_file_stats(&content);
        assert_eq!(min_ts, None);
        assert_eq!(max_ts, None);
        assert_eq!(levels, None);
    }

    #[test]
    fn parses_single_log_line() {
        let (min_ts, max_ts, levels) =
            compute_file_stats(b"2024-01-15 10:30:00 ERROR something went wrong");
        assert!(min_ts.is_some());
        assert!(max_ts.is_some());
        // ERROR level bit should be set
        assert!(levels.is_some());
    }

    #[test]
    fn parses_multiple_lines() {
        let content = b"2024-01-15 10:30:00 INFO start\n2024-01-15 10:31:00 ERROR error\n2024-01-15 10:32:00 DEBUG detail\n";
        let (min_ts, max_ts, levels) = compute_file_stats(content);
        assert!(min_ts.is_some());
        assert!(max_ts.is_some());
        assert!(levels.is_some());
        // min should be <= max
        assert!(min_ts.unwrap() <= max_ts.unwrap());
    }
}
