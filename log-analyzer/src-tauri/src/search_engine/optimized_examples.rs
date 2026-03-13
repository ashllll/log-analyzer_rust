//! Optimized Search Engine Usage Examples
//!
//! 本文件提供 OptimizedSearchEngineManager 的使用示例
//! 展示如何在实际项目中应用各项优化

use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::models::LogEntry;
use crate::search_engine::{
    OptimizedSearchConfig, OptimizedSearchEngineManager, SearchResults,
};

/// 示例 1: 基础配置创建
/// 
/// 展示如何根据不同的部署环境选择合适的配置
pub fn create_config_for_environment(env: &str) -> OptimizedSearchConfig {
    match env {
        "development" => OptimizedSearchConfig {
            default_timeout: Duration::from_millis(500),
            max_results: 10_000,
            index_path: PathBuf::from("./tmp/search_index"),
            writer_heap_size: 25_000_000, // 25MB
            memory_budget_mb: 128,
            enable_query_cache: true,
            enable_parallel_highlight: false, // 开发环境不需要
        },
        
        "production" => OptimizedSearchConfig {
            default_timeout: Duration::from_millis(200),
            max_results: 50_000,
            index_path: PathBuf::from("/var/lib/log-analyzer/search_index"),
            writer_heap_size: 200_000_000, // 200MB
            memory_budget_mb: 512,
            enable_query_cache: true,
            enable_parallel_highlight: true,
        },
        
        "high_performance" => OptimizedSearchConfig {
            default_timeout: Duration::from_millis(100),
            max_results: 100_000,
            index_path: PathBuf::from("/opt/log-analyzer/search_index"),
            writer_heap_size: 1_000_000_000, // 1GB
            memory_budget_mb: 2048, // 2GB
            enable_query_cache: true,
            enable_parallel_highlight: true,
        },
        
        _ => OptimizedSearchConfig::default(),
    }
}

/// 示例 2: 带取消令牌的搜索
/// 
/// 展示如何在用户取消操作时立即停止搜索
pub async fn search_with_cancellation(
    manager: &OptimizedSearchEngineManager,
    query: &str,
) -> anyhow::Result<SearchResults> {
    let token = CancellationToken::new();
    let token_clone = token.clone();
    
    // 模拟用户在 500ms 后取消
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        token_clone.cancel();
        println!("Search cancelled by user");
    });
    
    let results = manager
        .search_with_budget(query, Some(1000), None, Some(token), None)
        .await?;
    
    Ok(results)
}

/// 示例 3: 批量索引文档
/// 
/// 展示如何使用 Writer Pool 高效批量索引
pub async fn batch_index_documents(
    manager: &OptimizedSearchEngineManager,
    entries: Vec<LogEntry>,
) -> anyhow::Result<u64> {
    const BATCH_SIZE: usize = 1000;
    const COMMIT_INTERVAL: usize = 5000;
    
    let total = entries.len();
    let mut committed = 0;
    
    for (i, entry) in entries.iter().enumerate() {
        // 添加文档到 Writer Pool
        manager.add_document(entry).await?;
        
        // 每 COMMIT_INTERVAL 个文档提交一次
        if (i + 1) % COMMIT_INTERVAL == 0 {
            let opstamp = manager.commit().await?;
            committed = i + 1;
            println!("Committed {} / {} documents (opstamp: {})", 
                committed, total, opstamp);
        }
    }
    
    // 提交剩余文档
    if committed < total {
        let opstamp = manager.commit().await?;
        println!("Final commit: {} documents (opstamp: {})", total, opstamp);
    }
    
    Ok(total as u64)
}

/// 示例 4: 搜索并分页
/// 
/// 展示如何使用内存预算实现安全的分页搜索
pub async fn paginated_search(
    manager: &OptimizedSearchEngineManager,
    query: &str,
    page_size: usize,
    page: usize,
) -> anyhow::Result<(Vec<LogEntry>, usize)> {
    // 计算需要的 limit（获取到当前页的所有结果）
    let limit = page_size * (page + 1);
    
    // 使用内存预算搜索
    let results = manager
        .search_with_budget(
            query,
            Some(limit),
            Some(Duration::from_secs(2)),
            None,
            Some(256), // 256MB 内存预算
        )
        .await?;
    
    // 计算当前页的结果
    let start = page * page_size;
    let end = (start + page_size).min(results.entries.len());
    
    if start >= results.entries.len() {
        return Ok((vec![], results.total_count));
    }
    
    let page_entries = results.entries[start..end].to_vec();
    
    println!(
        "Page {}/{}, showing {} results (total: {}, memory: {} bytes)",
        page + 1,
        (results.total_count + page_size - 1) / page_size,
        page_entries.len(),
        results.total_count,
        results.memory_used_bytes
    );
    
    Ok((page_entries, results.total_count))
}

/// 示例 5: 高亮搜索并流式返回
/// 
/// 展示如何使用并行高亮提高性能
pub async fn highlight_search_streaming(
    manager: &OptimizedSearchEngineManager,
    query: &str,
    max_results: usize,
) -> anyhow::Result<Vec<LogEntry>> {
    let start = std::time::Instant::now();
    
    let results = manager
        .search_with_parallel_highlighting(
            query,
            Some(max_results),
            Some(Duration::from_secs(3)),
            None,
        )
        .await?;
    
    let total_time = start.elapsed();
    
    println!(
        "Found {} results in {}ms total (query: {}ms, highlight: {}ms)",
        results.entries.len(),
        total_time.as_millis(),
        results.query_time_ms,
        results.highlight_time_ms
    );
    
    // 计算加速比
    if results.highlight_time_ms > 0 {
        let speedup = (results.query_time_ms + results.highlight_time_ms) as f64 
            / results.query_time_ms.max(1) as f64;
        println!("Parallel highlighting speedup: {:.2}x", speedup);
    }
    
    Ok(results.entries)
}

/// 示例 6: 文件删除和索引清理
/// 
/// 展示如何删除文件并监控 Writer Pool 状态
pub async fn delete_file_and_monitor(
    manager: &OptimizedSearchEngineManager,
    file_path: &str,
) -> anyhow::Result<usize> {
    let pending_before = manager.get_writer_pending_count();
    println!("Pending operations before delete: {}", pending_before);
    
    let deleted = manager.delete_file_documents(file_path).await?;
    
    let pending_after = manager.get_writer_pending_count();
    println!("Deleted {} documents, pending operations: {}", 
        deleted, pending_after);
    
    Ok(deleted)
}

/// 示例 7: 性能监控和优化建议
/// 
/// 展示如何获取性能统计和优化建议
pub fn analyze_performance(manager: &OptimizedSearchEngineManager) {
    let stats = manager.get_stats();
    
    println!("\n=== Performance Statistics ===");
    println!("Total searches: {}", stats.total_searches);
    println!("Average query time: {}ms", 
        stats.total_query_time_ms / stats.total_searches.max(1));
    println!("Timeout count: {}", stats.timeout_count);
    println!("Cache hit rate: {:.1}%", 
        if stats.cache_hits + stats.cache_misses > 0 {
            100.0 * stats.cache_hits as f64 / 
                (stats.cache_hits + stats.cache_misses) as f64
        } else {
            0.0
        }
    );
    println!("Searcher cache hits: {}", stats.searcher_cache_hits);
    println!("Memory limited searches: {}", stats.memory_limited_count);
    println!("Total memory used: {} MB", 
        stats.total_memory_used_bytes / 1024 / 1024);
    
    // 获取优化分析
    if let Some(analysis) = manager.get_optimization_analysis() {
        println!("\n=== Optimization Analysis ===");
        println!("Hot queries: {:?}", analysis.hot_queries);
        println!("Slow queries: {:?}", analysis.slow_queries);
        println!("Recommendations: {:?}", analysis.recommendations);
    }
    
    // 获取热点查询
    let hot_queries = manager.get_hot_queries();
    if !hot_queries.is_empty() {
        println!("\n=== Top 10 Hot Queries ===");
        for (i, (query, stats)) in hot_queries.iter().take(10).enumerate() {
            println!(
                "{}. '{}' - {} times, avg {}ms",
                i + 1,
                query,
                stats.frequency,
                stats.average_time_ms
            );
        }
    }
}

/// 示例 8: 内存受限的批量搜索
/// 
/// 展示如何在内存受限环境下执行大量搜索
pub async fn memory_constrained_batch_search(
    manager: &OptimizedSearchEngineManager,
    queries: Vec<String>,
) -> Vec<anyhow::Result<SearchResults>> {
    let mut results = Vec::with_capacity(queries.len());
    
    for query in queries {
        // 每个查询使用 64MB 内存预算
        let result = manager
            .search_with_budget(
                &query,
                Some(100), // 小结果集
                Some(Duration::from_millis(500)),
                None,
                Some(64), // 64MB 内存预算
            )
            .await
            .map_err(|e| anyhow::anyhow!(e));
        
        results.push(result);
    }
    
    results
}

/// 示例 9: 搜索策略选择
/// 
/// 展示如何根据查询类型选择最佳搜索策略
pub async fn smart_search(
    manager: &OptimizedSearchEngineManager,
    query: &str,
) -> anyhow::Result<SearchResults> {
    // 分析查询类型
    let keywords: Vec<&str> = query.split_whitespace().collect();
    
    let results = if keywords.len() > 1 {
        // 多关键词查询 - 使用布尔查询处理器
        manager
            .search_multi_keyword(
                &keywords.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                true, // require_all
                Some(1000),
                Some(Duration::from_secs(1)),
                None,
            )
            .await?
    } else {
        // 单关键词查询 - 使用标准搜索
        manager
            .search_with_budget(
                query,
                Some(1000),
                Some(Duration::from_secs(1)),
                None,
                None,
            )
            .await?
    };
    
    Ok(results)
}

#[cfg(test)]
mod examples {
    use super::*;
    
    /// 展示如何配置不同环境的示例
    #[test]
    fn example_environment_configs() {
        let dev_config = create_config_for_environment("development");
        assert_eq!(dev_config.memory_budget_mb, 128);
        
        let prod_config = create_config_for_environment("production");
        assert_eq!(prod_config.memory_budget_mb, 512);
        
        let high_perf_config = create_config_for_environment("high_performance");
        assert_eq!(high_perf_config.memory_budget_mb, 2048);
    }
}
