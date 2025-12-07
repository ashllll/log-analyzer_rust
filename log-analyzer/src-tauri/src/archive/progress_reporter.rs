//! 智能进度报告器模块
//!
//! 减少进度事件发送频率,提供更有价值的进度信息

use std::time::Instant;

/// 进度报告器
///
/// 用于智能地报告解压进度,避免频繁发送事件
#[derive(Debug)]
pub struct ProgressReporter {
    /// 总条目数
    total_entries: usize,
    /// 已处理条目数
    processed_entries: usize,
    /// 上次报告的百分比
    last_reported_percentage: u8,
    /// 报告间隔(百分比)
    report_interval: u8,
    /// 开始时间
    start_time: Instant,
    /// 上次报告时间
    last_report_time: Instant,
}

impl ProgressReporter {
    /// 创建新的进度报告器
    ///
    /// # Arguments
    ///
    /// * `total_entries` - 总条目数
    /// * `report_interval` - 报告间隔(百分比,默认5)
    pub fn new(total_entries: usize, report_interval: u8) -> Self {
        let now = Instant::now();
        Self {
            total_entries,
            processed_entries: 0,
            last_reported_percentage: 0,
            report_interval,
            start_time: now,
            last_report_time: now,
        }
    }

    /// 记录一个条目处理完成
    pub fn inc(&mut self) {
        self.processed_entries += 1;
    }

    /// 判断是否应该报告进度
    ///
    /// # Returns
    ///
    /// 如果应该报告进度返回true
    pub fn should_report(&self) -> bool {
        if self.total_entries == 0 {
            return false;
        }

        let current_pct = (self.processed_entries * 100 / self.total_entries) as u8;

        // 条件1:百分比变化超过间隔
        let pct_changed = current_pct >= self.last_reported_percentage + self.report_interval;

        // 条件2:超过2秒未报告
        let time_elapsed = self.last_report_time.elapsed().as_secs() >= 2;

        // 条件3:最后一个条目
        let is_last = self.processed_entries == self.total_entries;

        pct_changed || time_elapsed || is_last
    }

    /// 获取进度消息
    pub fn get_progress_message(&self) -> String {
        if self.total_entries == 0 {
            return "正在处理...".to_string();
        }

        let percentage = (self.processed_entries * 100 / self.total_entries) as u8;
        let elapsed = self.start_time.elapsed().as_secs();
        let speed = if elapsed > 0 {
            self.processed_entries as f64 / elapsed as f64
        } else {
            0.0
        };
        let remaining = if speed > 0.0 {
            ((self.total_entries - self.processed_entries) as f64 / speed) as u64
        } else {
            0
        };

        format!(
            "正在解压... {}% ({}/{} 文件, 速度: {:.1}文件/秒, 预计剩余: {}秒)",
            percentage, self.processed_entries, self.total_entries, speed, remaining
        )
    }

    /// 获取当前百分比
    pub fn percentage(&self) -> u8 {
        if self.total_entries == 0 {
            return 0;
        }
        (self.processed_entries * 100 / self.total_entries) as u8
    }

    /// 标记已报告
    pub fn mark_reported(&mut self) {
        self.last_reported_percentage = self.percentage();
        self.last_report_time = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new() {
        let reporter = ProgressReporter::new(100, 5);
        assert_eq!(reporter.total_entries, 100);
        assert_eq!(reporter.processed_entries, 0);
        assert_eq!(reporter.percentage(), 0);
    }

    #[test]
    fn test_inc() {
        let mut reporter = ProgressReporter::new(100, 5);
        reporter.inc();
        assert_eq!(reporter.processed_entries, 1);
        assert_eq!(reporter.percentage(), 1);
    }

    #[test]
    fn test_percentage() {
        let mut reporter = ProgressReporter::new(100, 5);
        assert_eq!(reporter.percentage(), 0);

        reporter.processed_entries = 50;
        assert_eq!(reporter.percentage(), 50);

        reporter.processed_entries = 100;
        assert_eq!(reporter.percentage(), 100);
    }

    #[test]
    fn test_should_report_percentage_change() {
        let mut reporter = ProgressReporter::new(100, 5);

        // 初始不应该报告
        assert!(!reporter.should_report());

        // 处理5个条目(5%),应该报告
        reporter.processed_entries = 5;
        assert!(reporter.should_report());

        // 标记已报告
        reporter.mark_reported();
        assert!(!reporter.should_report());

        // 再处理5个(10%),应该报告
        reporter.processed_entries = 10;
        assert!(reporter.should_report());
    }

    #[test]
    fn test_should_report_last_entry() {
        let mut reporter = ProgressReporter::new(100, 5);

        // 处理到最后一个
        reporter.processed_entries = 100;
        assert!(reporter.should_report()); // 最后一个必须报告
    }

    #[test]
    fn test_should_report_time_elapsed() {
        let mut reporter = ProgressReporter::new(1000, 5);

        // 只处理1个条目(远小于5%)
        reporter.processed_entries = 1;
        assert!(!reporter.should_report());

        // 等待2秒
        thread::sleep(Duration::from_secs(2));

        // 超过2秒应该报告
        assert!(reporter.should_report());
    }

    #[test]
    fn test_get_progress_message() {
        let mut reporter = ProgressReporter::new(100, 5);
        reporter.processed_entries = 50;

        let msg = reporter.get_progress_message();
        assert!(msg.contains("50%"));
        assert!(msg.contains("50/100"));
    }

    #[test]
    fn test_zero_total() {
        let reporter = ProgressReporter::new(0, 5);
        assert_eq!(reporter.percentage(), 0);
        assert!(!reporter.should_report());
        assert!(reporter.get_progress_message().contains("正在处理"));
    }
}
