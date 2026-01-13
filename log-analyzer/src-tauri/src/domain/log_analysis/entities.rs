use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use super::value_objects::{LogLevel, LogMessage, Timestamp};

/// 日志条目实体
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub id: Uuid,
    pub timestamp: Timestamp,
    pub level: LogLevel,
    pub message: LogMessage,
    pub source_file: String,
    pub line_number: u64,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
}

impl LogEntry {
    pub fn new(
        timestamp: DateTime<Utc>,
        level: LogLevel,
        message: String,
        source_file: String,
        line_number: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Timestamp::new(timestamp),
            level,
            message: LogMessage::new(message),
            source_file,
            line_number,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// 日志文件实体
#[derive(Debug, Clone)]
pub struct LogFile {
    pub id: Uuid,
    pub path: String,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
    pub format: LogFormat,
    pub entries_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogFormat {
    PlainText,
    Json,
    Syslog,
    Apache,
    Nginx,
    Custom(String),
}

impl LogFile {
    pub fn new(path: String, size: u64, last_modified: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
            size,
            last_modified,
            format: LogFormat::PlainText,
            entries_count: 0,
        }
    }

    pub fn set_format(&mut self, format: LogFormat) {
        self.format = format;
    }

    pub fn increment_entries(&mut self) {
        self.entries_count += 1;
    }
}