//! 文件类型过滤器
//!
//! 从主 crate services/file_type_filter.rs 复制。
//! 三层检测策略过滤非日志文件：
//! - 第1层：二进制文件快速检测（魔数 + 空字节比例）
//! - 第2层：文件名 Glob 模式匹配
//! - 第3层：扩展名白名单/黑名单

use la_core::models::config::{FileFilterConfig, FilterMode};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 预编译的 glob 正则
struct CompiledGlob {
    #[allow(dead_code)]
    original_pattern: String,
    regex: regex::Regex,
}

/// 文件类型过滤器（三层检测策略）
pub struct FileTypeFilter {
    config: FileFilterConfig,
    compiled_globs: Vec<CompiledGlob>,
}

impl FileTypeFilter {
    /// 创建新的过滤器
    pub fn new(config: FileFilterConfig) -> Self {
        let compiled_globs: Vec<CompiledGlob> = config
            .filename_patterns
            .iter()
            .filter_map(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                let regex_pattern = pattern_lower
                    .replace('.', r"\.")
                    .replace('*', ".*")
                    .replace('?', ".");
                regex::Regex::new(&regex_pattern)
                    .ok()
                    .map(|regex| CompiledGlob {
                        original_pattern: pattern.clone(),
                        regex,
                    })
            })
            .collect();

        Self {
            config,
            compiled_globs,
        }
    }

    /// 检查文件是否应该被导入（三层检测）
    /// 返回 true 表示允许导入，false 表示跳过
    pub fn should_import_file(&self, path: &Path) -> bool {
        self.should_import_file_safe(path).unwrap_or(true)
    }

    /// 检查文件是否应该被导入（防御性版本，返回 Result）
    pub fn should_import_file_safe(&self, path: &Path) -> Result<bool, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if self.config.binary_detection_enabled {
                match self.detect_binary_file(path) {
                    Ok(is_binary) => {
                        if is_binary {
                            tracing::debug!(
                                file = %path.display(),
                                "File skipped: detected as binary"
                            );
                            return Ok(false);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            file = %path.display(),
                            error = %e,
                            "Binary detection failed, proceeding with filter rules"
                        );
                    }
                }
            }

            if !self.config.enabled {
                return Ok(true);
            }

            Ok(self.apply_filter_rules(path))
        }))
        .map_err(|e| {
            tracing::error!(
                file = %path.display(),
                panic_info = ?e,
                "File filter logic panicked, allowing file (fail-safe)"
            );
            "File filter logic panicked".to_string()
        })?
    }

    /// 第1层：二进制文件检测（仅读取前1KB）
    fn detect_binary_file(&self, path: &Path) -> std::io::Result<bool> {
        const MAGIC_NUMBERS: &[(&[u8], &str)] = &[
            (&[0xFF, 0xD8, 0xFF], "JPEG"),
            (&[0x89, 0x50, 0x4E, 0x47], "PNG"),
            (&[0x47, 0x49, 0x46], "GIF"),
            (&[0x42, 0x4D], "BMP"),
            (&[0x49, 0x49, 0x2A, 0x00], "TIFF"),
            (&[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70], "MP4"),
            (&[0x1A, 0x45, 0xDF, 0xA3], "MKV"),
            (&[0x49, 0x44, 0x33], "MP3"),
            (&[0xFF, 0xFB], "MP3"),
            (&[0x52, 0x49, 0x46, 0x46], "WAV/AVI"),
            (&[0x4D, 0x5A], "EXE"),
            (&[0x7F, 0x45, 0x4C, 0x46], "ELF"),
            (&[0xFE, 0xED, 0xFA, 0xCF], "Mach-O"),
            (&[0x50, 0x4B, 0x03, 0x04], "ZIP"),
            (&[0x1F, 0x8B], "GZ"),
        ];

        let mut file = File::open(path)?;
        let mut buffer = [0u8; 1024];
        let n = file.read(&mut buffer)?;

        if n == 0 {
            return Ok(false);
        }

        for &(magic, name) in MAGIC_NUMBERS {
            if buffer.starts_with(magic) {
                tracing::debug!(
                    file = %path.display(),
                    file_type = name,
                    "Detected binary file by magic number"
                );
                return Ok(true);
            }
        }

        let null_count = buffer[..n].iter().filter(|&&b| b == 0).count();
        let null_ratio = null_count as f64 / n as f64;

        if null_ratio > 0.05 {
            tracing::debug!(
                file = %path.display(),
                null_ratio = null_ratio,
                "Detected binary file by null byte ratio"
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// 第2层：应用智能过滤规则
    fn apply_filter_rules(&self, path: &Path) -> bool {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            let file_name_lower = file_name.to_lowercase();
            for compiled in &self.compiled_globs {
                if compiled.regex.is_match(&file_name_lower) {
                    return true;
                }
            }
        }

        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = extension.to_lowercase();

            match self.config.mode {
                FilterMode::Whitelist => {
                    if self.config.allowed_extensions.is_empty() {
                        return true;
                    }
                    return self
                        .config
                        .allowed_extensions
                        .iter()
                        .any(|allowed| allowed.eq_ignore_ascii_case(&ext_lower));
                }
                FilterMode::Blacklist => {
                    return !self
                        .config
                        .forbidden_extensions
                        .iter()
                        .any(|forbidden| forbidden.eq_ignore_ascii_case(&ext_lower));
                }
            }
        }

        match self.config.mode {
            FilterMode::Whitelist => false,
            FilterMode::Blacklist => true,
        }
    }
}
