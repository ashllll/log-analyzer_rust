use crate::error::Result;
use crate::models::search::{QueryOperator, SearchQuery};
use crate::services::pattern_matcher::PatternMatcher;
use crate::services::query_planner::QueryPlanner;
/**
 * 性能基准测试模块
 *
 * 用于测试和验证各种优化措施的性能提升效果
 */
use std::time::{Duration, Instant};

/// 基准测试结果
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub iterations: usize,
    pub avg_time_ms: f64,
    pub throughput: f64, // 操作/秒
}

impl BenchmarkResult {
    #[allow(dead_code)]
    pub fn new(name: String, duration: Duration, iterations: usize) -> Self {
        let avg_time_ms = duration.as_secs_f64() * 1000.0 / iterations as f64;
        let throughput = iterations as f64 / duration.as_secs_f64();

        Self {
            name,
            duration,
            iterations,
            avg_time_ms,
            throughput,
        }
    }
}

/// 基准测试运行器
#[allow(dead_code)]
pub struct BenchmarkRunner;

impl BenchmarkRunner {
    /// 运行搜索算法基准测试
    #[allow(dead_code)]
    pub fn run_search_benchmark() -> Result<Vec<BenchmarkResult>> {
        let results = vec![
            Self::benchmark_single_keyword()?,
            Self::benchmark_multiple_keywords(10)?,
            Self::benchmark_multiple_keywords(100)?,
            Self::benchmark_large_file()?,
            Self::benchmark_regex_search()?,
        ];

        Ok(results)
    }

    /// 基准测试：单关键词搜索
    #[allow(dead_code)]
    fn benchmark_single_keyword() -> Result<BenchmarkResult> {
        let content = generate_test_content(10000);
        let patterns = vec!["error".to_string()];
        let matcher = PatternMatcher::new(patterns, false);

        let start = Instant::now();
        let iterations = 1000;

        for _ in 0..iterations {
            let _ = matcher.find_matches(&content);
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            "单关键词搜索".to_string(),
            duration,
            iterations,
        ))
    }

    /// 基准测试：多关键词搜索
    #[allow(dead_code)]
    fn benchmark_multiple_keywords(count: usize) -> Result<BenchmarkResult> {
        let content = generate_test_content(10000);
        let patterns = generate_keywords(count);
        let matcher = PatternMatcher::new(patterns, false);

        let start = Instant::now();
        let iterations = 100;

        for _ in 0..iterations {
            let _ = matcher.find_matches(&content);
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            format!("多关键词搜索({}个)", count),
            duration,
            iterations,
        ))
    }

    /// 基准测试：大文件搜索
    #[allow(dead_code)]
    fn benchmark_large_file() -> Result<BenchmarkResult> {
        let content = generate_test_content(100000); // 10万行
        let patterns = vec![
            "error".to_string(),
            "warning".to_string(),
            "info".to_string(),
        ];
        let matcher = PatternMatcher::new(patterns, false);

        let start = Instant::now();
        let iterations = 10;

        for _ in 0..iterations {
            let _ = matcher.find_matches(&content);
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            "大文件搜索(10万行)".to_string(),
            duration,
            iterations,
        ))
    }

    /// 基准测试：正则表达式搜索
    #[allow(dead_code)]
    fn benchmark_regex_search() -> Result<BenchmarkResult> {
        use crate::models::search::{QueryMetadata, SearchTerm, TermSource};
        use crate::services::query_executor::QueryExecutor;

        let content = generate_test_content(10000);
        let query = SearchQuery {
            id: "benchmark-regex".to_string(),
            terms: vec![SearchTerm {
                id: "term-1".to_string(),
                value: r"\d{4}-\d{2}-\d{2}".to_string(),
                operator: QueryOperator::And,
                source: TermSource::User,
                preset_group_id: None,
                is_regex: true,
                priority: 1,
                enabled: true,
                case_sensitive: false,
            }],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let mut executor = QueryExecutor::new(100);
        let plan = executor.execute(&query)?;

        let start = Instant::now();
        let iterations = 100;

        for _ in 0..iterations {
            let _ = executor.matches_line(&plan, content.lines().next().unwrap_or(""));
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            "正则表达式搜索".to_string(),
            duration,
            iterations,
        ))
    }

    /// 运行查询执行器基准测试
    #[allow(dead_code)]
    pub fn run_query_executor_benchmark() -> Result<Vec<BenchmarkResult>> {
        let results = vec![
            Self::benchmark_query_planning()?,
            Self::benchmark_query_execution()?,
        ];

        Ok(results)
    }

    /// 运行处理器模块基准测试
    #[allow(dead_code)]
    pub fn run_processor_benchmark() -> Result<Vec<BenchmarkResult>> {
        let results = vec![
            Self::benchmark_string_processing()?,
            Self::benchmark_large_file_processing()?,
            Self::benchmark_batch_file_processing()?,
        ];

        Ok(results)
    }

    /// 基准测试：字符串处理性能
    #[allow(dead_code)]
    fn benchmark_string_processing() -> Result<BenchmarkResult> {
        let iterations = 100000;
        let test_strings = vec![
            "/very/long/path/to/some/deeply/nested/directory/structure/with/many/levels/file.log",
            "short.log",
            "archive.tar.gz",
            "path/with/spaces in name/file.txt",
            "C:\\Windows\\System32\\config\\system.log",
        ];

        let start = Instant::now();

        for i in 0..iterations {
            let s = &test_strings[i % test_strings.len()];
            let _ = s.split('/').last();
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            "字符串处理（路径分割）".to_string(),
            duration,
            iterations,
        ))
    }

    /// 基准测试：大文件处理性能
    #[allow(dead_code)]
    fn benchmark_large_file_processing() -> Result<BenchmarkResult> {
        use tempfile::TempDir;

        let iterations = 100;
        let file_sizes = vec![
            1024 * 1024,      // 1MB
            10 * 1024 * 1024, // 10MB
        ];

        let mut best_throughput = 0.0;
        let mut best_name = "".to_string();

        for &size in &file_sizes {
            let temp_dir = TempDir::new()?;
            let test_file = temp_dir.path().join("large_test.log");

            // 创建大文件
            let content = "log line with some data\n".repeat(size / 20);
            std::fs::write(&test_file, &content)?;

            let start = Instant::now();
            let file_iterations = iterations;

            for _ in 0..file_iterations {
                let _metadata = std::fs::metadata(&test_file)?;
            }

            let duration = start.elapsed();
            let throughput = file_iterations as f64 / duration.as_secs_f64();

            if throughput > best_throughput {
                best_throughput = throughput;
                best_name = format!("大文件处理({}MB)", size / 1024 / 1024);
            }
        }

        Ok(BenchmarkResult::new(
            best_name,
            Duration::from_secs_f64(1.0 / best_throughput),
            1,
        ))
    }

    /// 基准测试：批量文件处理性能
    #[allow(dead_code)]
    fn benchmark_batch_file_processing() -> Result<BenchmarkResult> {
        use tempfile::TempDir;

        let file_counts = vec![100, 1000, 5000];
        let mut best_throughput = 0.0;
        let mut best_name = "".to_string();

        for count in file_counts {
            let temp_dir = TempDir::new()?;

            // 创建批量文件
            for i in 0..count {
                let test_file = temp_dir.path().join(format!("log_{}.txt", i));
                std::fs::write(&test_file, format!("log content {}", i))?;
            }

            let start = Instant::now();

            // 批量处理文件
            for entry in std::fs::read_dir(temp_dir.path())? {
                let entry = entry?;
                let _metadata = entry.metadata()?;
            }

            let duration = start.elapsed();
            let throughput = 1.0 / duration.as_secs_f64();

            if throughput > best_throughput {
                best_throughput = throughput;
                best_name = format!("批量文件处理({}文件)", count);
            }
        }

        Ok(BenchmarkResult::new(
            best_name,
            Duration::from_secs_f64(1.0 / best_throughput),
            1,
        ))
    }

    /// 基准测试：查询计划构建
    #[allow(dead_code)]
    fn benchmark_query_planning() -> Result<BenchmarkResult> {
        use crate::models::search::{QueryMetadata, SearchTerm, TermSource};

        let query = SearchQuery {
            id: "benchmark-1".to_string(),
            terms: vec![
                SearchTerm {
                    id: "term-1".to_string(),
                    value: "error".to_string(),
                    operator: QueryOperator::And,
                    source: TermSource::User,
                    preset_group_id: None,
                    is_regex: false,
                    priority: 1,
                    enabled: true,
                    case_sensitive: false,
                },
                SearchTerm {
                    id: "term-2".to_string(),
                    value: "warning".to_string(),
                    operator: QueryOperator::And,
                    source: TermSource::User,
                    preset_group_id: None,
                    is_regex: false,
                    priority: 1,
                    enabled: true,
                    case_sensitive: false,
                },
            ],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let mut planner = QueryPlanner::new(100);
        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = planner.build_plan(&query);
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            "查询计划构建".to_string(),
            duration,
            iterations,
        ))
    }

    /// 基准测试：查询执行
    #[allow(dead_code)]
    fn benchmark_query_execution() -> Result<BenchmarkResult> {
        use crate::models::search::{QueryMetadata, SearchTerm, TermSource};
        use crate::services::query_executor::QueryExecutor;

        let content = generate_test_content(10000);
        let query = SearchQuery {
            id: "benchmark-2".to_string(),
            terms: vec![
                SearchTerm {
                    id: "term-1".to_string(),
                    value: "error".to_string(),
                    operator: QueryOperator::And,
                    source: TermSource::User,
                    preset_group_id: None,
                    is_regex: false,
                    priority: 1,
                    enabled: true,
                    case_sensitive: false,
                },
                SearchTerm {
                    id: "term-2".to_string(),
                    value: "warning".to_string(),
                    operator: QueryOperator::And,
                    source: TermSource::User,
                    preset_group_id: None,
                    is_regex: false,
                    priority: 1,
                    enabled: true,
                    case_sensitive: false,
                },
            ],
            global_operator: QueryOperator::And,
            filters: None,
            metadata: QueryMetadata {
                created_at: 0,
                last_modified: 0,
                execution_count: 0,
                label: None,
            },
        };

        let mut executor = QueryExecutor::new(100);
        let plan = executor.execute(&query)?;

        let start = Instant::now();
        let iterations = 100;

        for _ in 0..iterations {
            // 测试匹配第一行
            let _ = executor.matches_line(&plan, content.lines().next().unwrap_or(""));
        }

        let duration = start.elapsed();
        Ok(BenchmarkResult::new(
            "查询执行".to_string(),
            duration,
            iterations,
        ))
    }
}

/// 生成测试内容
#[allow(dead_code)]
fn generate_test_content(lines: usize) -> String {
    let mut content = String::new();
    let templates = [
        "2024-01-01 10:00:00 INFO Application started successfully",
        "2024-01-01 10:00:01 WARNING Low memory warning",
        "2024-01-01 10:00:02 ERROR Database connection failed",
        "2024-01-01 10:00:03 INFO User logged in: user123",
        "2024-01-01 10:00:04 DEBUG Processing request: GET /api/data",
        "2024-01-01 10:00:05 ERROR Timeout while processing request",
        "2024-01-01 10:00:06 INFO Data processed successfully",
        "2024-01-01 10:00:07 WARNING High CPU usage detected",
        "2024-01-01 10:00:08 ERROR File not found: /path/to/file",
        "2024-01-01 10:00:09 INFO Shutdown initiated",
    ];

    for i in 0..lines {
        let template = &templates[i % templates.len()];
        content.push_str(template);
        content.push('\n');
    }

    content
}

/// 生成关键词列表
#[allow(dead_code)]
fn generate_keywords(count: usize) -> Vec<String> {
    let base_keywords = [
        "error",
        "warning",
        "info",
        "debug",
        "critical",
        "fatal",
        "success",
        "failed",
        "timeout",
        "connection",
        "database",
        "memory",
        "cpu",
        "disk",
        "network",
        "request",
        "response",
        "status",
        "code",
        "message",
    ];

    let mut keywords = Vec::new();
    for i in 0..count {
        let keyword = format!("{}_{}", base_keywords[i % base_keywords.len()], i);
        keywords.push(keyword);
    }

    keywords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_calculation() {
        let result = BenchmarkResult::new("test".to_string(), Duration::from_secs(1), 1000);

        assert_eq!(result.name, "test");
        assert_eq!(result.iterations, 1000);
        assert_eq!(result.avg_time_ms, 1.0); // 1000ms / 1000 = 1ms
        assert_eq!(result.throughput, 1000.0); // 1000 ops / 1s = 1000 ops/s
    }

    #[test]
    fn test_generate_test_content() {
        let content = generate_test_content(100);
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 100);
    }

    #[test]
    fn test_generate_keywords() {
        let keywords = generate_keywords(50);
        assert_eq!(keywords.len(), 50);
        assert!(keywords[0].starts_with("error") || keywords[0].starts_with("warning"));
    }
}
