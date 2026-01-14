use chrono::{DateTime, Utc};
use std::fmt;

/// 时间戳值对象
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub fn new(datetime: DateTime<Utc>) -> Self {
        Self(datetime)
    }

    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

/// 日志级别值对象
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown(String),
}

impl LogLevel {
    pub fn from_str(level: &str) -> Self {
        match level.to_lowercase().as_str() {
            "trace" | "trc" => LogLevel::Trace,
            "debug" | "dbg" => LogLevel::Debug,
            "info" | "inf" => LogLevel::Info,
            "warn" | "warning" | "wrn" => LogLevel::Warn,
            "error" | "err" => LogLevel::Error,
            "fatal" | "ftl" => LogLevel::Fatal,
            _ => LogLevel::Unknown(level.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
            LogLevel::Unknown(s) => s.as_str(),
        }
    }

    pub fn severity(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
            LogLevel::Fatal => 5,
            LogLevel::Unknown(_) => 2,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 日志消息值对象
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogMessage(String);

impl LogMessage {
    pub fn new(message: String) -> Self {
        Self(message.trim().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, pattern: &str) -> bool {
        self.0.contains(pattern)
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
}

impl fmt::Display for LogMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for LogMessage {
    fn from(s: String) -> Self {
        LogMessage::new(s)
    }
}

impl From<&str> for LogMessage {
    fn from(s: &str) -> Self {
        LogMessage::new(s.to_string())
    }
}