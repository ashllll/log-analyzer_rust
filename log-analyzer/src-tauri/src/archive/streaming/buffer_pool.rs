use crossbeam::queue::ArrayQueue;
use tokio_util::bytes::BytesMut;

use std::sync::Arc;

/// A pool of reusable buffers to reduce allocations
pub struct BufferPool {
    pool: Arc<ArrayQueue<BytesMut>>,
    buffer_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let pool = ArrayQueue::new(capacity);
        for _ in 0..capacity {
            let _ = pool.push(BytesMut::with_capacity(buffer_size));
        }

        Self {
            pool: Arc::new(pool),
            buffer_size,
        }
    }

    /// Acquire a buffer from the pool
    pub fn acquire(&self) -> BytesMut {
        self.pool
            .pop()
            .unwrap_or_else(|| BytesMut::with_capacity(self.buffer_size))
    }

    /// Release a buffer back to the pool
    pub fn release(&self, mut buf: BytesMut) {
        buf.clear();
        let _ = self.pool.push(buf);
    }
}
