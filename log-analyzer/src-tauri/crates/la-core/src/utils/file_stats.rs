//! File-level log metadata statistics.
//!
//! Pure conversion from raw log bytes to the time range and level mask used by
//! incremental search metadata. Only complete statistics may exclude a file
//! from search; large files report unknown bounds instead of sampled bounds.

use super::{level_to_mask, parse_metadata, TimestampParser};

pub const MAX_FULL_PARSE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Default)]
struct FileStats {
    min_timestamp: Option<i64>,
    max_timestamp: Option<i64>,
    level_mask: u8,
    has_lines: bool,
}

impl FileStats {
    fn observe(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }

        self.has_lines = true;
        let (timestamp, level) = parse_metadata(line);
        self.level_mask |= level_to_mask(level);
        if let Some(parsed) = TimestampParser::parse_naive_datetime(&timestamp) {
            let timestamp = parsed.and_utc().timestamp();
            self.min_timestamp = Some(
                self.min_timestamp
                    .map_or(timestamp, |current| current.min(timestamp)),
            );
            self.max_timestamp = Some(
                self.max_timestamp
                    .map_or(timestamp, |current| current.max(timestamp)),
            );
        }
    }

    fn finish(self) -> (Option<i64>, Option<i64>, Option<u8>) {
        (
            self.min_timestamp,
            self.max_timestamp,
            self.has_lines.then_some(self.level_mask),
        )
    }
}

/// Compute `(minimum timestamp, maximum timestamp, level mask)` from raw log bytes.
pub fn compute_file_stats(content: &[u8]) -> (Option<i64>, Option<i64>, Option<u8>) {
    // Unknown statistics keep the file eligible for every search filter. A
    // sample cannot prove the absence of a level or an out-of-order timestamp.
    if content.len() > MAX_FULL_PARSE_BYTES {
        return (None, None, None);
    }
    let Ok(text) = std::str::from_utf8(content) else {
        return (None, None, None);
    };

    let mut stats = FileStats::default();
    for line in text.lines() {
        stats.observe(line);
    }

    stats.finish()
}

#[cfg(test)]
mod tests {
    use super::compute_file_stats;

    #[test]
    fn reports_known_timestamp_and_level_for_one_line() {
        assert_eq!(
            compute_file_stats(b"2024-01-15 10:30:00 ERROR request failed"),
            (Some(1_705_314_600), Some(1_705_314_600), Some(0b00001))
        );
    }

    #[test]
    fn empty_and_invalid_utf8_content_have_no_statistics() {
        assert_eq!(compute_file_stats(b""), (None, None, None));
        assert_eq!(compute_file_stats(&[0xff, 0xfe, 0x00]), (None, None, None));
    }

    #[test]
    fn reports_range_and_combined_mask_for_multiple_lines() {
        let content = b"2024-01-15T10:32:00 DEBUG detail\n2024-01-15 10:30:00 INFO start\n2024/01/15 10:31:00 WARN retry";

        assert_eq!(
            compute_file_stats(content),
            (Some(1_705_314_600), Some(1_705_314_720), Some(0b01110))
        );
    }

    #[test]
    fn supports_us_timestamp_format() {
        assert_eq!(
            compute_file_stats(b"01/15/2024 10:30:00 ERROR failed"),
            (Some(1_705_314_600), Some(1_705_314_600), Some(0b00001))
        );
    }

    #[test]
    fn content_without_a_timestamp_still_reports_its_level_mask() {
        assert_eq!(
            compute_file_stats(b"WARN retry without timestamp"),
            (None, None, Some(0b00010))
        );
    }

    #[test]
    fn large_files_do_not_publish_sampled_bounds_as_complete_statistics() {
        let filler = "x".repeat(6_000);
        let mut lines = vec![filler; 2_001];
        lines[0] = "2024-01-15 10:30:00 INFO first".to_string();
        lines[1_000] = "2024-01-15 10:31:00 ERROR unsampled".to_string();
        lines[2_000] = "2024-01-15 10:32:00 WARN last".to_string();
        let content = lines.join("\n");

        assert_eq!(compute_file_stats(content.as_bytes()), (None, None, None));
    }
}
