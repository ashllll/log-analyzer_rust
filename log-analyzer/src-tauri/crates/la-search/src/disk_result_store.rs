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
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use la_core::models::LogEntry;

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

/// 删除缓存目录中的崩溃残留（*.ndjson / *.idx）。
///
/// 只在构造时调用（此时没有活跃会话），进程崩溃后重启的残留文件
/// 不会再有写入者，可以安全删除。
fn sweep_crash_residue(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false)
            && matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("ndjson") | Some("idx")
            )
            && std::fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }
    if removed > 0 {
        tracing::debug!(
            path = %cache_dir.display(),
            removed,
            "Swept crash-residue search result files"
        );
    }
}

impl DiskResultStore {
    /// 创建新的 DiskResultStore，如果缓存目录不存在则自动创建
    ///
    /// 同时清扫崩溃残留的搜索结果文件（*.ndjson / *.idx）：新实例
    /// 没有任何会话，这些文件必然是无主孤儿，防止磁盘无限增长。
    pub fn new(cache_dir: PathBuf, max_sessions: usize) -> io::Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        sweep_crash_residue(&cache_dir);
        Ok(Self {
            cache_dir,
            sessions: DashMap::new(),
            max_sessions,
        })
    }

    // ─── 写入 API ────────────────────────────────────────────────────────────

    /// Validate search_id and build validated path
    fn validated_path(&self, search_id: &str) -> io::Result<std::path::PathBuf> {
        if search_id.contains('/') || search_id.contains('\\') || search_id.contains("..") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid search_id: contains path traversal characters".to_string(),
            ));
        }
        Ok(self.cache_dir.join(search_id))
    }

    /// 创建新搜索会话（在搜索开始前调用）
    pub fn create_session(&self, search_id: &str) -> io::Result<()> {
        let base_path = self.validated_path(search_id)?;

        // 超过最大会话数时驱逐最旧的**已完成**会话。进行中的会话
        // 绝不驱逐（否则会中断正在写入的搜索或前端正在分页的浏览）。
        // 若没有可驱逐的已完成会话，拒绝创建而不是破坏活动会话。
        if self.sessions.len() >= self.max_sessions && !self.evict_oldest_session() {
            return Err(io::Error::other(format!(
                "Too many active search sessions (limit {})",
                self.max_sessions
            )));
        }

        let data_path = base_path.with_extension("ndjson");
        let index_path = base_path.with_extension("idx");

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

        // 锁外预序列化 JSON，减少临界区序列化时间（chunk=2000 时 ~500KB-2MB 临时内存）
        let serialized: Vec<Vec<u8>> = entries
            .iter()
            .map(|entry| {
                let mut json = serde_json::to_vec(entry)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                json.push(b'\n'); // 预追加换行符，减少锁内写入次数
                Ok(json)
            })
            .collect::<io::Result<Vec<_>>>()?;

        let entry_count = serialized.len();

        let mut writer_guard = session.writer.lock();
        let writer = writer_guard
            .as_mut()
            .ok_or_else(|| io::Error::other("会话已完成，无法继续写入"))?;

        for json_bytes in &serialized {
            // 写入当前字节偏移到索引文件（8 字节小端序）
            writer
                .index_writer
                .write_all(&writer.current_byte_offset.to_le_bytes())?;
            writer.data_writer.write_all(json_bytes)?;
            writer.current_byte_offset += json_bytes.len() as u64;
        }
        // P2 竞态修复：先把 data 与 index 全部落盘，最后才发布新
        // total。读线程通过 total 判断可见性——若 total 先于 flush
        // 更新，分页读可能看到"索引指向尚未写完的 JSON 行"，解析
        // 失败被跳过，该条结果永久丢失。
        writer.data_writer.flush()?;
        writer.index_writer.flush()?;
        session
            .total_count
            .fetch_add(entry_count, Ordering::Release);

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

        // limit=0 时返回空页且不再继续：否则 next_offset 恒等于 offset、
        // has_more 恒为 true，前端会无限轮询同一偏移。
        if limit == 0 {
            return Ok(SearchPageResult {
                entries: vec![],
                total_count: total,
                is_complete,
                has_more: false,
                next_offset: None,
            });
        }

        // offset 超出当前已写入范围
        if offset >= total {
            return Ok(SearchPageResult {
                entries: vec![],
                total_count: total,
                is_complete,
                // 不能返回相同 next_offset，否则前端无限查询会重复请求空页。
                // 搜索进行中时由 progress/complete 事件触发 refetch。
                has_more: false,
                next_offset: None,
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
        let mut consumed = 0usize;

        for _ in 0..actual_limit {
            line.clear();
            let bytes_read = data_reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break; // EOF
            }
            consumed += 1;

            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<LogEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    // 半写防御：失败行恰是当前可见的最后一条且搜索仍在
                    // 进行——可能是写入批次尚未 flush 完成的残留，停止
                    // 本页读取，留待 progress/complete 事件触发重读。
                    let is_last_visible = offset + consumed >= total;
                    if !is_complete && is_last_visible {
                        tracing::debug!(
                            error = %e,
                            offset = offset,
                            "Last visible entry failed to parse while search is running, stopping page read"
                        );
                        break;
                    }
                    tracing::warn!(error = %e, offset = offset, "跳过无法解析的日志条目");
                }
            }
        }

        // next_offset 使用实际消费的索引位置（offset + 已请求条目数），
        // 而非成功解析的条目数，避免因空行/损坏数据导致偏移量漂移。
        let next_offset = offset + actual_limit;
        // 仅当下一偏移量小于已写入总数时才返回 has_more=true，
        // 不再用 !is_complete 独立触发：搜索进行中时 next_offset >= total 意味着
        // 当前无更多可读数据，前端应等待 search-complete 事件后再继续分页，
        // 而非立即发起空请求导致无效轮询
        let has_more = next_offset < total;

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
            let base_path = match self.validated_path(search_id) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(search_id = %search_id, error = %e, "删除时 search_id 校验失败");
                    return true;
                }
            };
            let data_path = base_path.with_extension("ndjson");
            let index_path = base_path.with_extension("idx");
            let mut errors = Vec::new();
            if let Err(e) = fs::remove_file(&data_path) {
                tracing::warn!(path = ?data_path, error = %e, "删除搜索结果数据文件失败");
                errors.push(format!("data: {e}"));
            }
            if let Err(e) = fs::remove_file(&index_path) {
                tracing::warn!(path = ?index_path, error = %e, "删除搜索结果索引文件失败");
                errors.push(format!("index: {e}"));
            }
            if !errors.is_empty() {
                tracing::warn!(
                    search_id = %search_id,
                    failed_count = errors.len(),
                    errors = %errors.join(", "),
                    "会话清理部分失败，磁盘文件可能残留"
                );
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

    /// 驱逐最旧的已完成会话，返回是否成功驱逐。
    ///
    /// 进行中的会话（`is_complete == false`）绝不会被驱逐：它们可能正在
    /// 被搜索线程写入，或正在被前端分页读取。找不到可驱逐的已完成会话
    /// 时返回 `false`，由调用方决定是否拒绝创建新会话。
    fn evict_oldest_session(&self) -> bool {
        let oldest_id = self
            .sessions
            .iter()
            .filter(|entry| entry.is_complete.load(std::sync::atomic::Ordering::Acquire))
            .min_by_key(|entry| entry.created_at)
            .map(|entry| entry.key().clone());

        if let Some(id) = oldest_id {
            tracing::debug!(search_id = %id, "驱逐最旧已完成搜索会话");
            self.remove_session(&id);
            true
        } else {
            false
        }
    }

    /// 清理所有会话并删除对应的磁盘文件
    pub fn cleanup_all(&self) {
        let session_ids: Vec<String> = self.sessions.iter().map(|e| e.key().clone()).collect();
        for id in session_ids {
            self.remove_session(&id);
        }
    }
}

impl Drop for DiskResultStore {
    fn drop(&mut self) {
        // 清理所有会话的磁盘文件
        let session_ids: Vec<String> = self.sessions.iter().map(|e| e.key().clone()).collect();
        for id in &session_ids {
            let base_path = match self.validated_path(id) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let data_path = base_path.with_extension("ndjson");
            let index_path = base_path.with_extension("idx");
            let _ = std::fs::remove_file(&data_path);
            let _ = std::fs::remove_file(&index_path);
        }
        tracing::debug!(
            sessions_cleaned = session_ids.len(),
            "DiskResultStore 已清理所有磁盘缓存文件"
        );
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

    // ── P2#2 regression: eviction must never destroy active sessions ──

    #[test]
    fn eviction_prefers_completed_sessions_and_keeps_active_ones() {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 2).unwrap();

        store.create_session("completed-old").unwrap();
        store.complete_session("completed-old").unwrap();
        store.create_session("active-new").unwrap();
        store
            .append_entries("active-new", &[make_entry(0, "live")])
            .unwrap();

        // 第三个会话触发驱逐：必须驱逐已完成的旧会话，
        // 进行中的会话保持可用。
        store.create_session("third").unwrap();

        assert!(
            !store.has_session("completed-old"),
            "completed oldest session should be evicted"
        );
        assert!(
            store.has_session("active-new"),
            "active session must survive"
        );
        assert!(store.has_session("third"));

        // 活动会话仍可继续写入与读取
        store
            .append_entries("active-new", &[make_entry(1, "more")])
            .unwrap();
        let page = store.read_page("active-new", 0, 10).unwrap();
        assert_eq!(page.total_count, 2);
    }

    #[test]
    fn create_session_fails_instead_of_evicting_when_all_sessions_are_active() {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 2).unwrap();

        store.create_session("active-1").unwrap();
        store.create_session("active-2").unwrap();

        // 没有可驱逐的已完成会话：拒绝创建而非破坏活动会话。
        let err = store.create_session("active-3").unwrap_err();
        assert!(err.to_string().contains("Too many active search sessions"));
        assert!(store.has_session("active-1"));
        assert!(store.has_session("active-2"));
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

    // ── P2 竞态回归：total 发布前 data/index 已落盘 ──

    #[test]
    fn zero_limit_returns_empty_page_without_infinite_polling() {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 10).unwrap();
        store.create_session("zero-limit").unwrap();
        store
            .append_entries("zero-limit", &[make_entry(0, "a")])
            .unwrap();

        let page = store.read_page("zero-limit", 0, 0).unwrap();
        assert!(page.entries.is_empty());
        assert!(!page.has_more, "has_more must be false to stop polling");
        assert_eq!(page.next_offset, None);
    }

    #[test]
    fn constructor_sweeps_crash_residue_files() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("search-cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("dead-session.ndjson"), b"{}").unwrap();
        std::fs::write(cache_dir.join("dead-session.idx"), b"\0\0").unwrap();
        std::fs::write(cache_dir.join("keep.txt"), b"unrelated").unwrap();

        DiskResultStore::new(cache_dir.clone(), 10).unwrap();

        assert!(!cache_dir.join("dead-session.ndjson").exists());
        assert!(!cache_dir.join("dead-session.idx").exists());
        assert!(
            cache_dir.join("keep.txt").exists(),
            "non-residue files must survive"
        );
    }

    #[test]
    fn every_published_total_count_is_fully_readable() {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskResultStore::new(dir.path().to_path_buf(), 10).unwrap();
        store.create_session("consistency-check").unwrap();

        // 多批次小写入；每批后立即分页读取，任何"total 已发布但数据
        // 未落盘"的窗口都会表现为读页报错或解析失败。
        for batch in 0..20 {
            let entries: Vec<LogEntry> = (batch * 5..batch * 5 + 5)
                .map(|i| make_entry(i, &format!("line {i}")))
                .collect();
            store.append_entries("consistency-check", &entries).unwrap();

            let page = store.read_page("consistency-check", 0, 10_000).unwrap();
            assert_eq!(
                page.entries.len(),
                (batch + 1) * 5,
                "published total must be fully readable at batch {batch}"
            );
        }
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
        assert!(page.has_more);

        let empty_tail = store.read_page("live-session", 50, 30).unwrap();
        assert!(empty_tail.entries.is_empty());
        assert_eq!(empty_tail.total_count, 50);
        assert!(!empty_tail.is_complete);
        assert!(!empty_tail.has_more);
        assert_eq!(empty_tail.next_offset, None);

        // 再写入 50 条并完成
        let batch2: Vec<LogEntry> = (50..100).map(|i| make_entry(i, "data")).collect();
        store.append_entries("live-session", &batch2).unwrap();
        store.complete_session("live-session").unwrap();

        let final_page = store.read_page("live-session", 50, 100).unwrap();
        assert_eq!(final_page.entries.len(), 50);
        assert!(final_page.is_complete);
    }
}
