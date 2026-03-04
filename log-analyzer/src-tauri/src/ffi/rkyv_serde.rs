//! rkyv 零拷贝序列化模块
//!
//! 提供基于 rkyv 的零拷贝 FFI 数据传输实现。
//! 参考 PRD V6.0: "基于 rkyv 的极端零拷贝穿透"
//!
//! ## 架构说明
//!
//! 传统 FFI 通信需要将 Rust 堆数据序列化后再拷贝到 Dart 堆。
//! 本模块使用 rkyv 实现:
//! - 内存对齐的二进制结构就地格式化
//! - 原始内存指针直接暴露给 Dart
//! - 实现物理意义上的 0 字节拷贝
//!
//! ## 使用方式
//!
//! ```rust
//! use crate::ffi::rkyv_serde::{serialize_to_buffer, deserialize_from_buffer};
//!
//! // 序列化 (Rust -> Dart)
//! let tokens = vec![HighlightToken::keyword(0, 5)];
//! let buffer = serialize_to_buffer(&tokens).unwrap();
//!
//! // 反序列化 (Dart -> Rust)
//! let tokens: Vec<HighlightToken> = deserialize_from_buffer(&buffer).unwrap();
//! ```

use rkyv::{Archive, Deserialize, Serialize};
use std::marker::PhantomData;
use std::ptr::NonNull;

pub use super::types::{
    DensityMap, FfiPointer, HighlightToken, LineContent, RowData, SearchProgress,
    SerializedData, ViewportData, ZeroCopyBuffer,
};

/// 序列化错误类型
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("序列化失败: {0}")]
    SerializationFailed(String),

    #[error("反序列化失败: {0}")]
    DeserializationFailed(String),

    #[error("指针无效: {0}")]
    InvalidPointer(String),

    #[error("内存对齐错误: {0}")]
    AlignmentError(String),
}

pub type Result<T> = std::result::Result<T, SerializeError>;

/// 序列化器配置
#[derive(Debug, Clone)]
pub struct SerializerConfig {
    /// 是否使用压缩
    pub use_compression: bool,
    /// 压缩级别
    pub compression_level: u32,
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self {
            use_compression: false, // 零拷贝优先，不压缩
            compression_level: 6,
        }
    }
}

/// rkyv 序列化器
///
/// 提供便捷的序列化和反序列化方法
pub struct RkyvSerializer {
    config: SerializerConfig,
}

impl RkyvSerializer {
    /// 创建新的序列化器
    pub fn new(config: SerializerConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建序列化器
    pub fn default() -> Self {
        Self::new(SerializerConfig::default())
    }

    /// 序列化数据到字节向量
    ///
    /// 返回的字节向量可直接传递给 FFI 层
    pub fn serialize<T: Archive + Serialize<rkyv::Scratch>>(
        &self,
        value: &T,
    ) -> Result<Vec<u8>> {
        // 使用 rkyv 序列化到堆分配器
        let mut serializer = rkyv::ser::Serializer::<rkyv::ser::HeapSerializer>::default();
        
        serializer
            .serialize(value)
            .map_err(|e| SerializeError::SerializationFailed(e.to_string()))
    }

    /// 序列化数据到零拷贝缓冲区
    pub fn serialize_to_buffer<T: Archive + Serialize<rkyv::Scratch>>(
        &self,
        value: &T,
    ) -> Result<ZeroCopyBuffer> {
        let data = self.serialize(value)?;
        Ok(ZeroCopyBuffer::new(data))
    }

    /// 从字节向量反序列化
    pub fn deserialize<T: Archive + Deserialize<T, rkyv::AmbiguousRoot>>(
        &self,
        data: &[u8],
    ) -> Result<T> {
        // 使用 ambiguous root 允许灵活的反序列化
        rkyv::from_bytes(data).map_err(|e| SerializeError::DeserializationFailed(e.to_string()))
    }

    /// 获取序列化的字节长度
    pub fn serialized_size<T: Archive>(&self, value: &T) -> Result<usize> {
        // 使用 Archived 类型获取固定大小
        let archived = rkyv::to_bytes::<_, 256>(value)
            .map_err(|e| SerializeError::SerializationFailed(e.to_string()))?;
        Ok(archived.len())
    }
}

impl Default for RkyvSerializer {
    fn default() -> Self {
        Self::default()
    }
}

/// 将数据序列化为零拷贝缓冲区
///
/// 这是主要的 API，供 FFI 层调用
pub fn serialize_to_buffer<T: Archive + Serialize<rkyv::Scratch>>(
    value: &T,
) -> Result<ZeroCopyBuffer> {
    RkyvSerializer::default().serialize_to_buffer(value)
}

/// 从缓冲区反序列化数据
pub fn deserialize_from_buffer<T: Archive + Deserialize<T, rkyv::AmbiguousRoot>>(
    buffer: &[u8],
) -> Result<T> {
    RkyvSerializer::default().deserialize(buffer)
}

/// 创建安全的 FFI 指针
///
/// 返回可用于跨 FFI 边界的指针信息
pub fn create_ffi_pointer(data: &[u8]) -> FfiPointer {
    FfiPointer {
        address: data.as_ptr() as u64,
        length: data.len() as u64,
    }
}

/// 从 FFI 指针创建安全的数据视图
///
/// 仅在确认指针有效时使用
pub unsafe fn from_ffi_pointer<T: Archive>(
    ptr: u64,
    length: u64,
) -> Result<NonNull<u8>> {
    if ptr == 0 {
        return Err(SerializeError::InvalidPointer("pointer is null".to_string()));
    }

    if length == 0 {
        return Err(SerializeError::InvalidPointer("length is zero".to_string()));
    }

    // 验证指针对齐
    let ptr = ptr as *const u8;
    if ptr as usize % std::mem::align_of::<T>() != 0 {
        return Err(SerializeError::AlignmentError(format!(
            "pointer not aligned for type {}",
            std::any::type_name::<T>()
        )));
    }

    Ok(NonNull::new(ptr as *mut u8).expect("pointer should be non-null"))
}

// ================================================================================================
// 专用类型序列化便捷函数
// ================================================================================================

/// 序列化高亮 Token 列表
pub fn serialize_highlights(tokens: &[HighlightToken]) -> Result<ZeroCopyBuffer> {
    serialize_to_buffer(tokens)
}

/// 反序列化高亮 Token 列表
pub fn deserialize_highlights(data: &[u8]) -> Result<Vec<HighlightToken>> {
    deserialize_from_buffer(data)
}

/// 序列化搜索进度
pub fn serialize_search_progress(progress: &SearchProgress) -> Result<ZeroCopyBuffer> {
    serialize_to_buffer(progress)
}

/// 从命中行号创建密度图
pub fn create_density_map(total_lines: u64, hit_lines: &[u64], viewport_height: u32) -> DensityMap {
    DensityMap::from_hits(total_lines, hit_lines, viewport_height)
}

/// 序列化密度图
pub fn serialize_density_map(map: &DensityMap) -> Result<ZeroCopyBuffer> {
    serialize_to_buffer(map)
}

// ================================================================================================
// 单元测试
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_token_serialization() {
        let tokens = vec![
            HighlightToken::keyword(0, 5),
            HighlightToken::timestamp(6, 23),
            HighlightToken::level(30, 5),
        ];

        let buffer = serialize_to_buffer(&tokens).unwrap();
        let deserialized: Vec<HighlightToken> = deserialize_from_buffer(&buffer).unwrap();

        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized[0].token_type, 1); // keyword
        assert_eq!(deserialized[1].token_type, 2); // timestamp
        assert_eq!(deserialized[2].token_type, 3); // level
    }

    #[test]
    fn test_search_progress_serialization() {
        let progress = SearchProgress {
            query_id: 12345,
            total_hits: 1000,
            is_done: true,
            gpu_texture_map: vec![10, 20, 30, 40, 50],
            hit_positions: vec![1, 5, 10, 15, 20],
        };

        let buffer = serialize_to_buffer(&progress).unwrap();
        let deserialized: SearchProgress = deserialize_from_buffer(&buffer).unwrap();

        assert_eq!(deserialized.query_id, 12345);
        assert_eq!(deserialized.total_hits, 1000);
        assert!(deserialized.is_done);
        assert_eq!(deserialized.gpu_texture_map.len(), 5);
    }

    #[test]
    fn test_density_map_creation() {
        let total_lines = 1000u64;
        let hit_lines = vec![1, 5, 10, 15, 20, 100, 200, 300];
        let viewport_height = 50;

        let density_map = create_density_map(total_lines, &hit_lines, viewport_height);

        assert_eq!(density_map.total_lines, 1000);
        assert_eq!(density_map.sampled_lines, 50);
        assert!(!density_map.densities.is_empty());
    }

    #[test]
    fn test_zero_copy_buffer() {
        let data = vec![1u8, 2, 3, 4, 5];
        let buffer = ZeroCopyBuffer::new(data.clone());

        assert_eq!(buffer.len(), 5);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.as_slice(), &data);
    }

    #[test]
    fn test_serializer_config() {
        let config = SerializerConfig::default();
        assert!(!config.use_compression);
        assert_eq!(config.compression_level, 6);
    }

    #[test]
    fn test_custom_serializer() {
        let config = SerializerConfig {
            use_compression: false,
            compression_level: 3,
        };
        let serializer = RkyvSerializer::new(config);

        let tokens = vec![HighlightToken::plain(0, 10)];
        let buffer = serializer.serialize_to_buffer(&tokens).unwrap();

        assert!(buffer.len() > 0);
    }

    #[test]
    fn test_serialization_size() {
        let serializer = RkyvSerializer::default();
        let tokens = vec![HighlightToken::keyword(0, 5); 10];
        
        let size = serializer.serialized_size(&tokens).unwrap();
        assert!(size > 0);
        
        let buffer = serializer.serialize(&tokens).unwrap();
        assert_eq!(buffer.len(), size);
    }
}
