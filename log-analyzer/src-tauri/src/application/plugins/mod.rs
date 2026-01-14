//! 插件化架构系统
//!
//! 支持动态加载插件，实现功能扩展

use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::log_analysis::entities::LogEntry;
use crate::error::Result;

/// 插件接口定义
pub trait Plugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &'static str;
    
    /// 插件版本
    fn version(&self) -> &'static str;
    
    /// 插件描述
    fn description(&self) -> &'static str;
    
    /// 初始化插件
    fn initialize(&mut self, config: &serde_json::Value) -> Result<()>;
    
    /// 处理日志条目
    fn process_log(&self, entry: &mut LogEntry) -> Result<()>;
    
    /// 处理搜索查询
    fn process_search(&self, query: &str) -> Result<String>;
    
    /// 清理资源
    fn cleanup(&self) -> Result<()>;
}

/// 插件管理器
#[derive(Default)]
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    loaded_libraries: Arc<RwLock<Vec<Library>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 加载插件
    pub async fn load_plugin(&self, path: &Path) -> Result<()> {
        let lib = unsafe { Library::new(path) }
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to load plugin: {}", e)))?;
        
        type PluginCreate = unsafe fn() -> *mut dyn Plugin;
        
        let create_plugin: Symbol<PluginCreate> = unsafe {
            lib.get(b"create_plugin")
                .map_err(|e| crate::error::AppError::Internal(format!("Plugin missing create_plugin symbol: {}", e)))?
        };
        
        let plugin_raw = unsafe { create_plugin() };
        let plugin = unsafe { Box::from_raw(plugin_raw) };
        
        let name = plugin.name().to_string();
        
        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(name.clone(), plugin);
        }
        
        {
            let mut libs = self.loaded_libraries.write().await;
            libs.push(lib);
        }
        
        Ok(())
    }
    
    /// 卸载插件
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.remove(name) {
            plugin.cleanup()?;
        }
        Ok(())
    }
    
    /// 获取已加载的插件
    pub async fn get_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }
    
    /// 处理日志条目
    pub async fn process_log(&self, entry: &mut LogEntry) -> Result<()> {
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.process_log(entry)?;
        }
        Ok(())
    }
    
    /// 处理搜索查询
    pub async fn process_search(&self, query: &str) -> Result<String> {
        let plugins = self.plugins.read().await;
        let mut processed_query = query.to_string();
        
        for plugin in plugins.values() {
            processed_query = plugin.process_search(&processed_query)?;
        }
        
        Ok(processed_query)
    }
    
    /// 初始化所有插件
    pub async fn initialize_all(&self, config: &serde_json::Value) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        for plugin in plugins.values_mut() {
            plugin.initialize(config)?;
        }
        Ok(())
    }
}

/// 示例插件：日志增强器
pub struct LogEnhancerPlugin;

impl Plugin for LogEnhancerPlugin {
    fn name(&self) -> &'static str {
        "log_enhancer"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    
    fn description(&self) -> &'static str {
        "Enhances log entries with additional metadata"
    }
    
    fn initialize(&mut self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
    
    fn process_log(&self, entry: &mut LogEntry) -> Result<()> {
        // 添加处理时间
        entry.add_metadata("processed_at".to_string(), chrono::Utc::now().to_rfc3339());
        
        // 添加长度信息
        entry.add_metadata("message_length".to_string(), entry.message.len().to_string());
        
        Ok(())
    }
    
    fn process_search(&self, query: &str) -> Result<String> {
        // 增强搜索查询
        Ok(format!("enhanced: {}", query))
    }
    
    fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

/// 示例插件：搜索过滤器
pub struct SearchFilterPlugin;

impl Plugin for SearchFilterPlugin {
    fn name(&self) -> &'static str {
        "search_filter"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    
    fn description(&self) -> &'static str {
        "Filters search queries based on security rules"
    }
    
    fn initialize(&mut self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
    
    fn process_log(&self, _entry: &mut LogEntry) -> Result<()> {
        Ok(())
    }
    
    fn process_search(&self, query: &str) -> Result<String> {
        // 过滤敏感词
        let filtered = query.replace("password", "***");
        Ok(filtered)
    }
    
    fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

/// 插件注册宏
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn create_plugin() -> *mut dyn Plugin {
            let boxed = Box::new($plugin_type);
            Box::into_raw(boxed)
        }
    };
}