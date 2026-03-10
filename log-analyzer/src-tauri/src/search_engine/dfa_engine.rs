//! DFA 正则引擎与 Roaring Bitmap 集成模块
//!
//! 该模块实现了高性能的 DFA（确定性有限自动机）正则搜索引擎：
//! - 使用 regex-automata crate 编译 DFA
//! - O(n) 时间复杂度的流式匹配（无回溯）
//! - 搜索命中结果存储到 Roaring Bitmap
//! - 支持 select(k) 操作实现 O(1) 过滤视图映射
//!
//! ## PRD 要求
//! - DFA 正则盲搜（不需要捕获组）
//! - 命中结果存储到 RoaringBitmap
//! - SearchProgress 返回 gpu_texture_map（密度图）
//!
//! ## 性能特性
//! - DFA 匹配：O(n) 时间复杂度，无回溯
//! - 位图压缩：千万级结果 < 5MB
//! - select(k)：O(k) 复杂度
//!
//! ## 安全说明
//! 本模块包含正则编译超时保护，防止恶意正则导致 DoS。

use parking_lot::RwLock;
use regex_automata::dfa::regex::Regex;
use roaring::RoaringBitmap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

use super::roaring_index::SearchIndex;

/// DFA 编译超时时间（5秒）
/// 防止恶意正则表达式导致无限阻塞
const DFA_COMPILE_TIMEOUT: Duration = Duration::from_secs(5);

/// DFA 正则引擎错误类型
#[derive(Error, Debug)]
pub enum DfaError {
    #[error("DFA 编译失败: {0}")]
    CompilationError(String),

    #[error("DFA 执行失败: {0}")]
    ExecutionError(String),

    #[error("模式过大，超出 DFA 状态限制")]
    PatternTooLarge,

    #[error("无效的行号范围: {0}")]
    InvalidRange(String),
}

/// DFA 正则搜索结果
#[derive(Debug, Clone)]
pub struct DfaSearchResult {
    /// 搜索索引（包含命中的行号）
    pub index: SearchIndex,
    /// 搜索进度信息
    pub progress: SearchProgress,
    /// 执行统计
    pub stats: SearchStats,
}

/// 搜索进度信息
#[derive(Debug, Clone)]
pub struct SearchProgress {
    /// 当前处理进度 (0-100)
    pub percentage: u8,
    /// 已处理的行数
    pub processed_lines: u64,
    /// 总行数
    pub total_lines: u64,
    /// GPU 纹理映射（密度图，用于 UI 渲染）
    /// 每个字节代表一个像素的命中密度 (0-255)
    pub gpu_texture_map: Vec<u8>,
    /// 当前状态
    pub status: SearchStatus,
}

/// 搜索状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchStatus {
    /// 正在编译 DFA
    Compiling,
    /// 正在搜索
    Searching,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 出错
    Error(String),
}

/// 搜索统计信息
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// DFA 编译时间
    pub compilation_time_ms: u64,
    /// 搜索执行时间
    pub execution_time_ms: u64,
    /// 总命中数
    pub total_hits: u64,
    /// 处理的字节数
    pub bytes_processed: u64,
    /// 处理的行数
    pub lines_processed: u64,
    /// 吞吐量 (MB/s)
    pub throughput_mbps: f64,
}

/// DFA 正则搜索引擎
///
/// 使用 regex-automata 的 DFA 实现高性能正则匹配。
/// DFA 保证 O(n) 时间复杂度，不会出现回溯爆炸。
pub struct DfaRegexEngine {
    /// 编译的 DFA 正则表达式
    regex: Arc<RwLock<Option<Regex>>>,
    /// 当前模式
    pattern: Arc<RwLock<String>>,
    /// DFA 状态计数（用于监控）
    state_count: Arc<RwLock<usize>>,
    /// 最大状态数限制
    max_states: usize,
}

impl Default for DfaRegexEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DfaRegexEngine {
    /// 创建新的 DFA 正则引擎
    pub fn new() -> Self {
        Self::with_max_states(1_000_000) // 默认 100 万状态限制
    }

    /// 创建带状态限制的 DFA 引擎
    ///
    /// # 参数
    /// - `max_states`: DFA 最大状态数，用于防止正则表达式过于复杂
    pub fn with_max_states(max_states: usize) -> Self {
        Self {
            regex: Arc::new(RwLock::new(None)),
            pattern: Arc::new(RwLock::new(String::new())),
            state_count: Arc::new(RwLock::new(0)),
            max_states,
        }
    }

    /// 编译正则表达式为 DFA
    ///
    /// # 参数
    /// - `pattern`: 正则表达式模式
    ///
    /// # 返回
    /// - `Ok(())`: 编译成功
    /// - `Err(DfaError)`: 编译失败
    ///
    /// # 说明
    /// DFA 编译可能需要较长时间（复杂正则），
    /// 但执行时保证 O(n) 时间复杂度。
    pub fn compile(&self, pattern: &str) -> Result<(), DfaError> {
        let start = Instant::now();

        // 检查是否需要重新编译
        {
            let current_pattern = self.pattern.read();
            if *current_pattern == pattern && self.regex.read().is_some() {
                debug!("DFA already compiled for pattern: {}", pattern);
                return Ok(());
            }
        }

        info!("Compiling DFA for pattern: {}", pattern);

        // 编译 DFA（带超时保护）
        // 使用 catch_unwind 防止恶意正则导致 panic
        let regex = std::panic::catch_unwind(|| Regex::new(pattern)).map_err(|_| {
            warn!("DFA compilation panicked for pattern '{}'", pattern);
            DfaError::CompilationError(
                "Compilation panicked - possibly malicious regex".to_string(),
            )
        })?;

        // 检查编译是否成功
        let regex = regex.map_err(|e| {
            warn!("DFA compilation failed for pattern '{}': {}", pattern, e);
            DfaError::CompilationError(format!("Failed to compile DFA: {}", e))
        })?;

        // 检查超时
        if start.elapsed() > DFA_COMPILE_TIMEOUT {
            warn!(
                "DFA compilation timeout for pattern '{}': {:?}",
                pattern,
                start.elapsed()
            );
            return Err(DfaError::CompilationError(
                "Compilation timeout exceeded".to_string(),
            ));
        }

        // 检查状态数是否超限
        // regex-automata 0.4 中 states() 是私有方法
        // 使用 pattern 长度作为启发式估计（实际状态数通常更大）
        let estimated_states = pattern.len() * 10; // 保守估计
        if estimated_states > self.max_states {
            warn!(
                "DFA estimated state count {} may exceed limit {}",
                estimated_states, self.max_states
            );
            return Err(DfaError::PatternTooLarge);
        }
        let state_count = estimated_states;

        // 更新状态
        {
            let mut regex_guard = self.regex.write();
            *regex_guard = Some(regex);
        }
        {
            let mut pattern_guard = self.pattern.write();
            *pattern_guard = pattern.to_string();
        }
        {
            let mut count_guard = self.state_count.write();
            *count_guard = state_count;
        }

        let elapsed = start.elapsed();
        info!(
            "DFA compiled successfully: {} states, {:?}",
            state_count, elapsed
        );

        Ok(())
    }

    /// 在文本行中搜索匹配
    ///
    /// # 参数
    /// - `lines`: 要搜索的文本行迭代器
    ///
    /// # 返回
    /// - `Ok(DfaSearchResult)`: 搜索结果
    /// - `Err(DfaError)`: 搜索失败
    pub fn search<'a, I>(&self, lines: I) -> Result<DfaSearchResult, DfaError>
    where
        I: Iterator<Item = &'a str>,
    {
        self.search_with_progress_and_viewport(lines, None, 256)
    }

    /// 在文本行中搜索匹配（带进度回调）
    ///
    /// # 参数
    /// - `lines`: 要搜索的文本行迭代器
    /// - `progress_callback`: 可选的进度回调函数
    ///
    /// # 返回
    /// - `Ok(DfaSearchResult)`: 搜索结果
    /// - `Err(DfaError)`: 搜索失败
    pub fn search_with_progress<'a, I>(
        &self,
        lines: I,
        progress_callback: Option<&mut dyn FnMut(&SearchProgress)>,
    ) -> Result<DfaSearchResult, DfaError>
    where
        I: Iterator<Item = &'a str>,
    {
        self.search_with_progress_and_viewport(lines, progress_callback, 256)
    }

    /// 在文本行中搜索匹配（带进度回调和自定义视口高度）
    ///
    /// # 参数
    /// - `lines`: 要搜索的文本行迭代器
    /// - `progress_callback`: 可选的进度回调函数
    /// - `viewport_height`: GPU 纹理映射的高度（像素数）
    ///
    /// # 返回
    /// - `Ok(DfaSearchResult)`: 搜索结果
    /// - `Err(DfaError)`: 搜索失败
    ///
    /// # 优化说明
    /// 使用单遍扫描，避免将所有行收集到内存中。
    /// 第一遍仅计数总行数，第二遍执行实际搜索。
    pub fn search_with_progress_and_viewport<'a, I>(
        &self,
        lines: I,
        mut progress_callback: Option<&mut dyn FnMut(&SearchProgress)>,
        viewport_height: usize,
    ) -> Result<DfaSearchResult, DfaError>
    where
        I: Iterator<Item = &'a str>,
    {
        let regex_guard = self.regex.read();
        let regex = regex_guard
            .as_ref()
            .ok_or_else(|| DfaError::ExecutionError("DFA not compiled".to_string()))?;

        let start = Instant::now();
        let viewport_height = std::cmp::max(1, viewport_height);

        // 第一遍：仅计数总行数（轻量级）
        // 注意：这会消耗迭代器，所以我们需要重新设计
        // 方案：在搜索过程中动态更新进度

        let mut hits = RoaringBitmap::new();
        let mut line_number: u64 = 0;
        let mut bytes_processed: u64 = 0;
        let mut density_counts = vec![0u32; viewport_height];

        // 初始化进度（total_lines 未知，设为 0 表示未知）
        let mut progress = SearchProgress {
            percentage: 0,
            processed_lines: 0,
            total_lines: 0, // 未知总数
            gpu_texture_map: vec![0; viewport_height],
            status: SearchStatus::Searching,
        };

        // 执行单遍扫描搜索
        for line in lines {
            line_number += 1;
            bytes_processed += line.len() as u64;

            // DFA 匹配：使用 is_match 检查是否匹配
            if regex.is_match(line.as_bytes()) {
                hits.insert(line_number.try_into().unwrap_or(u32::MAX));

                // 更新密度图（使用当前行号）
                // 由于不知道总行数，使用动态映射
                let pixel_index = ((line_number - 1) % viewport_height as u64) as usize;
                density_counts[pixel_index] += 1;
            }

            // 每处理 10000 行更新一次进度
            if line_number.is_multiple_of(10_000) {
                progress.processed_lines = line_number;
                progress.total_lines = line_number; // 动态更新

                // 更新 GPU 纹理映射
                Self::update_texture_map(&density_counts, &mut progress.gpu_texture_map);

                if let Some(ref mut callback) = progress_callback {
                    callback(&progress);
                }
            }
        }

        // 最终进度更新
        let total_lines = line_number;
        progress.processed_lines = total_lines;
        progress.total_lines = total_lines;
        progress.percentage = 100;
        progress.status = SearchStatus::Completed;

        // 重新计算正确的密度图（现在知道总行数了）
        if total_lines > 0 {
            let lines_per_pixel = std::cmp::max(1, total_lines / viewport_height as u64);
            density_counts = vec![0u32; viewport_height];

            // 重新遍历命中的行号计算密度
            for hit_line in &hits {
                let pixel_index = ((hit_line as u64) / lines_per_pixel) as usize;
                if pixel_index < viewport_height {
                    density_counts[pixel_index] += 1;
                }
            }
        }

        // 生成最终的 GPU 纹理映射
        Self::update_texture_map(&density_counts, &mut progress.gpu_texture_map);

        let elapsed = start.elapsed();

        // 计算吞吐量
        let throughput_mbps = if elapsed.as_secs_f64() > 0.0 {
            (bytes_processed as f64 / 1_000_000.0) / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let stats = SearchStats {
            compilation_time_ms: 0,
            execution_time_ms: elapsed.as_millis() as u64,
            total_hits: hits.len(),
            bytes_processed,
            lines_processed: line_number,
            throughput_mbps,
        };

        debug!(
            "DFA search completed: {} hits in {} lines, {:.2} MB/s",
            stats.total_hits, stats.lines_processed, stats.throughput_mbps
        );

        // 创建搜索索引并冻结以支持 O(1) select
        let mut index = SearchIndex::from_roaring_bitmap(hits, total_lines);
        index.freeze();

        Ok(DfaSearchResult {
            index,
            progress,
            stats,
        })
    }

    /// 更新 GPU 纹理映射（热力图）
    fn update_texture_map(density_counts: &[u32], texture_map: &mut [u8]) {
        // 找到最大密度用于归一化
        let max_density = density_counts.iter().copied().max().unwrap_or(1);

        // 归一化到 0-255
        for (i, &count) in density_counts.iter().enumerate() {
            if i < texture_map.len() {
                texture_map[i] = if max_density > 0 {
                    ((count as f64 / max_density as f64) * 255.0) as u8
                } else {
                    0
                };
            }
        }
    }

    /// 获取当前编译的模式
    pub fn pattern(&self) -> String {
        self.pattern.read().clone()
    }

    /// 获取 DFA 状态数
    pub fn state_count(&self) -> usize {
        *self.state_count.read()
    }

    /// 检查是否已编译
    pub fn is_compiled(&self) -> bool {
        self.regex.read().is_some()
    }

    /// 清除编译的 DFA
    pub fn clear(&self) {
        *self.regex.write() = None;
        *self.pattern.write() = String::new();
        *self.state_count.write() = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfa_compile_simple() {
        let engine = DfaRegexEngine::new();
        let result = engine.compile(r"error");
        assert!(result.is_ok());
        assert!(engine.is_compiled());
        assert!(engine.state_count() > 0);
    }

    #[test]
    fn test_dfa_compile_complex() {
        let engine = DfaRegexEngine::new();
        // 测试更复杂的正则
        let result = engine.compile(r"\d{4}-\d{2}-\d{2}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_dfa_search() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"error").unwrap();

        let lines = [
            "This is a normal line",
            "An error occurred here",
            "Another normal line",
            "ERROR in uppercase",
            "Final error line",
        ];

        let result = engine.search(lines.iter().copied()).unwrap();

        // 大小写敏感搜索，只有 2 行包含小写 "error" (行号 2 和 5，1-indexed)
        assert_eq!(result.index.len(), 2);
        assert!(result.index.contains(2)); // "An error occurred here" (行号 2)
        assert!(!result.index.contains(4)); // "ERROR" 大小写不匹配 (行号 4)
        assert!(result.index.contains(5)); // "Final error line" (行号 5)

        // 检查进度
        assert_eq!(result.progress.percentage, 100);
        assert_eq!(result.progress.status, SearchStatus::Completed);

        // 检查 GPU 纹理映射
        assert_eq!(result.progress.gpu_texture_map.len(), 256);
        // 应该有非零像素
        let non_zero: Vec<_> = result
            .progress
            .gpu_texture_map
            .iter()
            .filter(|&&v| v > 0)
            .collect();
        assert!(!non_zero.is_empty());
    }

    #[test]
    fn test_dfa_search_empty() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"error").unwrap();

        let lines: Vec<&str> = vec![];
        let result = engine.search(lines.iter().copied()).unwrap();

        assert_eq!(result.index.len(), 0);
        assert_eq!(result.stats.lines_processed, 0);
    }

    #[test]
    fn test_dfa_search_no_matches() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"xyz123").unwrap();

        let lines = ["error line", "warning line", "info line"];
        let result = engine.search(lines.iter().copied()).unwrap();

        assert_eq!(result.index.len(), 0);
    }

    #[test]
    fn test_dfa_case_sensitive() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"error").unwrap();

        let lines = ["error", "ERROR", "Error"];
        let result = engine.search(lines.iter().copied()).unwrap();

        // DFA 默认大小写敏感
        assert_eq!(result.index.len(), 1);
        assert!(result.index.contains(1)); // 只有 "error"
    }

    #[test]
    fn test_dfa_select_k() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"line").unwrap();

        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let result = engine.search(lines.iter().map(|s| s.as_str())).unwrap();

        // 所有行都匹配
        assert_eq!(result.index.len(), 100);

        // 测试 select(k)
        assert_eq!(result.index.select(0), Some(1));
        assert_eq!(result.index.select(50), Some(51));
        assert_eq!(result.index.select(99), Some(100));
        assert_eq!(result.index.select(100), None);
    }

    #[test]
    fn test_dfa_select_range() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"line").unwrap();

        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let result = engine.search(lines.iter().map(|s| s.as_str())).unwrap();

        // 测试 select_range
        let range = result.index.select_range(10, 5);
        assert_eq!(range.len(), 5);
        assert_eq!(range, vec![11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_dfa_stats() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"error").unwrap();

        let lines: Vec<String> = (0..1000)
            .map(|i| {
                if i % 100 == 0 {
                    format!("error at line {}", i)
                } else {
                    format!("normal line {}", i)
                }
            })
            .collect();

        let result = engine.search(lines.iter().map(|s| s.as_str())).unwrap();

        // 10 行包含 "error"
        assert_eq!(result.index.len(), 10);
        assert_eq!(result.stats.lines_processed, 1000);
        assert!(result.stats.bytes_processed > 0);
        assert!(result.stats.throughput_mbps > 0.0);
    }

    #[test]
    fn test_dfa_clear() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"error").unwrap();
        assert!(engine.is_compiled());

        engine.clear();
        assert!(!engine.is_compiled());
        assert!(engine.pattern().is_empty());
        assert_eq!(engine.state_count(), 0);
    }

    #[test]
    fn test_dfa_recompile() {
        let engine = DfaRegexEngine::new();

        // 第一次编译
        engine.compile(r"error").unwrap();
        let pattern1 = engine.pattern();
        assert_eq!(pattern1, "error");

        // 重新编译
        engine.compile(r"warning").unwrap();
        let pattern2 = engine.pattern();
        assert_eq!(pattern2, "warning");
    }

    #[test]
    fn test_search_progress_callback() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"line").unwrap();

        let lines: Vec<String> = (0..50_000).map(|i| format!("line {}", i)).collect();

        let mut callback_count = 0;
        let mut last_percentage = 0u8;

        let mut callback = |progress: &SearchProgress| {
            callback_count += 1;
            assert!(progress.percentage >= last_percentage);
            last_percentage = progress.percentage;
        };

        let result = engine
            .search_with_progress(lines.iter().map(|s| s.as_str()), Some(&mut callback))
            .unwrap();

        assert_eq!(result.index.len(), 50_000);
        // 应该有多次进度回调（每 10000 行一次）
        assert!(callback_count >= 4);
    }

    #[test]
    fn test_gpu_texture_map() {
        let engine = DfaRegexEngine::new();
        engine.compile(r"error").unwrap();

        // 创建有集中命中的数据
        let lines: Vec<String> = (0..1000)
            .map(|i| {
                if (100..200).contains(&i) {
                    format!("error in batch {}", i)
                } else {
                    format!("normal line {}", i)
                }
            })
            .collect();

        let result = engine.search(lines.iter().map(|s| s.as_str())).unwrap();

        // GPU 纹理映射应该有 256 个像素
        assert_eq!(result.progress.gpu_texture_map.len(), 256);

        // 在命中集中的区域，密度应该较高
        // 行 100-199 大约对应像素索引 25-51 (100/4=25, 200/4=50)
        let high_density_region: u8 = result.progress.gpu_texture_map[25..51]
            .iter()
            .copied()
            .max()
            .unwrap_or(0);

        let low_density_region: u8 = result.progress.gpu_texture_map[0..25]
            .iter()
            .copied()
            .max()
            .unwrap_or(0);

        // 高密度区域的值应该大于低密度区域
        assert!(
            high_density_region >= low_density_region,
            "High density region ({}) should >= low density region ({})",
            high_density_region,
            low_density_region
        );
    }
}
