//! Simple test for monitoring system functionality

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::{metrics_collector::MetricsCollector, ProductionMonitor};
    use std::time::Duration;

    #[tokio::test]
    async fn test_production_monitor_creation() {
        let result = ProductionMonitor::new();
        assert!(result.is_ok(), "Should be able to create ProductionMonitor");
    }

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let result = MetricsCollector::new();
        assert!(result.is_ok(), "Should be able to create MetricsCollector");
    }

    #[test]
    fn test_counter_operations() {
        use crate::monitoring::metrics_collector::Counter;
        use std::collections::HashMap;

        let counter = Counter::new("test_counter".to_string(), HashMap::new());

        assert_eq!(counter.get(), 0);

        counter.increment();
        assert_eq!(counter.get(), 1);

        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge_operations() {
        use crate::monitoring::metrics_collector::Gauge;
        use std::collections::HashMap;

        let gauge = Gauge::new("test_gauge".to_string(), HashMap::new());

        assert_eq!(gauge.get(), 0.0);

        gauge.set(42.5);
        assert_eq!(gauge.get(), 42.5);

        gauge.add(7.5);
        assert_eq!(gauge.get(), 50.0);
    }

    #[test]
    fn test_histogram_operations() {
        use crate::monitoring::metrics_collector::Histogram;
        use std::collections::HashMap;

        let buckets = vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0];
        let histogram = Histogram::new("test_histogram".to_string(), buckets, HashMap::new());

        histogram.observe(3.0);
        histogram.observe(15.0);
        histogram.observe(75.0);

        let metric = histogram.to_histogram_metric();
        assert_eq!(metric.total_count, 3);
        assert_eq!(metric.sum, 93.0);
    }
}
