//! 结构化查询命令实现

use tauri::command;

use crate::models::SearchQuery;
use crate::services::QueryExecutor;

#[command]
pub fn execute_structured_query(
    query: SearchQuery,
    logs: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut executor = QueryExecutor::new(1000);
    let plan = executor.execute(&query).map_err(|e| e.to_string())?;

    let filtered: Vec<String> = logs
        .iter()
        .filter(|line| executor.matches_line(&plan, line))
        .cloned()
        .collect();

    Ok(filtered)
}

#[command]
pub fn validate_query(query: SearchQuery) -> Result<bool, String> {
    let mut executor = QueryExecutor::new(1000);
    executor
        .execute(&query)
        .map(|_| true)
        .map_err(|e| e.to_string())
}
