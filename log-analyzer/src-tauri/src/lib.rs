use tauri::{command, AppHandle}; // 移除了未使用的 Emitter
use serde::Serialize;
use rand::Rng; // 引入 Rng trait

#[derive(Serialize, Clone, Debug)]
struct LogEntry {
    id: usize,
    timestamp: String,
    level: String,
    file: String,
    line: usize,
    content: String,
    tags: Vec<Tag>,
}

#[derive(Serialize, Clone, Debug)]
struct Tag {
    text: String,
    tooltip: String,
    color: String,
}

// 注意: pattern 参数用于模拟，_app 添加下划线忽略警告
#[command]
async fn search_logs(pattern: String, _app: AppHandle) -> Result<Vec<LogEntry>, String> {
    if pattern.is_empty() { return Ok(vec![]); }
    println!("Searching for: {}", pattern);

    let mut results = Vec::new();
    let mut rng = rand::rng(); // rand 0.9 写法
    let files = vec!["auth_service.log", "db_shard_01.log", "nginx_access.log", "payment_gateway.log"];
    
    for i in 0..5000 {
        let file = files[rng.random_range(0..files.len())].to_string(); // 使用 random_range
        // 简单模拟概率
        let roll = rng.random_range(0..100); 
        let level = if roll < 10 { "ERROR" } else if roll < 30 { "WARN" } else { "INFO" };
        
        let content = format!("Session ID: {} connection state changed. Reason: {} occurred in transaction loop.", 
            rng.random_range(10000..99999), 
            pattern
        );

        let mut tags = Vec::new();
        if pattern.contains("timeout") || pattern.contains("refused") {
            tags.push(Tag { text: "Network".to_string(), tooltip: "Check Firewall/VPN".to_string(), color: "#F14C4C".to_string() });
        }
        if file.contains("db") {
            tags.push(Tag { text: "SQL Slow".to_string(), tooltip: "Query time > 2s".to_string(), color: "#007ACC".to_string() });
        }

        results.push(LogEntry {
            id: i,
            // 模拟时间生成
            timestamp: format!("2023-11-25 10:{:02}:{:02}.{}", 
                rng.random_range(0..59), 
                rng.random_range(0..59), 
                rng.random_range(100..999)
            ),
            level: level.to_string(),
            file,
            line: rng.random_range(1000..90000),
            content,
            tags,
        });
    }
    
    // 模拟极短耗时
    std::thread::sleep(std::time::Duration::from_millis(100));
    Ok(results)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![search_logs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}