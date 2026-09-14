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
    /// The file shrank below the committed offset, including truncation to zero.
    pub truncated: bool,
    previous_offset: Option<u64>,
    previous_line_count: Option<usize>,
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
    /// 只有以换行结尾的完整行会被消费；末尾的半行留到下一次事件补全，
    /// 避免一条日志被拆成两行、行号漂移。
    pub fn tail(&mut self, path: &Path) -> Result<TailResult> {
        let previous_offset = self.offsets.get(path).copied();
        let previous_line_count = self.line_counts.get(path).copied();
        let offset = previous_offset.unwrap_or(0);
        let (lines, consumed_offset, start_offset) = read_file_from_offset(path, offset)?;

        if start_offset < offset {
            self.line_counts.insert(path.to_path_buf(), 0);
        }
        self.offsets.insert(path.to_path_buf(), consumed_offset);

        Ok(TailResult {
            lines,
            new_offset: consumed_offset,
            truncated: start_offset < offset,
            previous_offset,
            previous_line_count,
        })
    }

    /// Restore the offset and line-count state captured by a `tail` call.
    ///
    /// Watcher persistence and indexing use this when either side effect fails,
    /// allowing the same bytes to be retried on the next filesystem event.
    pub fn rollback(&mut self, path: &Path, result: &TailResult) {
        match result.previous_offset {
            Some(offset) => {
                self.offsets.insert(path.to_path_buf(), offset);
            }
            None => {
                self.offsets.remove(path);
            }
        }

        match result.previous_line_count {
            Some(count) => {
                self.line_counts.insert(path.to_path_buf(), count);
            }
            None => {
                self.line_counts.remove(path);
            }
        }
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
        // Import uses the selected root basename followed by the relative path.
        // For a watched file the relative suffix is empty, so keep its basename.
        let root_name = self.watched_path.file_name().unwrap_or_default();
        match path.strip_prefix(&self.watched_path) {
            Ok(relative) if relative.as_os_str().is_empty() => {
                root_name.to_string_lossy().into_owned()
            }
            Ok(relative) => Path::new(root_name)
                .join(relative)
                .to_string_lossy()
                .into_owned(),
            Err(_) => path.to_string_lossy().into_owned(),
        }
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

    let mut reader = BufReader::with_capacity(65536, file);
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut consumed_offset = start_offset;
    loop {
        current.clear();
        let n = reader
            .read_until(b'\n', &mut current)
            .map_err(AppError::Io)?;
        if n == 0 {
            break; // EOF
        }
        if current.last() != Some(&b'\n') {
            // 半行（无换行结尾）：不消费，留到下一次事件补全。
            break;
        }
        while matches!(current.last(), Some(b'\n' | b'\r')) {
            current.pop();
        }
        lines.push(String::from_utf8_lossy(&current).into_owned());
        consumed_offset += n as u64;
    }

    Ok((lines, consumed_offset, start_offset))
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

    // ── P2 回归：末尾半行留待补全，不拆行 ──

    #[test]
    fn tail_holds_back_a_partial_final_line_until_it_is_completed() {
        let dir = tempfile::tempdir().unwrap();
        let watched = dir.path();
        let file_path = dir.path().join("app.log");
        std::fs::write(&file_path, b"line1\npartial").unwrap();

        let mut tailer = FileTailer::new(watched.to_path_buf());
        let first = tailer.tail(&file_path).unwrap();

        // 完整行被消费；半行保留。
        assert_eq!(first.lines, vec!["line1".to_string()]);
        assert_eq!(first.new_offset, "line1\n".len() as u64);

        // 写者补全同一逻辑行。
        std::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .unwrap()
            .write_all(b" rest\n")
            .unwrap();
        let second = tailer.tail(&file_path).unwrap();

        // 半行 + 追加内容合并为一条完整日志，不拆成两行。
        assert_eq!(second.lines, vec!["partial rest".to_string()]);
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
    fn rollback_replays_uncommitted_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        let first = tailer.tail(&path).unwrap();
        tailer.rollback(&path, &first);

        let retried = tailer.tail(&path).unwrap();
        assert_eq!(retried.lines, vec!["line1", "line2"]);
    }

    #[test]
    fn rollback_restores_line_count_after_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        tailer.tail(&path).unwrap();
        tailer.add_lines(&path, 2);
        std::fs::write(&path, "short\n").unwrap();

        let truncated = tailer.tail(&path).unwrap();
        assert_eq!(tailer.line_count(&path), 0);
        tailer.rollback(&path, &truncated);

        assert_eq!(tailer.line_count(&path), 2);
        assert_eq!(tailer.tail(&path).unwrap().lines, vec!["short"]);
    }

    #[test]
    fn virtual_path_matches_import_root_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let path = write_temp_file(&sub, "test.log", "a\n");

        let tailer = FileTailer::new(dir.path().to_path_buf());
        let vp = tailer.virtual_path(&path);
        let expected = Path::new(dir.path().file_name().unwrap()).join("sub/test.log");
        assert_eq!(vp, expected.to_string_lossy());
        assert!(
            vp.ends_with("test.log"),
            "Expected '.../test.log', got '{vp}'"
        );
    }

    #[test]
    fn single_file_identity_is_its_basename() {
        let path = PathBuf::from("/logs/app.log");
        assert_eq!(FileTailer::new(path.clone()).virtual_path(&path), "app.log");
    }

    #[test]
    fn truncation_to_empty_is_reported_even_without_new_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "app.log", "old ERROR\n");
        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.tail(&path).unwrap();
        std::fs::write(&path, "").unwrap();
        let result = tailer.tail(&path).unwrap();
        assert!(result.truncated);
        assert!(result.lines.is_empty());
        assert!(!tailer.tail(&path).unwrap().truncated);
    }

    #[test]
    fn truncated_file_is_reread_from_beginning() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_temp_file(dir.path(), "test.log", "line1\nline2\nline3\n");

        let mut tailer = FileTailer::new(dir.path().to_path_buf());
        tailer.on_create(&path);
        tailer.tail(&path).unwrap();
        tailer.add_lines(&path, 3);

        // Truncate the file
        std::fs::write(&path, "short\n").unwrap();

        let result = tailer.tail(&path).unwrap();
        assert_eq!(result.lines, vec!["short"]);
        assert_eq!(
            tailer.line_count(&path),
            0,
            "truncation must reset the source line-number base"
        );

        let unchanged = tailer.tail(&path).unwrap();
        assert!(
            unchanged.lines.is_empty(),
            "truncated content must not be emitted again without another write"
        );

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"next\n").unwrap();
        drop(file);

        let appended = tailer.tail(&path).unwrap();
        assert_eq!(appended.lines, vec!["next"]);
    }
}
