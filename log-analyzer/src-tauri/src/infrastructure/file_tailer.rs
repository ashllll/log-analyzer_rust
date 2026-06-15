//! FileTailer — owns file offset and line-count maps for incremental read.
//!
//! Extracted from `services/file_watcher.rs` so the offset-tracking logic
//! can be unit-tested with temporary files without spawning threads.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use la_core::error::{AppError, Result};

/// Result of a tail operation.
#[derive(Debug, PartialEq, Eq)]
pub struct TailResult {
    /// New lines read since the last offset.
    pub lines: Vec<String>,
    /// New file offset (file size after reading).
    pub new_offset: u64,
}

/// Tracks per-file read offsets and line counts for incremental file watching.
pub struct FileTailer {
    offsets: HashMap<PathBuf, u64>,
    line_counts: HashMap<PathBuf, usize>,
    watched_path: PathBuf,
}

impl FileTailer {
    pub fn new(watched_path: PathBuf) -> Self {
        Self {
            offsets: HashMap::new(),
            line_counts: HashMap::new(),
            watched_path,
        }
    }

    /// Register a new file (reset offset and line count to 0).
    pub fn on_create(&mut self, path: &Path) {
        self.offsets.insert(path.to_path_buf(), 0);
        self.line_counts.insert(path.to_path_buf(), 0);
    }

    /// Read new content from a file starting at its tracked offset.
    ///
    /// Updates the internal offset. Returns the new lines and the new offset.
    pub fn tail(&mut self, path: &Path) -> Result<TailResult> {
        let offset = self.offsets.get(path).copied().unwrap_or(0);
        let (lines, file_size, _start_offset) = read_file_from_offset(path, offset)?;

        if file_size > offset {
            self.offsets.insert(path.to_path_buf(), file_size);
        }

        Ok(TailResult {
            lines,
            new_offset: file_size,
        })
    }

    /// Get the line count for a file (used for line-number computation).
    pub fn line_count(&self, path: &Path) -> usize {
        self.line_counts.get(path).copied().unwrap_or(0)
    }

    /// Update the line count after processing new lines.
    pub fn add_lines(&mut self, path: &Path, count: usize) {
        let entry = self.line_counts.entry(path.to_path_buf()).or_insert(0);
        *entry += count;
    }

    /// Compute the virtual path relative to the watched root.
    pub fn virtual_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.watched_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }
}

// ── Internal helper ──

fn read_file_from_offset(path: &Path, offset: u64) -> Result<(Vec<String>, u64, u64)> {
    let mut file = File::open(path).map_err(AppError::Io)?;
    let file_size = file.metadata().map_err(AppError::Io)?.len();

    let start_offset = if file_size < offset {
        tracing::warn!(file = %path.display(), "File truncated, reading from beginning");
        0
    } else {
        offset
    };

    if start_offset >= file_size {
        return Ok((Vec::new(), file_size, start_offset));
    }

    file.seek(SeekFrom::Start(start_offset))
        .map_err(AppError::Io)?;

    let reader = BufReader::with_capacity(65536, file);
    let mut lines = Vec::new();
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => lines.push(line),
            Err(e) => {
                tracing::warn!(error = %e, "Error reading line, continuing");
                continue;
            }
        }
    }

    Ok((lines, file_size, start_offset))
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn tail_reads_entire_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\nline3\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        let result = tailer.tail(&path).unwrap();

        assert_eq!(result.lines, vec!["line1", "line2", "line3"]);
        assert!(result.new_offset > 0);
    }

    #[test]
    fn tail_reads_only_new_content_after_first_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);

        let first = tailer.tail(&path).unwrap();
        assert_eq!(first.lines.len(), 2);

        // Append more content
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(b"line3\nline4\n").unwrap();
        drop(f);

        let second = tailer.tail(&path).unwrap();
        assert_eq!(second.lines, vec!["line3", "line4"]);
    }

    #[test]
    fn tail_returns_empty_when_no_new_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        tailer.tail(&path).unwrap();

        let result = tailer.tail(&path).unwrap();
        assert!(result.lines.is_empty());
    }

    #[test]
    fn on_create_resets_offset_and_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);

        assert_eq!(tailer.line_count(&path), 0);
        let result = tailer.tail(&path).unwrap();
        assert_eq!(result.lines.len(), 2);
    }

    #[test]
    fn add_lines_updates_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "a\nb\nc\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        tailer.add_lines(&path, 3);

        assert_eq!(tailer.line_count(&path), 3);

        tailer.add_lines(&path, 2);
        assert_eq!(tailer.line_count(&path), 5);
    }

    #[test]
    fn virtual_path_strips_watched_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let path = write_temp_file(&sub, "test.log", "a\n");

        let tailer = FileTailer::new(dir.path().to_path_buf());
        let vp = tailer.virtual_path(&path);
        // Path separator varies by OS — just verify it starts with "sub"
        assert!(vp.starts_with("sub"), "Expected 'sub/...', got '{vp}'");
        assert!(
            vp.ends_with("test.log"),
            "Expected '.../test.log', got '{vp}'"
        );
    }

    #[test]
    fn truncated_file_is_reread_from_beginning() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\nline3\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        tailer.tail(&path).unwrap();

        // Truncate the file
        std::fs::write(&path, "short\n").unwrap();

        let result = tailer.tail(&path).unwrap();
        assert_eq!(result.lines, vec!["short"]);
    }
}
