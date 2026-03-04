//! 编码检测器模块
//!
//! 使用 chardetng 库探测文件编码，支持：
//! - UTF-8（含 BOM 和无 BOM）
//! - UTF-16 LE/BE
//! - GBK/GB18030（中文编码）
//! - Shift-JIS（日文编码）
//! - ISO-8859 系列（西文编码）
//!
//! PRD 2.4 要求：遭遇 UTF-16 等导致 SIMD 失效的编码时，立刻中断 Mmap

use encoding_rs::Encoding;

/// 检测采样大小（从文件开头读取的字节数）
const DETECTION_SAMPLE_SIZE: usize = 4096;

/// 会破坏 SIMD 优化的编码名称列表
/// UTF-16 和 UTF-32 使用多字节字符单元，无法使用 SIMD 字节扫描优化
const SIMD_BREAKING_ENCODING_NAMES: &[&str] = &["UTF-16LE", "UTF-16BE", "UTF-32LE", "UTF-32BE"];

/// 编码检测结果
#[derive(Debug, Clone)]
pub struct EncodingDetectionResult {
    /// 检测到的编码
    pub encoding: &'static Encoding,
    /// 编码名称
    pub encoding_name: String,
    /// 是否需要转码（非 UTF-8）
    pub needs_transcoding: bool,
    /// 是否会破坏 SIMD 优化（UTF-16/UTF-32 等）
    pub breaks_simd: bool,
    /// 检测置信度（0.0 - 1.0）
    pub confidence: f32,
    /// 是否检测到 BOM
    pub has_bom: bool,
}

/// 编码检测器
///
/// 使用 chardetng 进行编码探测，支持多种常见编码格式。
pub struct EncodingDetector;

impl EncodingDetector {
    /// 检测文件编码
    ///
    /// 从文件开头读取样本数据，使用 chardetng 探测编码。
    /// 同时检测 BOM 标记以识别 UTF-16/UTF-32 等编码。
    ///
    /// # 参数
    ///
    /// - `data`: 文件开头的数据样本（建议至少 4KB）
    ///
    /// # 返回值
    ///
    /// 返回编码检测结果，包含编码信息和是否需要转码
    ///
    /// # 示例
    ///
    /// ```
    /// use log_analyzer::utils::encoding_detector::EncodingDetector;
    ///
    /// let utf8_data = b"Hello, World!";
    /// let result = EncodingDetector::detect(utf8_data);
    /// assert_eq!(result.encoding_name, "UTF-8");
    /// assert!(!result.needs_transcoding);
    /// ```
    pub fn detect(data: &[u8]) -> EncodingDetectionResult {
        // 空数据处理
        if data.is_empty() {
            return EncodingDetectionResult {
                encoding: encoding_rs::UTF_8,
                encoding_name: "UTF-8".to_string(),
                needs_transcoding: false,
                breaks_simd: false,
                confidence: 1.0,
                has_bom: false,
            };
        }

        // 首先检查 BOM（字节顺序标记）
        if let Some(bom_result) = Self::detect_bom(data) {
            tracing::debug!(
                encoding = %bom_result.encoding.name(),
                "检测到 BOM 编码"
            );
            return bom_result;
        }

        // 使用 chardetng 探测编码
        let mut detector = chardetng::EncodingDetector::new();
        detector.feed(data, data.len() >= DETECTION_SAMPLE_SIZE);
        let encoding = detector.guess(None, true);

        // 判断编码特性
        let encoding_name = encoding.name().to_string();
        let needs_transcoding = !Self::is_utf8_compatible(encoding);
        let breaks_simd = Self::breaks_simd_optimizations(encoding);

        // 估算置信度（chardetng 不提供置信度，我们根据数据特征估算）
        let confidence = Self::estimate_confidence(data, encoding);

        tracing::debug!(
            encoding = %encoding_name,
            needs_transcoding = needs_transcoding,
            breaks_simd = breaks_simd,
            confidence = confidence,
            "编码检测完成"
        );

        EncodingDetectionResult {
            encoding,
            encoding_name,
            needs_transcoding,
            breaks_simd,
            confidence,
            has_bom: false,
        }
    }

    /// 从文件路径检测编码
    ///
    /// 读取文件开头的数据进行编码检测。
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回值
    ///
    /// 成功返回编码检测结果，失败返回 IO 错误
    pub fn detect_from_file(path: &std::path::Path) -> std::io::Result<EncodingDetectionResult> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut buffer = vec![0u8; DETECTION_SAMPLE_SIZE];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        Ok(Self::detect(&buffer))
    }

    /// 异步从文件路径检测编码
    ///
    /// # 参数
    ///
    /// - `path`: 文件路径
    ///
    /// # 返回值
    ///
    /// 成功返回编码检测结果，失败返回 IO 错误
    pub async fn detect_from_file_async(
        path: &std::path::Path,
    ) -> std::io::Result<EncodingDetectionResult> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(path).await?;
        let mut buffer = vec![0u8; DETECTION_SAMPLE_SIZE];
        let bytes_read = file.read(&mut buffer[..]).await?;
        buffer.truncate(bytes_read);

        Ok(Self::detect(&buffer))
    }

    /// 检测 BOM（字节顺序标记）
    ///
    /// BOM 优先级最高，可以准确识别 UTF-8/UTF-16/UTF-32 编码
    fn detect_bom(data: &[u8]) -> Option<EncodingDetectionResult> {
        // UTF-8 BOM: EF BB BF
        if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Some(EncodingDetectionResult {
                encoding: encoding_rs::UTF_8,
                encoding_name: "UTF-8-BOM".to_string(),
                needs_transcoding: false, // UTF-8 不需要转码，只需跳过 BOM
                breaks_simd: false,
                confidence: 1.0,
                has_bom: true,
            });
        }

        // UTF-32 LE BOM: FF FE 00 00
        // encoding_rs 不直接支持 UTF-32，使用 for_label 查找
        if data.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
            let encoding = Encoding::for_label(b"UTF-32LE").unwrap_or(encoding_rs::UTF_8);
            return Some(EncodingDetectionResult {
                encoding,
                encoding_name: "UTF-32LE".to_string(),
                needs_transcoding: true,
                breaks_simd: true,
                confidence: 1.0,
                has_bom: true,
            });
        }

        // UTF-32 BE BOM: 00 00 FE FF
        if data.starts_with(&[0x00, 0x00, 0xFE, 0xFF]) {
            let encoding = Encoding::for_label(b"UTF-32BE").unwrap_or(encoding_rs::UTF_8);
            return Some(EncodingDetectionResult {
                encoding,
                encoding_name: "UTF-32BE".to_string(),
                needs_transcoding: true,
                breaks_simd: true,
                confidence: 1.0,
                has_bom: true,
            });
        }

        // UTF-16 LE BOM: FF FE (且不是 UTF-32 LE)
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
            // 排除 UTF-32 LE 的情况（已在上面处理）
            if data.len() < 4 || !(data[2] == 0x00 && data[3] == 0x00) {
                return Some(EncodingDetectionResult {
                    encoding: encoding_rs::UTF_16LE,
                    encoding_name: "UTF-16LE".to_string(),
                    needs_transcoding: true,
                    breaks_simd: true,
                    confidence: 1.0,
                    has_bom: true,
                });
            }
        }

        // UTF-16 BE BOM: FE FF
        if data.starts_with(&[0xFE, 0xFF]) {
            return Some(EncodingDetectionResult {
                encoding: encoding_rs::UTF_16BE,
                encoding_name: "UTF-16BE".to_string(),
                needs_transcoding: true,
                breaks_simd: true,
                confidence: 1.0,
                has_bom: true,
            });
        }

        None
    }

    /// 检查编码是否与 UTF-8 兼容（不需要转码）
    ///
    /// UTF-8、ASCII 兼容编码不需要转码
    pub fn is_utf8_compatible(encoding: &'static Encoding) -> bool {
        let name = encoding.name();
        matches!(name, "UTF-8" | "ASCII" | "utf-8" | "ascii")
    }

    /// 检查编码是否会破坏 SIMD 优化
    ///
    /// UTF-16 和 UTF-32 使用多字节表示字符，会导致：
    /// 1. SIMD 字节扫描失效
    /// 2. 内存映射（Mmap）需要特殊处理
    /// 3. 行分割逻辑需要重新实现
    pub fn breaks_simd_optimizations(encoding: &'static Encoding) -> bool {
        let name = encoding.name();
        SIMD_BREAKING_ENCODING_NAMES.contains(&name)
    }

    /// 检测是否需要转码
    pub fn needs_transcoding(encoding: &'static Encoding) -> bool {
        !Self::is_utf8_compatible(encoding)
    }

    /// 估算检测置信度
    ///
    /// 基于数据特征估算编码检测的置信度：
    /// - BOM 标记：置信度 1.0
    /// - 纯 ASCII：置信度 1.0
    /// - 有效 UTF-8：置信度 0.9
    /// - 其他：置信度 0.5-0.8
    fn estimate_confidence(data: &[u8], encoding: &'static Encoding) -> f32 {
        // 检查是否为纯 ASCII
        if data.iter().all(|&b| b.is_ascii()) {
            return 1.0;
        }

        // 检查是否为有效 UTF-8
        if Self::is_utf8_compatible(encoding) {
            // 尝试解码，检查是否有错误
            let (decoded, _, had_errors) = encoding_rs::UTF_8.decode(data);
            if !had_errors && !decoded.contains('\u{FFFD}') {
                return 0.9;
            }
            // 有错误但检测为 UTF-8，置信度较低
            return 0.7;
        }

        // 非 UTF-8 编码，根据数据特征估算
        // 检查是否包含大量非 ASCII 字节
        let non_ascii_ratio =
            data.iter().filter(|&&b| !b.is_ascii()).count() as f32 / data.len().max(1) as f32;

        // 非 ASCII 字节越多，置信度越高（因为编码特征更明显）
        0.5 + non_ascii_ratio * 0.3
    }

    /// 获取编码的名称
    ///
    /// 返回标准的字符集名称，适用于 HTTP Content-Type 头
    pub fn get_mime_name(encoding: &'static Encoding) -> &'static str {
        encoding.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8_pure_ascii() {
        let data = b"Hello, World! This is a test.";
        let result = EncodingDetector::detect(data);

        assert_eq!(result.encoding_name, "UTF-8");
        assert!(!result.needs_transcoding);
        assert!(!result.breaks_simd);
        assert!(!result.has_bom);
    }

    #[test]
    fn test_detect_utf8_with_chinese() {
        // UTF-8 编码的中文
        let data = "你好，世界！Hello World!".as_bytes();
        let result = EncodingDetector::detect(data);

        assert!(!result.needs_transcoding);
        assert!(!result.breaks_simd);
    }

    #[test]
    fn test_detect_utf8_with_bom() {
        // UTF-8 BOM + 内容
        let data = &[0xEF, 0xBB, 0xBF, b'H', b'e', b'l', b'l', b'o'];
        let result = EncodingDetector::detect(data);

        assert!(result.has_bom);
        assert_eq!(result.encoding_name, "UTF-8-BOM");
        assert!(!result.needs_transcoding);
        assert!(!result.breaks_simd);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_detect_utf16_le_with_bom() {
        // UTF-16 LE BOM: FF FE
        let data = &[0xFF, 0xFE, b'H', 0x00, b'i', 0x00];
        let result = EncodingDetector::detect(data);

        assert!(result.has_bom);
        assert_eq!(result.encoding_name, "UTF-16LE");
        assert!(result.needs_transcoding);
        assert!(result.breaks_simd);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_detect_utf16_be_with_bom() {
        // UTF-16 BE BOM: FE FF
        let data = &[0xFE, 0xFF, 0x00, b'H', 0x00, b'i'];
        let result = EncodingDetector::detect(data);

        assert!(result.has_bom);
        assert_eq!(result.encoding_name, "UTF-16BE");
        assert!(result.needs_transcoding);
        assert!(result.breaks_simd);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_detect_utf32_le_with_bom() {
        // UTF-32 LE BOM: FF FE 00 00
        let data = &[0xFF, 0xFE, 0x00, 0x00, b'H', 0x00, 0x00, 0x00];
        let result = EncodingDetector::detect(data);

        assert!(result.has_bom);
        assert_eq!(result.encoding_name, "UTF-32LE");
        assert!(result.needs_transcoding);
        assert!(result.breaks_simd);
    }

    #[test]
    fn test_detect_empty_data() {
        let data: &[u8] = b"";
        let result = EncodingDetector::detect(data);

        assert_eq!(result.encoding_name, "UTF-8");
        assert!(!result.needs_transcoding);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_is_utf8_compatible() {
        assert!(EncodingDetector::is_utf8_compatible(encoding_rs::UTF_8));
        assert!(!EncodingDetector::is_utf8_compatible(encoding_rs::GBK));
        assert!(!EncodingDetector::is_utf8_compatible(encoding_rs::UTF_16LE));
    }

    #[test]
    fn test_breaks_simd_optimizations() {
        assert!(!EncodingDetector::breaks_simd_optimizations(
            encoding_rs::UTF_8
        ));
        assert!(EncodingDetector::breaks_simd_optimizations(
            encoding_rs::UTF_16LE
        ));
        assert!(EncodingDetector::breaks_simd_optimizations(
            encoding_rs::UTF_16BE
        ));
        // UTF-32 通过 for_label 获取（encoding_rs 不直接导出这些常量）
        let utf32le = Encoding::for_label(b"UTF-32LE").unwrap();
        assert!(EncodingDetector::breaks_simd_optimizations(utf32le));
    }

    #[test]
    fn test_needs_transcoding() {
        assert!(!EncodingDetector::needs_transcoding(encoding_rs::UTF_8));
        assert!(EncodingDetector::needs_transcoding(encoding_rs::GBK));
        assert!(EncodingDetector::needs_transcoding(encoding_rs::UTF_16LE));
    }

    #[test]
    fn test_get_mime_name() {
        assert_eq!(EncodingDetector::get_mime_name(encoding_rs::UTF_8), "UTF-8");
        assert_eq!(EncodingDetector::get_mime_name(encoding_rs::GBK), "GBK");
    }

    #[test]
    fn test_detect_gbk_sample() {
        // GBK 编码的 "你好" = C4 E3 BA C3
        let data = &[0xC4, 0xE3, 0xBA, 0xC3];
        let result = EncodingDetector::detect(data);

        // chardetng 应该检测出 GBK 或类似编码
        assert!(result.needs_transcoding);
    }

    #[test]
    fn test_estimate_confidence_pure_ascii() {
        let data = b"Pure ASCII text without any special characters.";
        let confidence = EncodingDetector::estimate_confidence(data, encoding_rs::UTF_8);

        assert_eq!(confidence, 1.0);
    }

    #[test]
    fn test_estimate_confidence_valid_utf8() {
        let data = "Valid UTF-8: 你好世界".as_bytes();
        let confidence = EncodingDetector::estimate_confidence(data, encoding_rs::UTF_8);

        assert!(confidence >= 0.9);
    }
}
