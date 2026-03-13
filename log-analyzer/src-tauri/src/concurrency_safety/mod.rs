//! 并发安全解决方案模块
//!
//! 提供针对以下并发安全问题的完整解决方案：
//! 1. CAS 存储 TOCTOU 问题 - 原子写入方案
//! 2. 搜索引擎伪异步 - spawn_blocking 模式
//! 3. 缺乏背压机制 - Semaphore 限流
//! 4. 取消机制不完善 - 协作式取消

pub mod backpressure;
pub mod spawn_blocking_pool;

// 重新导出主要类型
pub use backpressure::{BackpressureController, RateLimiter, SemaphoreConfig};
pub use spawn_blocking_pool::{BlockingPool, BlockingPoolConfig, CpuIntensiveTask};
