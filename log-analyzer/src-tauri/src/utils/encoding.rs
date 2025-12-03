//! 字符编码转换工具
//!
//! 处理多种字符编码（UTF-8、GBK、Windows-1252），主要用于压缩文件中的文件名解码。

/// 解码文件名字节序列
///
/// 尝试多种编码格式解码文件名，优先使用 UTF-8，回退到 GBK 和 Windows-1252。
///
/// # 参数
///
/// - `bytes` - 原始字节序列
///
/// # 返回值
///
/// 解码后的字符串
///
/// # 编码尝试顺序
///
/// 1. UTF-8（国际通用）
/// 2. GBK（中文 Windows）
/// 3. Windows-1252（西文 Windows）
///
/// # 示例
///
/// ```ignore
/// let bytes = b"\xc4\xe3\xba\xc3"; // GBK 编码的"你好"
/// let filename = decode_filename(bytes);
/// assert_eq!(filename, "你好");
/// ```
pub fn decode_filename(bytes: &[u8]) -> String {
    // 尝试 UTF-8
    let (cow, _, had_errors) = encoding_rs::UTF_8.decode(bytes);
    if !had_errors && !cow.contains('\u{FFFD}') {
        return cow.into_owned();
    }

    // 尝试 GBK (中文 Windows)
    let (cow_gbk, _, had_errors_gbk) = encoding_rs::GBK.decode(bytes);
    if !had_errors_gbk {
        return cow_gbk.into_owned();
    }

    // Windows-1252 (西文 Windows)
    let (cow_win, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    cow_win.into_owned()
}
