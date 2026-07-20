//! State synchronization data models

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Workspace event types
///
/// 线上协议事实：目前仅有 `StatusChanged` 一个变体在生产代码中构造
/// （load_workspace / delete_workspace）。历史上的 ProgressUpdate /
/// TaskCompleted / Error 变体从未被发送（进度与任务生命周期由
/// `task-update` 通道承载），已作为死协议面移除。
///
/// 线上 payload 形状由 `state_sync/contract_tests.rs` 与前端共享夹具
/// `log-analyzer/src/events/__fixtures__/workspace-event-contract.json`
/// 双向锁定，禁止单方面漂移。
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum WorkspaceEvent {
    StatusChanged {
        workspace_id: String,
        status: WorkspaceStatus,
    },
    /// Watch mode: 新内容已写入搜索索引（轻量信号，不含日志负载）。
    FilesUpdated {
        workspace_id: String,
        /// 本轮 debounce 窗口内累计写入的行数
        new_lines: u64,
    },
}

/// Workspace status
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "status")]
pub enum WorkspaceStatus {
    Idle,
    Processing {
        #[serde(with = "system_time_serde")]
        started_at: SystemTime,
    },
    Completed {
        #[serde(with = "duration_serde")]
        duration: Duration,
    },
    Failed {
        error: String,
        #[serde(with = "system_time_serde")]
        failed_at: SystemTime,
    },
    Cancelled {
        #[serde(with = "system_time_serde")]
        cancelled_at: SystemTime,
    },
}

// Serde helpers for SystemTime
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
