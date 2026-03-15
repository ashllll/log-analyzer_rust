//! 插件化架构系统
//!
//! 支持动态加载插件，实现功能扩展

use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::domain::log_analysis::entities::LogEntry;
use crate::error::Result;

/// 插件 ABI 版本，用于确保插件与主程序兼容
pub const PLUGIN_ABI_VERSION: u32 = 1;

/// 插件接口定义
pub trait Plugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &'static str;

    /// 插件版本
    fn version(&self) -> &'static str;

    /// 插件 ABI 版本
    fn abi_version(&self) -> u32 {
        PLUGIN_ABI_VERSION
    }

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

/// 库引用计数包装器
///
/// 使用 Arc 实现引用计数，跟踪库的使用情况
#[derive(Clone)]
struct LibraryHandle {
    /// 库路径，用于日志记录和调试
    path: std::path::PathBuf,
    /// 实际的动态库，使用 Arc 实现引用计数
    library: Arc<Library>,
    /// 引用计数，用于调试
    ref_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl LibraryHandle {
    /// 创建新的库句柄
    fn new(path: std::path::PathBuf, library: Library) -> Self {
        Self {
            path,
            library: Arc::new(library),
            ref_count: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        }
    }

    /// 增加引用计数
    fn increment(&self) {
        let count = self.ref_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!(
            path = ?self.path,
            count = count + 1,
            "Library reference incremented"
        );
    }

    /// 减少引用计数并返回当前值
    fn decrement(&self) -> usize {
        let count = self.ref_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug!(
            path = ?self.path,
            count = count - 1,
            "Library reference decremented"
        );
        count.saturating_sub(1)
    }

    /// 获取当前引用计数
    fn ref_count(&self) -> usize {
        self.ref_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// 插件管理器
pub struct PluginManager {
    /// 已加载的插件，key 为插件名称
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    /// 库路径到库句柄的映射，使用引用计数跟踪
    loaded_libraries: Arc<RwLock<HashMap<std::path::PathBuf, LibraryHandle>>>,
    /// 插件名称到库路径的映射，用于卸载时查找对应的库
    plugin_to_library: Arc<RwLock<HashMap<String, std::path::PathBuf>>>,
    /// 插件目录，用于安全验证
    plugin_directory: std::path::PathBuf,
}

impl PluginManager {
    pub fn new(plugin_dir: std::path::PathBuf) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            loaded_libraries: Arc::new(RwLock::new(HashMap::new())),
            plugin_to_library: Arc::new(RwLock::new(HashMap::new())),
            plugin_directory: plugin_dir,
        }
    }

    /// 加载插件
    pub async fn load_plugin(&self, path: &Path) -> Result<()> {
        // 1. 安全性检查：规范化路径并验证
        let canonical_path = path
            .canonicalize()
            .map_err(|e| crate::error::AppError::Internal(format!("Invalid plugin path: {}", e)))?;

        // ✅ 安全性检查：仅允许从指定目录加载插件，防止非法路径加载恶意库
        if !canonical_path.starts_with(&self.plugin_directory) {
            return Err(crate::error::AppError::Security(format!(
                "Plugin path not in whitelist: {:?}",
                canonical_path
            )));
        }

        // 2. 检查库是否已加载，如果已加载则增加引用计数
        // 如果库已加载，我们返回克隆的句柄
        let (library_handle, is_new_load) = {
            let mut libs = self.loaded_libraries.write().await;
            if let Some(handle) = libs.get(&canonical_path) {
                // 库已加载，增加引用计数
                handle.increment();
                info!(
                    path = ?canonical_path,
                    ref_count = handle.ref_count(),
                    "Plugin library already loaded, incremented reference count"
                );
                (handle.clone(), false)
            } else {
                // 3. 加载新的动态库
                let lib = unsafe { Library::new(&canonical_path) }.map_err(|e| {
                    crate::error::AppError::Internal(format!("Failed to load plugin: {}", e))
                })?;

                let handle = LibraryHandle::new(canonical_path.clone(), lib);
                info!(
                    path = ?canonical_path,
                    "Loaded new plugin library"
                );
                libs.insert(canonical_path.clone(), handle.clone());
                (handle, true)
            }
        };

        // 4. 获取创建函数
        // 注意：这是一个技术挑战点。libloading 的 Symbol 生命周期绑定到 Library，
        // 但我们的 Library 被 Arc 包装。
        // 解决方案：在检查库是否已加载后，如果已加载则跳过 symbol 获取和插件创建。
        // 只有新加载的库才需要获取 symbol 并创建插件。
        //
        // 但实际上，每个插件都需要自己的实例，所以即使库已加载，
        // 我们仍然需要调用 create_plugin 来创建新的插件实例。
        //
        // 安全的解决方案：使用 Arc 的 into_inner 或手动管理生命周期
        type PluginCreate = unsafe fn() -> *mut dyn Plugin;

        // 我们需要从 Arc 中获取 Library 的引用来获取 symbol
        // 这需要小心处理生命周期问题
        let plugin = unsafe {
            // 创建一个临时的 Library 引用
            // 这是安全的，因为我们持有 Arc，Library 不会被卸载
            let lib_ref = &*Arc::as_ptr(&library_handle.library);

            // 获取 symbol
            let create_plugin: Symbol<PluginCreate> = lib_ref.get(b"create_plugin").map_err(|e| {
                crate::error::AppError::Internal(format!(
                    "Plugin missing create_plugin symbol: {}",
                    e
                ))
            })?;

            // 调用创建函数
            let plugin_raw = create_plugin();
            if plugin_raw.is_null() {
                // 创建失败，减少引用计数
                library_handle.decrement();
                return Err(crate::error::AppError::Internal(
                    "Plugin creation returned null pointer".to_string(),
                ));
            }

            // 转换为 Box
            let plugin = Box::from_raw(plugin_raw);

            // 验证 ABI 版本（在移动前检查）
            let abi_version = plugin.abi_version();
            if abi_version != PLUGIN_ABI_VERSION {
                // ABI 版本不匹配，减少引用计数并清理
                let _ = Box::into_raw(plugin); // 避免 double free
                library_handle.decrement();
                return Err(crate::error::AppError::Internal(format!(
                    "Plugin ABI mismatch: expected {}, found {}",
                    PLUGIN_ABI_VERSION, abi_version
                )));
            }

            plugin
        };

        let name = plugin.name().to_string();

        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(name.clone(), plugin);
        }

        {
            let mut plugin_to_lib = self.plugin_to_library.write().await;
            plugin_to_lib.insert(name.clone(), canonical_path.clone());
        }

        info!(
            plugin_name = %name,
            path = ?canonical_path,
            ref_count = library_handle.ref_count(),
            is_new_load = is_new_load,
            "Plugin loaded successfully"
        );

        Ok(())
    }

    /// 卸载插件
    ///
    /// 当引用计数降为 0 时，真正卸载动态库以释放内存
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        // 1. 获取并移除插件实例
        let plugin = {
            let mut plugins = self.plugins.write().await;
            plugins.remove(name)
        };

        if plugin.is_none() {
            warn!(plugin_name = %name, "Plugin not found for unloading");
            return Ok(());
        }

        let plugin = plugin.unwrap();

        // 2. 调用插件的清理方法
        plugin.cleanup()?;

        // 3. 获取对应的库路径
        let library_path = {
            let mut plugin_to_lib = self.plugin_to_library.write().await;
            plugin_to_lib.remove(name)
        };

        let library_path = match library_path {
            Some(path) => path,
            None => {
                warn!(
                    plugin_name = %name,
                    "No library path found for plugin"
                );
                return Ok(());
            }
        };

        // 4. 减少库的引用计数，如果降为 0 则卸载
        let should_unload = {
            let mut libs = self.loaded_libraries.write().await;
            if let Some(handle) = libs.get(&library_path) {
                let count = handle.decrement();
                if count == 0 {
                    // 引用计数降为 0，卸载库
                    info!(
                        path = ?library_path,
                        plugin_name = %name,
                        "Unloading plugin library (reference count reached 0)"
                    );
                    libs.remove(&library_path);
                    true
                } else {
                    info!(
                        path = ?library_path,
                        plugin_name = %name,
                        ref_count = count,
                        "Plugin unloaded, but library still in use (reference count > 0)"
                    );
                    false
                }
            } else {
                warn!(
                    path = ?library_path,
                    plugin_name = %name,
                    "Library handle not found for plugin"
                );
                false
            }
        };

        if should_unload {
            info!(
                plugin_name = %name,
                path = ?library_path,
                "Plugin unloaded and library freed"
            );
        } else {
            info!(
                plugin_name = %name,
                path = ?library_path,
                "Plugin unloaded (library still referenced by other plugins)"
            );
        }

        Ok(())
    }

    /// 获取已加载的插件
    pub async fn get_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }

    /// 获取已加载的库及其引用计数信息
    ///
    /// 用于调试和监控，返回每个库的路径和引用计数
    pub async fn get_library_info(&self) -> Vec<(String, usize)> {
        let libs = self.loaded_libraries.read().await;
        libs.iter()
            .map(|(path, handle)| (path.to_string_lossy().to_string(), handle.ref_count()))
            .collect()
    }

    /// 获取已加载的库数量
    pub async fn loaded_library_count(&self) -> usize {
        let libs = self.loaded_libraries.read().await;
        libs.len()
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
        entry.add_metadata(
            "message_length".to_string(),
            entry.message.len().to_string(),
        );

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
