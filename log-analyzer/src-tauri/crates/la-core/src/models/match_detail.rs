use serde::{Deserialize, Serialize};

/// 匹配结果详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetail {
    /// 匹配的搜索项ID
    pub term_id: String,
    /// 匹配的关键词值
    pub term_value: String,
    /// 优先级
    pub priority: u32,
    /// 匹配位置（可选）
    pub match_position: Option<(usize, usize)>,
}
