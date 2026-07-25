//! 共享默认值函数
//!
//! 被多个配置子模块使用，避免重复定义。

#![allow(dead_code)]

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

pub fn default_none<T>() -> Option<T> {
    None
}

pub fn default_10_usize() -> usize {
    10
}

pub fn default_100_usize() -> usize {
    100
}

pub fn default_1000_usize() -> usize {
    1000
}

pub fn default_100000_usize() -> usize {
    100_000
}

pub fn default_10_u64() -> u64 {
    10
}

pub fn default_30_u64() -> u64 {
    30
}

pub fn default_60_u64() -> u64 {
    60
}

pub fn default_100_u64() -> u64 {
    100
}

pub fn default_300_u64() -> u64 {
    300
}

pub fn default_500_u64() -> u64 {
    500
}

pub fn default_1000_u64() -> u64 {
    1000
}

pub fn default_10000_u64() -> u64 {
    10_000
}

pub fn default_30000_u64() -> u64 {
    30_000
}

pub fn default_300000_u64() -> u64 {
    300_000
}

pub fn default_600000_u64() -> u64 {
    600_000
}

pub fn default_64kb() -> u64 {
    64 * 1024
}

pub fn default_10mb() -> u64 {
    10 * 1024 * 1024
}

pub fn default_100mb() -> u64 {
    100 * 1024 * 1024
}

pub fn default_1gb() -> u64 {
    1024 * 1024 * 1024
}

pub fn default_10gb() -> u64 {
    10 * 1024 * 1024 * 1024
}

pub fn default_50gb() -> u64 {
    50 * 1024 * 1024 * 1024
}

pub fn default_5_u64() -> u64 {
    5
}
