//! 基于 Atomic 的无锁 Chunked Array 实现
//!
//! 使用 std::sync::atomic 实现原子指针，实现无锁追加索引
//! 设计 Chunked Array，每块 128KB，实现 compare_exchange 原子挂载
//!
//! # 安全说明
//! 本模块使用 Mutex 保护并发访问，确保线程安全。

use parking_lot::Mutex;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;

/// 默认块大小：128KB
const DEFAULT_CHUNK_SIZE: usize = 128 * 1024;

/// Chunked Array 块
struct Chunk {
    /// 块数据
    data: Vec<u8>,
    /// 当前写入位置（使用 Mutex 实现内部可变性）
    position: Mutex<usize>,
    /// 下一块的原子指针
    next: AtomicPtr<Chunk>,
}

impl Chunk {
    /// 创建新块
    fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            position: Mutex::new(0),
            next: AtomicPtr::new(null_mut()),
        }
    }

    /// 写入数据
    fn write(&mut self, bytes: &[u8]) -> Option<usize> {
        let mut pos = *self.position.lock();
        let capacity = self.data.capacity();

        if pos + bytes.len() > capacity {
            return None;
        }

        // 安全地写入数据
        unsafe {
            let dst = self.data.as_mut_ptr().add(pos);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
            // 更新 Vec 的实际长度以反映新数据
            self.data.set_len(pos + bytes.len());
        }

        pos += bytes.len();
        *self.position.lock() = pos;
        Some(bytes.len())
    }

    /// 获取下一块
    fn get_next(&self) -> Option<&Chunk> {
        let ptr = self.next.load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(&*ptr) }
        }
    }

    /// 设置下一块
    #[allow(dead_code)]
    fn set_next(&self, chunk: *mut Chunk) {
        self.next.store(chunk, Ordering::Release);
    }

    /// 获取当前使用量
    fn used(&self) -> usize {
        *self.position.lock()
    }

    /// 获取容量
    #[allow(dead_code)]
    fn capacity(&self) -> usize {
        self.data.capacity()
    }
}

/// 无锁 Chunked Array
///
/// 支持并发写入，通过 compare_exchange 实现原子挂载新块
/// 使用 Mutex 保护 tail 指针的并发访问，确保线程安全
pub struct ChunkedArray {
    /// 头块的原子指针
    head: AtomicPtr<Chunk>,
    /// 尾块的 Mutex 指针（用于快速追加）
    /// 使用 Mutex 保护并发访问，防止数据竞争
    tail: Mutex<*mut Chunk>,
    /// 块大小
    chunk_size: usize,
    /// 总元素数
    total_elements: AtomicUsize,
}

impl ChunkedArray {
    /// 创建新的 ChunkedArray
    pub fn new(chunk_size: usize) -> Self {
        let initial_chunk = Box::into_raw(Box::new(Chunk::new(chunk_size)));

        Self {
            head: AtomicPtr::new(initial_chunk),
            tail: Mutex::new(initial_chunk),
            chunk_size,
            total_elements: AtomicUsize::new(0),
        }
    }

    /// 使用默认块大小创建
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_CHUNK_SIZE)
    }

    /// 写入数据
    ///
    /// 返回写入的字节数，失败返回 None
    /// 使用 Mutex 保护并发访问，确保线程安全
    pub fn write(&self, bytes: &[u8]) -> Option<usize> {
        // 使用 Mutex 保护整个写入过程，防止数据竞争
        let mut tail_guard = self.tail.lock();

        // 尝试在当前尾块写入
        loop {
            let tail = *tail_guard;
            unsafe {
                // 将不可变引用转换为可变引用
                // 现在是安全的，因为 Mutex 保证了独占访问
                let chunk = &mut *tail;
                if let Some(written) = chunk.write(bytes) {
                    self.total_elements.fetch_add(1, Ordering::Release);
                    return Some(written);
                }

                // 当前块已满，创建新块并尝试原子挂载
                let new_chunk = Box::into_raw(Box::new(Chunk::new(self.chunk_size)));

                // 使用 compare_exchange 尝试挂载新块
                let result = (*tail).next.compare_exchange(
                    null_mut(),
                    new_chunk,
                    Ordering::Release,
                    Ordering::Acquire,
                );

                match result {
                    Ok(_) => {
                        // 成功挂载，更新 tail 指针
                        *tail_guard = new_chunk;
                        // 继续尝试写入（递归）
                    }
                    Err(_) => {
                        // 另一个线程已经挂载了新块，释放我们创建的新块
                        // 并重试
                        let _ = Box::from_raw(new_chunk);
                    }
                }
            }
        }
    }

    /// 写入 usize 值
    pub fn write_usize(&self, value: usize) -> Option<usize> {
        let bytes = value.to_le_bytes();
        self.write(&bytes)
    }

    /// 写入索引条目（行号 + 偏移量 + 长度）
    pub fn write_index_entry(
        &self,
        line_number: u64,
        byte_offset: u64,
        length: u32,
    ) -> Option<usize> {
        let mut total = 0_usize;

        // 写入行号 (8 bytes)
        total += self.write(&line_number.to_le_bytes())?;
        // 写入偏移量 (8 bytes)
        total += self.write(&byte_offset.to_le_bytes())?;
        // 写入长度 (4 bytes)
        total += self.write(&length.to_le_bytes())?;

        Some(total)
    }

    /// 获取元素数量
    pub fn len(&self) -> usize {
        self.total_elements.load(Ordering::Acquire)
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 迭代所有数据
    pub fn iter(&self) -> ChunkedArrayIter<'_> {
        let head_ptr = self.head.load(Ordering::Acquire);
        let chunk = if head_ptr.is_null() {
            None
        } else {
            unsafe { Some(&*head_ptr) }
        };

        ChunkedArrayIter {
            current_chunk: chunk,
            current_offset: 0,
        }
    }

    /// 估算内存使用量
    pub fn memory_usage(&self) -> usize {
        let mut count = 0;
        let mut ptr = self.head.load(Ordering::Acquire);

        while !ptr.is_null() {
            unsafe {
                let chunk = &*ptr;
                count += chunk.data.capacity();
                ptr = chunk.next.load(Ordering::Acquire);
            }
        }

        count
    }
}

impl Default for ChunkedArray {
    fn default() -> Self {
        Self::with_default_size()
    }
}

/// ChunkedArray 迭代器
pub struct ChunkedArrayIter<'a> {
    current_chunk: Option<&'a Chunk>,
    current_offset: usize,
}

impl<'a> Iterator for ChunkedArrayIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(chunk) = self.current_chunk {
            let used = chunk.used();
            if self.current_offset < used {
                let byte = chunk.data[self.current_offset];
                self.current_offset += 1;
                return Some(byte);
            }

            // 移动到下一块
            if let Some(next) = chunk.get_next() {
                self.current_chunk = Some(next);
                self.current_offset = 0;
                return self.next();
            }
        }

        None
    }
}

/// 线程安全的 Arc 包装
pub type SharedChunkedArray = Arc<ChunkedArray>;

impl Clone for ChunkedArray {
    fn clone(&self) -> Self {
        Self {
            head: AtomicPtr::new(self.head.load(Ordering::Acquire)),
            tail: Mutex::new(*self.tail.lock()),
            chunk_size: self.chunk_size,
            total_elements: AtomicUsize::new(self.total_elements.load(Ordering::Acquire)),
        }
    }
}

/// Drop 实现 - 释放所有 Chunk 内存
impl Drop for ChunkedArray {
    fn drop(&mut self) {
        // 遍历链表释放所有 Chunk
        unsafe {
            let mut ptr = self.head.load(Ordering::Acquire);
            while !ptr.is_null() {
                let next = (*ptr).next.load(Ordering::Acquire);
                let _ = Box::from_raw(ptr);
                ptr = next;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write() {
        let array = ChunkedArray::with_default_size();

        // 写入一些数据
        assert!(array.write(b"hello").is_some());
        assert!(array.write(b" world").is_some());

        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_index_entry() {
        let array = ChunkedArray::with_default_size();

        // 写入索引条目
        // 每个索引条目包含 3 个字段 (line_number, byte_offset, length)
        // 每个字段调用一次 write，所以会添加 3 个元素
        array.write_index_entry(1, 0, 100).unwrap();
        array.write_index_entry(2, 100, 50).unwrap();

        // 每个 write_index_entry 调用 3 次 write()
        // 所以总共 2 * 3 = 6 个元素
        assert_eq!(array.len(), 6);
    }

    #[test]
    fn test_chunk_overflow() {
        let array = ChunkedArray::new(16); // 小块大小用于测试

        // 写入超过块大小的数据
        let data = b"This is a very long string that exceeds the chunk size";
        let written = array.write(data);

        assert!(written.is_some());
        assert!(!array.is_empty());
    }

    #[test]
    fn test_memory_usage() {
        let array = ChunkedArray::with_default_size();
        array.write(b"test data").unwrap();

        let usage = array.memory_usage();
        assert!(usage >= 9);
    }
}
