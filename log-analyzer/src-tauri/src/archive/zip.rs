//! ZIP 压缩文件处理器
//!
//! 支持 Windows 编码优化（GBK/GB2312 文件名）
//! 使用workspace_id作为解压目录名,解决长路径问题
//! 集成路径安全验证和详细错误追踪

use crate::archive::path_tracker::PathTracker;
use crate::archive::processor::process_path_recursive;
use crate::archive::progress_reporter::ProgressReporter;
use crate::models::extraction_error::{
    ErrorCollector, ErrorTypeSummary, ExtractionError, ExtractionErrorType, ExtractionMetadata,
    ExtractionSummary,
};
use crate::utils::encoding::decode_filename;
use crate::utils::path::{remove_readonly, safe_path_join};
use crate::utils::path_security::{
    check_path_depth, validate_and_sanitize_path, PathValidationResult, SecurityConfig,
};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use tauri::AppHandle;
use uuid::Uuid;

/// 处理 ZIP 归档文件（Windows 编码优化）
///
/// # Arguments
///
/// * `path` - ZIP 文件路径
/// * `file_name` - 文件名
/// * `virtual_path` - 虚拟路径
/// * `target_root` - 解压目标根目录
/// * `workspace_id` - 工作区ID(用作解压目录名)
/// * `map` - 路径映射表
/// * `app` - Tauri 应用句柄
/// * `task_id` - 任务 ID
///
/// # Returns
///
/// - `Ok(ExtractionSummary)`: 解压完成摘要(包括成功/失败统计)
/// - `Err(String)`: 致命错误(无法继续)
///
/// # 特性
///
/// - **短路径命名**：使用workspace_id作为解压目录名,彻底解决长路径问题
/// - **路径安全验证**：防止路径穿越、Windows保留字符、控制字符等
/// - **详细错误追踪**：收集所有错误详情,提供完整报告
/// - **多编码支持**：自动检测并解码 UTF-8/GBK/GB2312 文件名
/// - **错误容忍**：单个条目失败不中断整体流程
#[allow(clippy::too_many_arguments)]
pub fn process_zip_archive(
    path: &Path,
    _file_name: &str,
    virtual_path: &str,
    target_root: &Path,
    workspace_id: &str,
    map: &mut HashMap<String, String>,
    app: &AppHandle,
    task_id: &str,
) -> Result<ExtractionSummary, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid zip archive: {}", e))?;

    // 使用workspace_id作为解压目录名(短路径方案)
    // 如果workspace_id为空,使用UUID生成一个
    let actual_workspace_id = if workspace_id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        workspace_id.to_string()
    };
    let extract_path = target_root.join("extracted").join(&actual_workspace_id);
    fs::create_dir_all(&extract_path)
        .map_err(|e| format!("Failed to create extract dir: {}", e))?;

    // 初始化错误收集器
    let mut error_collector = ErrorCollector::new();

    // 路径安全配置
    let security_config = SecurityConfig::default();

    // 初始化性能优化模块
    let mut path_tracker = PathTracker::new();
    let mut progress_reporter = ProgressReporter::new(archive.len(), 5);

    let mut success_count = 0;
    let mut skipped_count = 0;
    let total_entries = archive.len();

    for i in 0..total_entries {
        let entry_result = (|| -> Result<(), String> {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;
            let name_raw = file.name_raw().to_vec();
            let name = decode_filename(&name_raw);

            // Windows 兼容：路径分隔符规范化
            let normalized_name = name.replace('\\', "/");

            // 路径安全验证
            // 1. 检查路径深度
            let temp_path = std::path::PathBuf::from(&normalized_name);
            if let Err(reason) = check_path_depth(&temp_path, security_config.max_path_depth) {
                error_collector.add_unsafe_path_error(normalized_name.clone(), reason);
                return Err(format!("路径深度超限: {}", normalized_name));
            }

            // 2. 验证每个路径组件
            let mut sanitized_components = Vec::new();
            for component in temp_path.components() {
                if let Some(comp_str) = component.as_os_str().to_str() {
                    match validate_and_sanitize_path(comp_str, &security_config) {
                        PathValidationResult::Valid(clean) => {
                            sanitized_components.push(clean);
                        }
                        PathValidationResult::RequiresSanitization(_, clean) => {
                            sanitized_components.push(clean);
                        }
                        PathValidationResult::Unsafe(reason) => {
                            error_collector
                                .add_unsafe_path_error(normalized_name.clone(), reason.clone());
                            return Err(format!("不安全路径: {} - {}", normalized_name, reason));
                        }
                    }
                } else {
                    error_collector.add_error(ExtractionError::new(
                        normalized_name.clone(),
                        ExtractionErrorType::EncodingError,
                        "路径编码错误".to_string(),
                    ));
                    return Err(format!("路径编码错误: {}", normalized_name));
                }
            }

            let safe_name = sanitized_components.join("/");
            let out_path = safe_path_join(&extract_path, &safe_name);

            // 使用PathTracker检查路径冲突(优化性能)
            if path_tracker.exists(&out_path) {
                eprintln!("[WARNING] Path already exists in tracker: {:?}", out_path);
                // 继续处理,可能是压缩包内重复条目
            }

            if file.is_dir() {
                fs::create_dir_all(&out_path)
                    .map_err(|e| format!("Failed to create directory {}: {}", safe_name, e))?;
                path_tracker.add(out_path.clone()); // 记录目录
                skipped_count += 1; // 目录计入跳过数
            } else {
                if let Some(p) = out_path.parent() {
                    fs::create_dir_all(p).map_err(|e| {
                        format!("Failed to create parent dir for {}: {}", safe_name, e)
                    })?;
                }

                // Windows 兼容：在解压前移除只读属性
                if out_path.exists() {
                    remove_readonly(&out_path).map_err(|e| {
                        format!("Failed to remove readonly for {}: {}", safe_name, e)
                    })?;
                }

                let mut outfile = File::create(&out_path)
                    .map_err(|e| format!("Failed to create file {}: {}", safe_name, e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract {}: {}", safe_name, e))?;

                let new_virtual = format!("{}/{}", virtual_path, safe_name);
                // 递归处理
                process_path_recursive(
                    &out_path,
                    &new_virtual,
                    target_root,
                    map,
                    app,
                    task_id,
                    workspace_id,
                );
                path_tracker.add(out_path.clone()); // 记录文件
                success_count += 1;
            }

            Ok(())
        })();

        match entry_result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("[WARNING] ZIP entry extraction failed: {}", e);
                // 错误已经在entry_result中添加到error_collector
                // 继续处理其他文件
            }
        }

        // 智能进度报告
        progress_reporter.inc();
        if progress_reporter.should_report() {
            let progress_msg = progress_reporter.get_progress_message();
            eprintln!("[INFO] {}", progress_msg);
            // TODO: 发送task-update事件到前端
            // app.emit_all("task-update", TaskUpdatePayload { task_id, message: progress_msg })?;
            progress_reporter.mark_reported();
        }
    }

    // 构建解压摘要
    let summary = error_collector.into_summary(
        actual_workspace_id,
        total_entries,
        success_count,
        skipped_count,
        "ZIP".to_string(),
    );

    eprintln!(
        "[INFO] ZIP extraction complete: {} total, {} succeeded, {} skipped, {} failed",
        summary.total_entries, summary.success_count, summary.skipped_count, summary.failed_count
    );

    // 写入解压元数据
    if let Err(e) = write_extraction_metadata(&extract_path, path, &summary) {
        eprintln!("[WARNING] Failed to write extraction metadata: {}", e);
        // 元数据写入失败不影响解压结果
    }

    // 即使有错误,只要有部分文件成功就返回Ok(带着摘要)
    Ok(summary)
}

/// 写入解压元数据到.extraction_metadata.json
fn write_extraction_metadata(
    extract_path: &Path,
    source_path: &Path,
    summary: &ExtractionSummary,
) -> Result<(), String> {
    // 统计错误类型
    let mut error_type_counts: HashMap<String, usize> = HashMap::new();
    for error in &summary.errors {
        let type_desc = error.error_type.description();
        *error_type_counts.entry(type_desc).or_insert(0) += 1;
    }

    let error_summary: Vec<ErrorTypeSummary> = error_type_counts
        .into_iter()
        .map(|(error_type, count)| ErrorTypeSummary { error_type, count })
        .collect();

    let metadata = ExtractionMetadata {
        workspace_id: summary.workspace_id.clone(),
        source_archive: source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        source_path: source_path.to_string_lossy().to_string(),
        extraction_time: chrono::Utc::now().to_rfc3339(),
        extraction_duration_ms: summary.duration_ms,
        total_entries: summary.total_entries,
        successful_files: summary.success_count,
        failed_files: summary.failed_count,
        total_size_bytes: 0, // TODO: 计算总大小
        extractor_version: env!("CARGO_PKG_VERSION").to_string(),
        error_summary,
    };

    let metadata_path = extract_path.join(".extraction_metadata.json");
    let json_content = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    fs::write(&metadata_path, json_content)
        .map_err(|e| format!("Failed to write metadata file: {}", e))?;

    eprintln!(
        "[INFO] Extraction metadata written to: {}",
        metadata_path.display()
    );

    Ok(())
}
