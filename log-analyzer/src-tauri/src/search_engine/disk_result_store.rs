//! 磁盘搜索结果存储
//!
//! 将搜索结果写入磁盘（NDJSON 格式）并建立二进制偏移索引，
//! 实现类似 Notepad++ 的"磁盘文件即数据源"架构：
//! - 搜索结果不在前端内存中积累
//! - 前端通过 offset/limit 按需读取
//! - 虚拟滚动器仅需知道总行数
//!
//! # 文件格式
//! - `{search_id}.ndjson` — 每行一条 JSON 序列化的 LogEntry
//! - `{search_id}.idx`    — 每条记录起始字节偏移量（u64 小端序，8 字节/条）
//!
//! # 并发安全
//! - 写操作：Mutex<Option<SessionWriter>>（同一时刻仅一个写线程）
//! - 读操作：直接打开文件描述符，无需锁（多线程并发读安全）
//! - 计数/完成标志：AtomicUsize/AtomicBool（无锁读取）

use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use crate::models::LogEntry;

// ─── 公共类型 ────────────────────────────────────────────────────────────────

/// 分页读取结果，包含元数据供前端判断是否还有更多数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPageResult {
    /// 本页日志条目
    pub entries: Vec<LogEntry>,
    /// 当前已写入的总条目数（搜索仍在进行时可能增长）
    pub total_count: usize,
    /// 搜索是否已完成
    pub is_complete: bool,
    /// 是否还有更多数据可读
    pub has_more: bool,
    /// 下一页起始偏移量（None 表示没有更多）
    pub next_offset: Option<usize>,
}

// ─── 内部结构 ────────────────────────────────────────────────────────────────

struct SessionWriter {
    data_writer: BufWriter<File>,
    index_writer: BufWriter<File>,
    /// 当前数据文件的字节写入位置
    current_byte_offset: u64,
}

struct SearchSession {
    data_path: PathBuf,
    index_path: PathBuf,
    /// 当前已写入条目数（原子操作，无锁读）
    total_count: AtomicUsize,
    /// 搜索是否已完成
    is_complete: AtomicBool,
    /// 写入器（搜索完成后设为 None）
    writer: Mutex<Option<SessionWriter>>,
    /// 创建时间（用于 LRU 驱逐）
    created_at: std::time::Instant,
}

// ─── 主结构 ──────────────────────────────────────────────────────────────────

/// 磁盘搜索结果存储
///
/// 管理多个搜索会话，每个会话对应磁盘上一对文件。
pub struct DiskResultStore {
    /// 缓存目录
    cache_dir: PathBuf,
    /// 活跃会话映射（search_id → session）
    sessions: DashMap<String, Arc<SearchSession>>,
    /// 最大并发会话数（LRU 驱逐）
    max_sessions: usize,
}

impl DiskResultStore {
    /// 创建新的 DiskResultStore，如果缓存目录不存在则自动创建
    pub fn new(cache_dir: PathBuf, max_sessions: usize) -> io::Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            sessions: DashMap::new(),
            max_sessions,
        })
    }

    // ─── 写入 API ────────────────────────────────────────────────────────────

    /// 创建新搜索会话（在搜索开始前调用）
    pub fn create_session(&self, search_id: &str) -> io::Result<()> {
        // 超过最大会话数时驱逐最旧的
        if self.sessions.len() >= self.max_sessions {
            self.evict_oldest_session();
        }

        let data_path = self.cache_dir.join(format!("{search_id}.ndjson"));
        let index_path = self.cache_dir.join(format!("{search_id}.idx"));

        let data_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&data_path)?;
        let index_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&index_path)?;

        let session = Arc::new(SearchSession {
            data_path,
            index_path,
            total_count: AtomicUsize::new(0),
            is_complete: AtomicBool::new(false),
            writer: Mutex::new(Some(SessionWriter {
                // 256KB 数据缓冲，64KB 索引缓冲（索引行更小）
                data_writer: BufWriter::with_capacity(256 * 1024, data_file),
                index_writer: BufWriter::with_capacity(64 * 1024, index_file),
                current_byte_offset: 0,
            })),
            created_at: std::time::Instant::now(),
        });

        self.sessions.insert(search_id.to_string(), session);
        Ok(())
    }

    /// 追加日志条目到会话（线程安全，可从搜索线程调用）
    ///
    /// 返回写入后的总条目数。
    pub fn append_entries(&self, search_id: &str, entries: &[LogEntry]) -> io::Result<usize> {
        let session = self
            .sessions
            .get(search_id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "搜索会话不存在"))?
            .clone();

        let mut writer_guard = session.writer.lock();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "会话已完成，无法继续写入"))?;

        for entry in entries {
            // 写入当前字节偏移到索引文件（8 字节小端序）
            writer
                .index_writer
                .write_all(&writer.current_byte_offset.to_le_bytes())?;

            // 序列化 LogEntry 为 JSON 并写入数据文件（追加换行符）
            let json = serde_json::to_string(entry)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            writer.data_writer.write_all(json.as_bytes())?;
            writer.data_writer.write_all(b"\n")?;

            writer.current_byte_offset += json.len() as u64 + 1;
            session.total_count.fetch_add(1, Ordering::Relaxed);
        }

        // 每批刷新缓冲区，确保读线程能读到最新数据
        writer.data_writer.flush()?;
        writer.index_writer.flush()?;

        Ok(session.total_count.load(Ordering::Relaxed))
    }

    /// 完成搜索会话（关闭写入器，标记为完成）
    pub fn complete_session(&self, search_id: &str) -> io::Result<()> {
        let session = match self.sessions.get(search_id) {
            Some(s) => s.clone(),
            None => return Ok(()), // 会话已不存在，静默处理
        };

        {
            let mut writer_guard = session.writer.lock();
            if let Some(mut writer) = writer_guard.take() {
                writer.data_writer.flush()?;
                writer.index_writer.flush()?;
                // 写入器 drop 时关闭文件描述符
            }
        }

        session.is_complete.store(true, Ordering::Release);
        Ok(())
    }

    // ─── 读取 API ────────────────────────────────────────────────────────────

    /// 按偏移量读取一页结果
    ///
    /// 支持在搜索进行中调用（`is_complete = false` 时 `total_count` 仍可能增长）。
    pub fn read_page(
        &self,
        search_id: &str,
        offset: usize,
        limit: usize,
    ) -> io::Result<SearchPageResult> {
        let session = self
            .sessions
            .get(search_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Search session '{search_id}' not found or expired"),
                )
            })?
            .clone();

        let total = session.total_count.load(Ordering::Acquire);
        let is_complete = session.is_complete.load(Ordering::Acquire);

        // offset 超出当前已写入范围
        if offset >= total {
            return Ok(SearchPageResult {
                entries: vec![],
                total_count: total,
                is_complete,
                // 搜索未完成时可能还会写入更多条目
                has_more: !is_complete,
                next_offset: if !is_complete { Some(offset) } else { None },
            });
        }

        let actual_limit = limit.min(total - offset);

        // ── 读取索引获取起始字节偏移 ──────────────────────────────────────
        let mut index_file = File::open(&session.index_path)?;
        // 每个索引条目 8 字节
        index_file.seek(SeekFrom::Start((offset as u64) * 8))?;

        let mut start_byte_buf = [0u8; 8];
        index_file.read_exact(&mut start_byte_buf)?;
        let start_byte_offset = u64::from_le_bytes(start_byte_buf);

        // ── 定位数据文件并逐行读取 ────────────────────────────────────────
        let data_file = File::open(&session.data_path)?;
        let mut data_reader = BufReader::with_capacity(256 * 1024, data_file);
        data_reader.seek(SeekFrom::Start(start_byte_offset))?;

        let mut entries = Vec::with_capacity(actual_limit);
        let mut line = String::new();

        for _ in 0..actual_limit {
            line.clear();
            let bytes_read = data_reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break; // EOF
            }

            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<LogEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!(error = %e, offset = offset, "跳过无法解析的日志条目");
                }
            }
        }

        let next_offset = offset + entries.len();
        let has_more = next_offset < total || !is_complete;

        Ok(SearchPageResult {
            entries,
            total_count: total,
            is_complete,
            has_more,
            next_offset: if has_more { Some(next_offset) } else { None },
        })
    }

    /// 获取会话状态（计数 + 是否完成）
    pub fn get_status(&self, search_id: &str) -> Option<(usize, bool)> {
        self.sessions.get(search_id).map(|s| {
            (
                s.total_count.load(Ordering::Acquire),
                s.is_complete.load(Ordering::Acquire),
            )
        })
    }

    /// 检查会话是否存在
    pub fn has_session(&self, search_id: &str) -> bool {
        self.sessions.contains_key(search_id)
    }

    // ─── 管理 API ────────────────────────────────────────────────────────────

    /// 移除会话并删除磁盘文件
    pub fn remove_session(&self, search_id: &str) -> bool {
        if self.sessions.remove(search_id).is_some() {
            let data_path = self.cache_dir.join(format!("{search_id}.ndjson"));
            let index_path = self.cache_dir.join(format!("{search_id}.idx"));
            if let Err(e) = fs::remove_file(&data_path) {
                tracing::warn!(path = ?data_path, error = %e, "删除搜索结果数据文件失败");
            }
            if let Err(e) = fs::remove_file(&index_path) {
                tracing::warn!(path = ?index_path, error = %e, "删除搜索结果索引文件失败");
            }
            true
        } else {
            false
        }
    }

    /// 获取活跃会话数
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    // ─── 内部方法 ────────────────────────────────────────────────────────────

    fn evict_oldest_session(&self) {
        // 找到创建时间最早的会话 ID
        let oldest_id = self
            .sessions
            .iter()
            .min_by_key(|entry| entry.created_at)
            .map(|entry| entry.key().clone());

        if let Some(id) = oldest_id {
            tracing::debug!(search_id = %id, "LRU 驱逐最旧搜索会话");
            self.remove_session(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc as StdArc;

    fn make_entry(id: usize, content: &str) -> LogEntry {
        LogEntry {
            id,
            timestamp: StdArc::from("2026-01-01T00:00:00Z"),
            level: StdArc::from("INFO"),
            file: StdArc::from("test.log"),
            real_path: StdArc::from("/tmp/test.log"),
            line: id,
            content: StdArc::from(content),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }
    }

    #[test]
    fn test_write_and_read_page() {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 10).unwrap();

        let search_id = "test-session-1";
        store.create_session(search_id).unwrap();

        let entries: Vec<LogEntry> = (0..100)
            .map(|i| make_entry(i, &format!("line {i}")))
            .collect();
        let count = store.append_entries(search_id, &entries).unwrap();
        assert_eq!(count, 100);
        store.complete_session(search_id).unwrap();

        // 读取第一页
        let page = store.read_page(search_id, 0, 10).unwrap();
        assert_eq!(page.entries.len(), 10);
        assert_eq!(page.total_count, 100);
        assert!(page.is_complete);
        assert!(page.has_more);
        assert_eq!(page.next_offset, Some(10));

        // 读取中间页
        let page2 = store.read_page(search_id, 50, 20).unwrap();
        assert_eq!(page2.entries.len(), 20);
        assert_eq!(page2.entries[0].id, 50);

        // 读取最后一页
        let last = store.read_page(search_id, 95, 10).unwrap();
        assert_eq!(last.entries.len(), 5);
        assert!(!last.has_more);
        assert_eq!(last.next_offset, None);
    }

    #[test]
    fn test_session_removal() {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 10).unwrap();

        store.create_session("session-x").unwrap();
        store
            .append_entries("session-x", &[make_entry(0, "hello")])
            .unwrap();
        store.complete_session("session-x").unwrap();

        assert!(store.has_session("session-x"));
        store.remove_session("session-x");
        assert!(!store.has_session("session-x"));
    }

    #[test]
    fn test_read_during_write() {
        // 验证搜索进行中（is_complete=false）读取的正确性
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 10).unwrap();

        store.create_session("live-session").unwrap();

        // 写入 50 条
        let batch1: Vec<LogEntry> = (0..50).map(|i| make_entry(i, "data")).collect();
        store.append_entries("live-session", &batch1).unwrap();

        // 此时搜索未完成，读取
        let page = store.read_page("live-session", 0, 30).unwrap();
        assert_eq!(page.entries.len(), 30);
        assert_eq!(page.total_count, 50);
        assert!(!page.is_complete);
        assert!(page.has_more); // 搜索还在进行中

        // 再写入 50 条并完成
        let batch2: Vec<LogEntry> = (50..100).map(|i| make_entry(i, "data")).collect();
        store.append_entries("live-session", &batch2).unwrap();
        store.complete_session("live-session").unwrap();

        let final_page = store.read_page("live-session", 50, 100).unwrap();
        assert_eq!(final_page.entries.len(), 50);
        assert!(final_page.is_complete);
    }
}
