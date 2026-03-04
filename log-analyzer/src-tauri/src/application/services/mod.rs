//! 应用服务层
//!
//! 提供日志分析相关的应用服务，协调领域层和基础设施层。
//! 支持插件化架构，允许通过插件扩展日志处理和搜索功能。

use crate::application::plugins::PluginManager;
use crate::domain::log_analysis::entities::{LogEntry, LogFile};
use crate::domain::log_analysis::value_objects::LogLevel;
use crate::error::Result;
use crate::infrastructure::config::AppConfig;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 日志分析应用服务
///
/// # 职责
/// - 协调日志文件的导入和分析
/// - 执行搜索查询（支持插件预处理）
/// - 提供系统状态信息
///
/// # 插件集成
/// 服务初始化时会加载指定目录下的所有插件，插件可以：
/// - 预处理搜索查询（如过滤敏感词、增强查询语法）
/// - 增强日志条目（如添加元数据、格式化内容）
///
/// # 线程安全
/// 所有字段都使用 `Arc` 包装，确保跨线程共享安全。
pub struct LogAnalysisService {
    /// 插件管理器，用于扩展日志处理和搜索功能
    plugins: Arc<PluginManager>,
    /// 应用配置
    config: Arc<AppConfig>,
}

impl LogAnalysisService {
    /// 创建新的日志分析服务实例
    ///
    /// # 参数
    /// - `config`: 应用配置
    ///
    /// # 返回
    /// 返回初始化完成的服务实例
    ///
    /// # 插件目录
    /// 插件目录从配置的 `storage.data_dir` 路径下的 `plugins` 子目录获取。
    /// 如果目录不存在，将使用空目录（无插件加载）。
    pub fn new(config: Arc<AppConfig>) -> Self {
        // 构建插件目录路径
        let plugin_dir = PathBuf::from(&config.storage.data_dir).join("plugins");

        // 确保插件目录存在
        if !plugin_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&plugin_dir) {
                tracing::warn!("无法创建插件目录 {:?}: {}", plugin_dir, e);
            }
        }

        let plugins = Arc::new(PluginManager::new(plugin_dir));

        Self { plugins, config }
    }

    /// 使用现有插件管理器创建服务实例
    ///
    /// # 参数
    /// - `config`: 应用配置
    /// - `plugins`: 已初始化的插件管理器
    ///
    /// # 用例
    /// 适用于需要共享插件管理器的场景，如测试环境。
    pub fn with_plugins(config: Arc<AppConfig>, plugins: Arc<PluginManager>) -> Self {
        Self { plugins, config }
    }

    /// 获取插件管理器引用
    ///
    /// # 返回
    /// 返回插件管理器的 Arc 引用，可用于直接操作插件
    pub fn plugins(&self) -> Arc<PluginManager> {
        Arc::clone(&self.plugins)
    }

    /// 分析日志文件
    ///
    /// # 参数
    /// - `file_path`: 日志文件路径
    ///
    /// # 返回
    /// 返回解析后的日志条目列表
    ///
    /// # 插件处理
    /// 每个日志条目都会通过已加载的插件进行处理，插件可以：
    /// - 添加元数据（如处理时间、消息长度）
    /// - 修改或增强日志内容
    pub async fn analyze_log_file(&self, file_path: &str) -> Result<Vec<LogEntry>> {
        let _file = LogFile::new(
            file_path.to_string(),
            0, // 将在实际处理时更新
            Utc::now(),
        );

        // 这里将实现实际的日志分析逻辑
        let mut entries = Vec::new();

        // 模拟日志条目
        let mut entry = LogEntry::new(
            Utc::now(),
            LogLevel::Info,
            "Sample log message".to_string(),
            file_path.to_string(),
            1,
        );

        // 通过插件处理日志条目
        if let Err(e) = self.plugins.process_log(&mut entry).await {
            tracing::warn!("插件处理日志条目失败: {}", e);
        }

        entries.push(entry);

        Ok(entries)
    }

    /// 搜索日志
    ///
    /// # 参数
    /// - `query`: 搜索查询字符串
    ///
    /// # 返回
    /// 返回匹配的日志条目列表
    ///
    /// # 插件处理
    /// 搜索查询会先通过插件进行预处理，插件可以：
    /// - 过滤敏感词
    /// - 增强查询语法
    /// - 添加额外的搜索条件
    pub async fn search_logs(&self, query: &str) -> Result<Vec<LogEntry>> {
        // 通过插件预处理搜索查询
        let processed_query = match self.plugins.process_search(query).await {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!("插件处理搜索查询失败，使用原始查询: {}", e);
                query.to_string()
            }
        };

        // 这里将实现实际的搜索逻辑
        let mut entries = Vec::new();

        // 模拟搜索结果
        let mut entry = LogEntry::new(
            Utc::now(),
            LogLevel::Info,
            format!("Search result for: {}", processed_query),
            "sample.log".to_string(),
            1,
        );

        // 通过插件处理结果条目
        if let Err(e) = self.plugins.process_log(&mut entry).await {
            tracing::warn!("插件处理搜索结果失败: {}", e);
        }

        entries.push(entry);

        Ok(entries)
    }

    /// 获取系统状态
    ///
    /// # 返回
    /// 返回包含系统状态信息的 JSON 对象，包括：
    /// - 运行状态
    /// - 版本信息
    /// - 已加载插件数量
    /// - 关键配置参数
    pub async fn get_system_status(&self) -> Result<serde_json::Value> {
        // 获取已加载的插件列表
        let loaded_plugins = self.plugins.get_plugins().await;

        Ok(serde_json::json!({
            "status": "running",
            "version": "2.0.0",
            "plugins_loaded": loaded_plugins.len(),
            "plugins": loaded_plugins,
            "config": {
                "max_results": self.config.search.max_results,
                "timeout_seconds": self.config.search.timeout_seconds,
            }
        }))
    }

    /// 初始化所有插件
    ///
    /// # 参数
    /// - `config`: 传递给插件的配置信息
    ///
    /// # 返回
    /// 成功返回 Ok(())，失败返回错误信息
    ///
    /// # 用例
    /// 在应用启动时调用，确保所有插件正确初始化
    pub async fn initialize_plugins(&self, config: &serde_json::Value) -> Result<()> {
        self.plugins.initialize_all(config).await
    }

    /// 加载指定路径的插件
    ///
    /// # 参数
    /// - `path`: 插件动态库文件路径
    ///
    /// # 安全性
    /// - 仅允许从配置的插件目录加载
    /// - 进行 ABI 版本验证
    /// - 验证插件符号表完整性
    pub async fn load_plugin(&self, path: &std::path::Path) -> Result<()> {
        self.plugins.load_plugin(path).await
    }

    /// 卸载指定名称的插件
    ///
    /// # 参数
    /// - `name`: 插件名称
    ///
    /// # 返回
    /// 成功返回 Ok(())，如果插件不存在也会返回 Ok(())
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        self.plugins.unload_plugin(name).await
    }
}

/// 配置管理服务
pub struct ConfigurationService {
    config: Arc<RwLock<AppConfig>>,
}

impl ConfigurationService {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, new_config: AppConfig) -> Result<()> {
        *self.config.write().await = new_config;
        Ok(())
    }

    /// 获取配置摘要
    pub async fn get_config_summary(&self) -> serde_json::Value {
        let config = self.config.read().await;
        serde_json::json!({
            "server": {
                "host": config.server.host,
                "port": config.server.port,
            },
            "search": {
                "max_results": config.search.max_results,
                "timeout_seconds": config.search.timeout_seconds,
            },
            "monitoring": {
                "log_level": config.monitoring.log_level,
                "metrics_enabled": config.monitoring.metrics_enabled,
            }
        })
    }
}
