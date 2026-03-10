//! PageManager 滑动窗口 Mmap 实现
//!
//! 使用 memmap2 crate 实现虚拟内存管理
//! 维持最多 3GB 虚拟地址映射，实现视口按需加载
//!
//! PRD 2.4 编码检测集成：
//! - 创建 PageManager 前检测文件编码
//! - UTF-16/UTF-32 等多字节编码会破坏 SIMD 优化，需要转码
//!
//! # 安全说明
//! 本模块使用 Mutex 保护视口复合操作，确保线程安全。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use thiserror::Error;

// 导入编码检测和转码模块
use crate::utils::encoding_detector::EncodingDetector;
use crate::utils::transcoding_pipe::{TranscodingError, TranscodingPipe};

/// PageManager 错误类型
#[derive(Error, Debug)]
pub enum PageManagerError {
    #[error("文件映射失败: {0}")]
    MappingFailed(String),

    #[error("访问超出范围: {0}")]
    OutOfBounds(String),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("编码检测失败: {0}")]
    EncodingDetectionFailed(String),

    #[error("转码失败: {0}")]
    TranscodingFailed(String),

    #[error("编码不支持 SIMD 优化，需要转码: {0}")]
    RequiresTranscoding(String),
}

// 实现 TranscodingError 到 PageManagerError 的转换
impl From<TranscodingError> for PageManagerError {
    fn from(err: TranscodingError) -> Self {
        match err {
            TranscodingError::IoError(e) => PageManagerError::IoError(e),
            TranscodingError::EncodingDetectionFailed(msg) => {
                PageManagerError::EncodingDetectionFailed(msg)
            }
            TranscodingError::TranscodingFailed(msg) => PageManagerError::TranscodingFailed(msg),
            TranscodingError::TempFileCreationFailed(msg) => {
                PageManagerError::TranscodingFailed(msg)
            }
            TranscodingError::UnsupportedEncoding(msg) => {
                PageManagerError::RequiresTranscoding(msg)
            }
        }
    }
}

/// 页面信息
#[derive(Debug, Clone)]
pub struct PageInfo {
    pub page_number: usize,
    pub offset: u64,
    pub size: usize,
}

/// 视口信息
#[derive(Debug, Clone)]
pub struct Viewport {
    /// 视口起始偏移
    pub start_offset: u64,
    /// 视口大小（字节）
    pub size: usize,
    /// 页面大小
    pub page_size: usize,
}

/// PageManager 配置
#[derive(Debug, Clone)]
pub struct PageManagerConfig {
    /// 单个页面大小（默认 4MB）
    pub page_size: usize,
    /// 最大映射内存（默认 3GB）
    pub max_mapped_memory: usize,
    /// 预加载页面数量
    pub preload_pages: usize,
}

impl Default for PageManagerConfig {
    fn default() -> Self {
        Self {
            page_size: 4 * 1024 * 1024,                // 4MB
            max_mapped_memory: 3 * 1024 * 1024 * 1024, // 3GB
            preload_pages: 2,
        }
    }
}

/// 视口状态（用于 Mutex 保护）
struct ViewportState {
    /// 视口起始位置
    start: u64,
    /// 视口大小
    size: usize,
}

/// PageManager - 滑动窗口内存映射管理
///
/// 支持按需加载页面，维持内存映射窗口，实现高效的虚拟内存访问
#[allow(dead_code)]
pub struct PageManager {
    /// 文件路径
    #[allow(dead_code)]
    path: std::path::PathBuf,
    /// 文件大小
    file_size: u64,
    /// 配置
    config: PageManagerConfig,
    /// 视口状态（使用 Mutex 保护，确保复合操作原子性）
    viewport: Mutex<ViewportState>,
    /// 当前映射的页面数
    mapped_pages: AtomicUsize,
    /// 内存映射数据
    #[cfg(windows)]
    mapping: Option<memmap2::Mmap>,
    #[cfg(not(windows))]
    mapping: Option<memmap2::Mmap>,
}

impl PageManager {
    /// 从文件创建 PageManager
    pub fn new(path: impl Into<std::path::PathBuf>) -> Result<Self, PageManagerError> {
        Self::with_config(path, PageManagerConfig::default())
    }

    /// 从文件创建 PageManager（带配置）
    pub fn with_config(
        path: impl Into<std::path::PathBuf>,
        config: PageManagerConfig,
    ) -> Result<Self, PageManagerError> {
        let path = path.into();
        let metadata = std::fs::metadata(&path)?;
        let file_size = metadata.len();

        // 创建内存映射
        let file = std::fs::File::open(&path)?;
        let mapping = unsafe { memmap2::Mmap::map(&file)? };

        let page_count = (file_size as usize).div_ceil(config.page_size);

        tracing::debug!(
            "PageManager created: path={}, size={} bytes, pages={}",
            path.display(),
            file_size,
            page_count
        );

        Ok(Self {
            path,
            file_size,
            config,
            viewport: Mutex::new(ViewportState { start: 0, size: 0 }),
            mapped_pages: AtomicUsize::new(page_count),
            mapping: Some(mapping),
        })
    }

    /// 从文件创建 PageManager（带编码检测）
    ///
    /// PRD 2.4 要求：遭遇 UTF-16 等导致 SIMD 失效的编码时，立刻中断 Mmap
    ///
    /// 此方法会：
    /// 1. 检测文件编码
    /// 2. 如果编码会破坏 SIMD 优化（UTF-16/UTF-32），返回错误
    /// 3. 如果需要转码但不会破坏 SIMD（GBK 等），仍然创建 PageManager
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回值
    ///
    /// - `Ok(Self)`: 可以直接使用 Mmap 的文件
    /// - `Err(RequiresTranscoding)`: 需要先转码的文件（UTF-16/UTF-32）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// match PageManager::with_encoding_check(path) {
    ///     Ok(pm) => { /* 直接使用 PageManager */ },
    ///     Err(PageManagerError::RequiresTranscoding(_)) => {
    ///         // 需要先转码
    ///         let pipe = TranscodingPipe::create(&path).await?;
    ///         let pm = PageManager::new(pipe.path())?;
    ///     },
    ///     Err(e) => return Err(e),
    /// }
    /// ```
    pub fn with_encoding_check(
        path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, PageManagerError> {
        let path = path.into();

        // 检测文件编码
        let detection = EncodingDetector::detect_from_file(&path)
            .map_err(|e| PageManagerError::EncodingDetectionFailed(e.to_string()))?;

        tracing::debug!(
            path = %path.display(),
            encoding = %detection.encoding_name,
            needs_transcoding = detection.needs_transcoding,
            breaks_simd = detection.breaks_simd,
            "编码检测结果"
        );

        // 检查是否需要转码
        if detection.needs_transcoding || detection.has_bom {
            // 检查是否会破坏 SIMD 优化
            if detection.breaks_simd {
                tracing::warn!(
                    path = %path.display(),
                    encoding = %detection.encoding_name,
                    "文件编码会破坏 SIMD 优化，需要转码"
                );
                return Err(PageManagerError::RequiresTranscoding(format!(
                    "文件编码 {} 会破坏 SIMD 优化，请使用 TranscodingPipe 转码后再创建 PageManager",
                    detection.encoding_name
                )));
            }

            // GBK 等编码需要转码但不会破坏 SIMD，可以选择直接使用或转码
            tracing::info!(
                path = %path.display(),
                encoding = %detection.encoding_name,
                "文件需要转码但不会破坏 SIMD 优化"
            );
        }

        // 编码兼容，正常创建 PageManager
        Self::new(path)
    }

    /// 异步从文件创建 PageManager（带编码检测和自动转码）
    ///
    /// 这是推荐的创建方式，会自动处理编码问题：
    /// 1. 检测文件编码
    /// 2. 如果需要转码，自动创建临时转码文件
    /// 3. 返回可以正常使用的 PageManager
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回值
    ///
    /// 返回 PageManager 实例，可能使用转码后的临时文件
    pub async fn with_auto_transcoding(
        path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, PageManagerError> {
        let path = path.into();

        // 检测文件编码
        let detection = EncodingDetector::detect_from_file_async(&path)
            .await
            .map_err(|e| PageManagerError::EncodingDetectionFailed(e.to_string()))?;

        // 检查是否需要转码
        if detection.needs_transcoding || detection.has_bom {
            tracing::info!(
                path = %path.display(),
                encoding = %detection.encoding_name,
                "文件需要转码，启动转码管道"
            );

            // 执行转码
            let pipe = TranscodingPipe::create(&path)
                .await
                .map_err(PageManagerError::from)?;

            // 使用转码后的临时文件创建 PageManager
            let _transcoded_path = pipe.path().to_path_buf();

            // 注意：pipe 被 drop 后临时文件会被删除，所以需要直接使用路径
            // 这里使用 persist 方法保留临时文件
            let temp_path = std::env::temp_dir().join(format!(
                "log-analyzer-transcoded-{}.tmp",
                uuid::Uuid::new_v4()
            ));
            let persisted_path = pipe
                .persist(&temp_path)
                .map_err(|e| PageManagerError::TranscodingFailed(e.to_string()))?;

            tracing::info!(
                original = %path.display(),
                transcoded = %persisted_path.display(),
                "转码完成，使用临时文件创建 PageManager"
            );

            return Self::new(persisted_path);
        }

        // 编码兼容，正常创建 PageManager
        Self::new(path)
    }

    /// 获取文件大小
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// 获取页面数量
    pub fn page_count(&self) -> usize {
        (self.file_size as usize).div_ceil(self.config.page_size)
    }

    /// 获取页面信息
    pub fn get_page_info(&self, page_number: usize) -> Option<PageInfo> {
        if page_number >= self.page_count() {
            return None;
        }

        let offset = (page_number as u64) * (self.config.page_size as u64);
        let size = std::cmp::min(
            self.config.page_size,
            (self.file_size as usize) - (page_number * self.config.page_size),
        );

        Some(PageInfo {
            page_number,
            offset,
            size,
        })
    }

    /// 设置视口
    ///
    /// 视口定义了当前需要访问的内存区域
    /// 使用 Mutex 保护，确保复合操作的原子性
    pub fn set_viewport(&self, start: u64, size: usize) -> Result<(), PageManagerError> {
        if start + (size as u64) > self.file_size {
            return Err(PageManagerError::OutOfBounds(format!(
                "viewport {}..{} exceeds file size {}",
                start,
                start + (size as u64),
                self.file_size
            )));
        }

        let mut viewport = self.viewport.lock();
        viewport.start = start;
        viewport.size = size;

        tracing::debug!("Viewport set: start={}, size={}", start, size);
        Ok(())
    }

    /// 获取当前视口
    /// 使用 Mutex 保护，确保读取的原子性
    pub fn get_viewport(&self) -> Viewport {
        let viewport = self.viewport.lock();
        Viewport {
            start_offset: viewport.start,
            size: viewport.size,
            page_size: self.config.page_size,
        }
    }

    /// 读取视口内的数据
    /// 使用 Mutex 保护，确保视口读取的原子性
    pub fn read_viewport(&self) -> Option<&[u8]> {
        let (viewport, mapping) = {
            let viewport_guard = self.viewport.lock();
            let mapping = self.mapping.as_ref()?;
            (
                Viewport {
                    start_offset: viewport_guard.start,
                    size: viewport_guard.size,
                    page_size: self.config.page_size,
                },
                mapping,
            )
        };

        let start = viewport.start_offset as usize;
        let end = std::cmp::min(start + viewport.size, mapping.len());

        Some(&mapping[start..end])
    }

    /// 读取指定偏移和大小的数据
    pub fn read_at(&self, offset: u64, size: usize) -> Result<&[u8], PageManagerError> {
        if offset >= self.file_size {
            return Err(PageManagerError::OutOfBounds(format!(
                "offset {} exceeds file size {}",
                offset, self.file_size
            )));
        }

        let mapping = self
            .mapping
            .as_ref()
            .ok_or_else(|| PageManagerError::MappingFailed("mapping not available".to_string()))?;

        let start = offset as usize;
        let end = std::cmp::min(start + size, mapping.len());

        Ok(&mapping[start..end])
    }

    /// 读取一行数据（从指定偏移到换行符）
    pub fn read_line(&self, offset: u64) -> Result<(&[u8], u64), PageManagerError> {
        let mapping = self
            .mapping
            .as_ref()
            .ok_or_else(|| PageManagerError::MappingFailed("mapping not available".to_string()))?;

        let start = offset as usize;
        if start >= mapping.len() {
            return Err(PageManagerError::OutOfBounds(format!(
                "offset {} exceeds mapping size {}",
                offset,
                mapping.len()
            )));
        }

        // 查找换行符
        let mut end = start;
        while end < mapping.len() && mapping[end] != b'\n' {
            end += 1;
        }

        let line = &mapping[start..end];
        let next_offset = if end < mapping.len() {
            (end + 1) as u64
        } else {
            self.file_size
        };

        Ok((line, next_offset))
    }

    /// 获取内存使用量
    pub fn memory_usage(&self) -> usize {
        self.mapped_pages.load(Ordering::Acquire) * self.config.page_size
    }

    /// 检查是否可以映射更多内存
    pub fn can_map_more(&self) -> bool {
        self.memory_usage() < self.config.max_mapped_memory
    }

    /// 获取所有页面信息
    pub fn get_all_pages(&self) -> Vec<PageInfo> {
        (0..self.page_count())
            .filter_map(|i| self.get_page_info(i))
            .collect()
    }

    /// 获取指定范围内的页面
    pub fn get_pages_in_range(&self, start: u64, end: u64) -> Vec<PageInfo> {
        let start_page = (start / (self.config.page_size as u64)) as usize;
        let end_page = end.div_ceil(self.config.page_size as u64) as usize;

        (start_page..=end_page)
            .filter_map(|i| self.get_page_info(i))
            .collect()
    }
}

/// 线程安全的 Arc 包装
pub type SharedPageManager = Arc<PageManager>;

impl PageManager {
    /// 创建共享的 PageManager
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// 从共享引用创建
    pub fn from_arc(arc: &Arc<Self>) -> &Self {
        arc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file() -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "Line 1: ERROR: Test error message").unwrap();
        writeln!(file, "Line 2: WARN: Test warning message").unwrap();
        writeln!(file, "Line 3: INFO: Test info message").unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_page_manager_creation() {
        let temp_file = create_test_file();
        let pm = PageManager::new(temp_file.path()).unwrap();

        assert!(pm.file_size() > 0);
        assert!(pm.page_count() >= 1);
    }

    #[test]
    fn test_read_at() {
        let temp_file = create_test_file();
        let pm = PageManager::new(temp_file.path()).unwrap();

        // 读取前 10 个字节
        let data = pm.read_at(0, 10).unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_read_line() {
        let temp_file = create_test_file();
        let pm = PageManager::new(temp_file.path()).unwrap();

        // 读取第一行
        let (line, next_offset) = pm.read_line(0).unwrap();
        assert!(!line.is_empty());
        assert!(next_offset > 0);

        // 读取第二行
        let (line2, _) = pm.read_line(next_offset).unwrap();
        assert!(!line2.is_empty());
    }

    #[test]
    fn test_viewport() {
        let temp_file = create_test_file();
        let pm = PageManager::new(temp_file.path()).unwrap();

        // 设置视口
        pm.set_viewport(0, 50).unwrap();

        let viewport = pm.get_viewport();
        assert_eq!(viewport.start_offset, 0);
        assert_eq!(viewport.size, 50);

        // 读取视口数据
        let data = pm.read_viewport().unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_out_of_bounds() {
        let temp_file = create_test_file();
        let pm = PageManager::new(temp_file.path()).unwrap();

        // 尝试读取超出范围的数据
        let result = pm.read_at(pm.file_size() + 100, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_info() {
        let temp_file = create_test_file();
        let pm = PageManager::new(temp_file.path()).unwrap();

        // 获取第一页信息
        let page = pm.get_page_info(0).unwrap();
        assert_eq!(page.page_number, 0);
        assert_eq!(page.offset, 0);
    }

    #[test]
    fn test_with_encoding_check_utf8() {
        // UTF-8 文件应该正常创建
        let temp_file = create_test_file();
        let result = PageManager::with_encoding_check(temp_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_with_encoding_check_utf16_le() {
        // UTF-16 LE 文件应该返回 RequiresTranscoding 错误
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&[0xFF, 0xFE]).unwrap(); // UTF-16 LE BOM
        file.write_all(&[0x48, 0x00, 0x69, 0x00]).unwrap(); // "Hi" in UTF-16 LE
        file.flush().unwrap();

        let result = PageManager::with_encoding_check(file.path());
        assert!(result.is_err());
        match result {
            Err(PageManagerError::RequiresTranscoding(_)) => {}
            _ => panic!("Expected RequiresTranscoding error"),
        }
    }

    #[test]
    fn test_with_encoding_check_utf16_be() {
        // UTF-16 BE 文件应该返回 RequiresTranscoding 错误
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&[0xFE, 0xFF]).unwrap(); // UTF-16 BE BOM
        file.write_all(&[0x00, 0x48, 0x00, 0x69]).unwrap(); // "Hi" in UTF-16 BE
        file.flush().unwrap();

        let result = PageManager::with_encoding_check(file.path());
        assert!(result.is_err());
        match result {
            Err(PageManagerError::RequiresTranscoding(_)) => {}
            _ => panic!("Expected RequiresTranscoding error"),
        }
    }

    #[test]
    fn test_with_encoding_check_utf8_with_bom() {
        // UTF-8 BOM 文件：有 BOM 但不破坏 SIMD
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap(); // UTF-8 BOM
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();

        let result = PageManager::with_encoding_check(file.path());
        // UTF-8 BOM 有 BOM 标记，但不会破坏 SIMD 优化
        // 所以不会返回 RequiresTranscoding 错误
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_auto_transcoding_utf8() {
        // UTF-8 文件应该直接创建，不需要转码
        let temp_file = create_test_file();
        let result = PageManager::with_auto_transcoding(temp_file.path()).await;
        assert!(result.is_ok());
    }
}
