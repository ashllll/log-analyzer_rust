use super::buffer_pool::BufferPool;
use crate::error::Result;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Semaphore;

/// The StreamingPipeline handles high-performance data transfer with backpressure
pub struct StreamingPipeline {
    buffer_pool: Arc<BufferPool>,
    // Semaphore for controlling concurrent I/O operations (backpressure)
    io_semaphore: Arc<Semaphore>,
}

impl StreamingPipeline {
    pub fn new(buffer_pool: Arc<BufferPool>, max_concurrent_io: usize) -> Self {
        Self {
            buffer_pool,
            io_semaphore: Arc::new(Semaphore::new(max_concurrent_io)),
        }
    }

    /// Stream data from reader to writer with backpressure and buffer pooling
    pub async fn stream_transfer<R, W>(&self, mut reader: R, mut writer: W) -> Result<u64>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        // Acquire permit for I/O
        let _permit = self.io_semaphore.acquire().await.map_err(|e| {
            crate::error::AppError::archive_error(format!("Semaphore error: {}", e), None)
        })?;

        let mut total_bytes = 0u64;
        let mut buffer = self.buffer_pool.acquire();

        // Ensure buffer has enough space
        if buffer.capacity() < 64 * 1024 {
            buffer = tokio_util::bytes::BytesMut::with_capacity(64 * 1024);
        }
        // Safety: we need to use the full capacity for reading
        buffer.resize(buffer.capacity(), 0);

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            writer.write_all(&buffer[..n]).await?;
            total_bytes += n as u64;

            // Periodically flush to ensure progress
            if total_bytes.is_multiple_of(1024 * 1024) {
                writer.flush().await?;
            }
        }

        writer.flush().await?;
        self.buffer_pool.release(buffer);

        Ok(total_bytes)
    }
}
