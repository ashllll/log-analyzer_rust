//! 服务层共享 trait 抽象
//!
//! 定义存储和服务层的核心 trait，供多个 crate 共享使用。
//! 遵循依赖倒置原则 (DIP)，解耦具体实现与消费者。

use crate::error::Result;
use crate::models::search::SearchQuery;
use crate::storage_types::FileMetadata;
use async_trait::async_trait;

/// 查询验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 查询是否有效
    pub is_valid: bool,
    /// 验证错误列表（有效时为空）
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// 创建成功的验证结果
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    /// 创建带错误列表的失败验证结果
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
        }
    }

    /// 创建带单条错误的失败验证结果
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            errors: vec![msg.into()],
        }
    }
}

/// 查询验证 trait
///
/// 实现者可以验证搜索查询的正确性和约束条件。
/// 该 trait 是 object-safe 的，可用于依赖注入。
pub trait QueryValidation: Send + Sync {
    /// 验证搜索查询
    ///
    /// # Arguments
    /// * `query` - 待验证的搜索查询
    ///
    /// # Returns
    /// 包含验证状态和错误信息的 `ValidationResult`
    fn validate(&self, query: &SearchQuery) -> ValidationResult;
}

/// 查询计划结果
///
/// 计划的简化表示，具体计划细节由实现决定。
#[derive(Debug, Clone)]
pub struct PlanResult {
    /// 执行步骤描述
    pub steps: Vec<String>,
    /// 估计成本（任意单位，越低越好）
    pub estimated_cost: u32,
}

impl PlanResult {
    /// 创建新的计划结果
    pub fn new(steps: Vec<String>, estimated_cost: u32) -> Self {
        Self {
            steps,
            estimated_cost,
        }
    }
}

/// 内容存储 trait（CAS 抽象）
///
/// 抽象内容寻址存储操作，允许不同的存储后端互换使用。
#[async_trait]
pub trait ContentStorage: Send + Sync {
    /// 存储内容并返回其哈希值
    ///
    /// # Arguments
    /// * `content` - 要存储的内容字节
    ///
    /// # Returns
    /// 内容哈希（通常为 SHA-256）的字符串表示
    async fn store(&self, content: &[u8]) -> Result<String>;

    /// 通过哈希值检索内容
    ///
    /// # Arguments
    /// * `hash` - 内容哈希值
    ///
    /// # Returns
    /// 内容字节，未找到时返回错误
    async fn retrieve(&self, hash: &str) -> Result<Vec<u8>>;

    /// 检查内容是否存在
    ///
    /// # Arguments
    /// * `hash` - 内容哈希值
    ///
    /// # Returns
    /// 内容存在时返回 `true`，否则返回 `false`
    async fn exists(&self, hash: &str) -> bool;
}

/// 元数据存储 trait
///
/// 抽象元数据存储操作，支持不同的数据库后端或测试 mock。
#[async_trait]
pub trait MetadataStorage: Send + Sync {
    /// 插入文件元数据
    ///
    /// # Arguments
    /// * `metadata` - 要插入的文件元数据
    ///
    /// # Returns
    /// 自动生成的文件 ID
    async fn insert_file(&self, metadata: &FileMetadata) -> Result<i64>;

    /// 获取所有文件
    ///
    /// # Returns
    /// 所有文件元数据的向量
    async fn get_all_files(&self) -> Result<Vec<FileMetadata>>;

    /// 通过哈希值获取文件
    ///
    /// # Arguments
    /// * `hash` - 文件哈希值
    ///
    /// # Returns
    /// 找到的文件元数据，或 `None`
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileMetadata>>;
}

/// 通用查询执行器 trait
///
/// 抽象查询执行，支持不同的执行策略和便捷测试。
pub trait QueryExecutor: Send + Sync {
    /// 执行搜索查询并返回结果
    ///
    /// # Type Parameters
    /// * `T` - 结果类型
    ///
    /// # Arguments
    /// * `query` - 要执行的搜索查询
    ///
    /// # Returns
    /// 查询结果或错误
    fn execute<T>(&self, query: &SearchQuery) -> Result<T>
    where
        T: Send + Sync + 'static;
}

/// 应用配置提供者 trait
///
/// 解耦业务逻辑对 Tauri AppHandle 的直接依赖。
/// 具体实现在主 crate 中为 `tauri::AppHandle` 提供。
pub trait AppConfigProvider: Send + Sync {
    /// 获取应用配置目录路径
    fn config_dir(&self) -> std::result::Result<std::path::PathBuf, String>;
}
