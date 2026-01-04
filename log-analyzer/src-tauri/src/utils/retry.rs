//! 文件操作重试机制
//!
//! 提供重试逻辑，用于处理可能暂时失败的文件操作（如权限问题、文件被占用等）。

use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

/// 文件操作重试辅助函数
///
/// 对可能暂时失败的文件操作进行重试，支持自定义重试次数和延迟时间。
///
/// # 参数
///
/// - `operation` - 要执行的操作闭包，返回 `Result<T, String>`
/// - `max_retries` - 最大重试次数
/// - `delays_ms` - 每次重试的延迟时间（毫秒），数组长度应至少为 max_retries
/// - `operation_name` - 操作名称（用于日志输出）
///
/// # 返回值
///
/// - `Ok(T)` - 操作成功，返回结果
/// - `Err(String)` - 所有重试都失败，返回最后一次的错误信息
///
/// # 可重试的错误类型
///
/// - 权限被拒绝（permission denied）
/// - 访问被拒绝（Access is denied）
/// - 文件正在使用（file is being used）
/// - 无法访问（cannot access）
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
///     &[100, 500, 1000], // 延迟时间
///     "remove_temp_dir",
/// )?;
/// ```
pub fn retry_file_operation<T, F>(
    mut operation: F,
    max_retries: usize,
    delays_ms: &[u64],
    operation_name: &str,
) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_error = String::new();

    for attempt in 0..=max_retries {
        match operation() {
            Ok(result) => {
                if attempt > 0 {
                    info!(
                        operation = %operation_name,
                        retries = attempt,
                        "Operation succeeded after retries"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = e.clone();

                // 检查是否是可重试的错误
                let is_retryable = e.contains("permission denied")
                    || e.contains("Access is denied")
                    || e.contains("file is being used")
                    || e.contains("cannot access");

                if !is_retryable || attempt >= max_retries {
                    error!(
                        operation = %operation_name,
                        attempts = attempt + 1,
                        error = %e,
                        "Operation failed after all attempts"
                    );
                    break;
                }

                // 等待后重试
                let delay = delays_ms.get(attempt).copied().unwrap_or(500);
                warn!(
                    operation = %operation_name,
                    attempt = attempt + 1,
                    delay_ms = delay,
                    error = %e,
                    "Operation failed, retrying"
                );
                thread::sleep(Duration::from_millis(delay));
            }
        }
    }

    Err(format!(
        "{} failed after {} attempts: {}",
        operation_name,
        max_retries + 1,
        last_error
    ))
}
