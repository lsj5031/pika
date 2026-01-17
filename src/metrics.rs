// Performance monitoring utilities for the Rust backend

use std::time::Instant;

#[derive(Debug, Clone)]
pub struct MetricData {
    pub name: String,
    pub value: f64,
    pub timestamp: std::time::SystemTime,
}

pub struct PerformanceMetrics {
    metrics: Vec<MetricData>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }

    pub fn record_timing(&mut self, name: String, duration_ms: f64) {
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

    pub fn duration_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

#[macro_export]
macro_rules! timed {
    ($metrics:expr, $name:expr, $block:expr) => {{
        let timer = Timer::new($name.to_string());
        let result = $block;
        $metrics.record_timing($name.to_string(), timer.duration_ms());
        result
    }};
}
