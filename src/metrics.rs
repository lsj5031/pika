// Performance monitoring utilities for the Rust backend

use std::time::Instant;

const MAX_METRICS: usize = 10000;

#[derive(Debug, Clone)]
pub struct MetricData {
    pub name: String,
    pub value: f64,
    pub timestamp: std::time::SystemTime,
}

pub struct PerformanceMetrics {
    metrics: Vec<MetricData>,
    max_size: usize,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self::with_capacity(MAX_METRICS)
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            metrics: Vec::with_capacity(max_size.min(1000)),
            max_size,
        }
    }

    pub fn record_timing(&mut self, name: String, duration_ms: f64) {
        if self.metrics.len() >= self.max_size {
            self.metrics.remove(0);
        }
        self.metrics.push(MetricData {
            name,
            value: duration_ms,
            timestamp: std::time::SystemTime::now(),
        });
    }

    pub fn get_metrics(&self) -> &[MetricData] {
        &self.metrics
    }

    pub fn clear(&mut self) {
        self.metrics.clear();
    }

    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Timer {
    name: String,
    start: Instant,
}

impl Timer {
    pub fn new(name: String) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn duration_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

#[macro_export]
macro_rules! timed {
    ($metrics:expr, $name:expr, $block:expr) => {{
        let timer = $crate::metrics::Timer::new($name.to_string());
        let result = $block;
        $metrics.record_timing($name.to_string(), timer.duration_ms());
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_cap() {
        let mut metrics = PerformanceMetrics::with_capacity(3);
        metrics.record_timing("a".to_string(), 1.0);
        metrics.record_timing("b".to_string(), 2.0);
        metrics.record_timing("c".to_string(), 3.0);
        metrics.record_timing("d".to_string(), 4.0);

        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics.get_metrics()[0].name, "b");
        assert_eq!(metrics.get_metrics()[2].name, "d");
    }
}
