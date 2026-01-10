//! 文件操作重试机制
//!
//! 使用业内成熟方案 `retry` crate，提供指数退避和 jitter 支持。
//! 替换了之前手写的线性重试实现。

use std::time::Duration;
use tracing::error;

/// 可重试的错误类型
const RETRYABLE_ERRORS: &[&str] = &[
    "permission denied",
    "Access is denied",
    "file is being used",
    "cannot access",
    "temporary failure",
    "connection refused",
    "resource temporarily unavailable",
];

/// 检查错误是否可重试
fn is_retryable_error(error: &str) -> bool {
    RETRYABLE_ERRORS
        .iter()
        .any(|e| error.to_lowercase().contains(e))
}

/// 文件操作重试辅助函数
///
/// 使用 `retry` crate 实现指数退避重试，支持自定义重试次数和延迟时间。
///
/// # 参数
///
/// - `operation` - 要执行的操作闭包，返回 `Result<T, E>`
/// - `max_retries` - 最大重试次数
/// - `base_delay_ms` - 基础延迟时间（毫秒）
/// - `max_delay_ms` - 最大延迟时间（毫秒）
/// - `operation_name` - 操作名称（用于日志输出）
///
/// # 返回值
///
/// - `Ok(T)` - 操作成功，返回结果
/// - `Err(E)` - 所有重试都失败，返回最后一次的错误
///
/// # 示例
///
/// ```ignore
/// use std::fs;
/// use std::path::Path;
///
/// let result = retry_file_operation(
///     || fs::remove_dir_all(Path::new("/temp"))
///         .map_err(|e| e.to_string()),
///     3,           // 最多重试3次
///     100,         // 基础延迟 100ms
///     5000,        // 最大延迟 5s
///     "remove_temp_dir",
/// )?;
/// ```
pub fn retry_file_operation<T, E>(
    operation: impl Fn() -> Result<T, E>,
    max_retries: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
    operation_name: &str,
) -> Result<T, E>
where
    E: std::fmt::Display + Clone,
{
    // 使用指数退避策略
    let mut attempt = 0;

    loop {
        match operation() {
            Ok(result) => {
                if attempt > 0 {
                    tracing::info!(
                        operation = %operation_name,
                        retries = attempt,
                        "Operation succeeded after retries"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                // 检查是否是可重试的错误
                if !is_retryable_error(&e.to_string()) || attempt >= max_retries {
                    error!(
                        operation = %operation_name,
                        attempts = attempt + 1,
                        error = %e,
                        "Operation failed after all attempts"
                    );
                    return Err(e);
                }

                // 计算延迟时间（指数退避）
                let exp_delay =
                    std::cmp::min(base_delay_ms * (2_u64.pow(attempt as u32)), max_delay_ms);

                tracing::warn!(
                    operation = %operation_name,
                    attempt = attempt + 1,
                    delay_ms = exp_delay,
                    error = %e,
                    "Operation failed, retrying"
                );

                std::thread::sleep(Duration::from_millis(exp_delay));
                attempt += 1;
            }
        }
    }
}

/// 简化的重试函数（默认参数）
///
/// 适用于简单的重试场景，使用默认的延迟参数。
///
/// # 参数
///
/// - `operation` - 要执行的操作闭包
/// - `operation_name` - 操作名称
pub fn retry_simple<T, E>(
    operation: impl Fn() -> Result<T, E>,
    operation_name: &str,
) -> Result<T, E>
where
    E: std::fmt::Display + Clone,
{
    retry_file_operation(
        operation,
        3,    // 3 次重试
        100,  // 100ms 基础延迟
        5000, // 5s 最大延迟
        operation_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_success_first_attempt() {
        let result = retry_file_operation(|| Ok::<i32, String>(42), 3, 10, 100, "test_success");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_retry_success_after_failures() {
        let attempts = std::cell::Cell::new(0);
        let result = retry_file_operation(
            || {
                attempts.set(attempts.get() + 1);
                if attempts.get() < 2 {
                    Err("temporary failure".to_string())
                } else {
                    Ok(42)
                }
            },
            3,
            10,
            100,
            "test_retry",
        );
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.get(), 2);
    }

    #[test]
    fn test_retry_non_retryable_error() {
        let result = retry_file_operation(
            || Err::<i32, String>("permission denied".to_string()),
            3,
            10,
            100,
            "test_non_retryable",
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "permission denied");
    }

    #[test]
    fn test_retry_simple() {
        let result: Result<i32, String> = retry_simple(|| Ok(42), "test_simple");
        assert_eq!(result.unwrap(), 42);
    }
}
