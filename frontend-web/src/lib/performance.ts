import { onCLS, onFCP, onINP, onLCP, onTTFB } from 'web-vitals';

interface PerformanceMetric {
  name: string;
  value: number;
  rating: 'good' | 'needs-improvement' | 'poor';
  timestamp: number;
}

class PerformanceMonitor {
  private metrics: PerformanceMetric[] = [];

  constructor() {
    if (typeof window !== 'undefined') {
      this.init();
    }
  }

  private init() {
    // Core Web Vitals - web-vitals v4+ uses on* functions
    onLCP(this.recordMetric);
    onCLS(this.recordMetric);

    // New metrics (FID replaced by INP in v4+)
    onTTFB(this.recordMetric);
    onFCP(this.recordMetric);
    onINP(this.recordMetric);
  }

  private recordMetric = (metric: any) => {
    const performanceMetric: PerformanceMetric = {
      name: metric.name,
      value: metric.value,
      rating: metric.rating,
      timestamp: Date.now(),
    };

    this.metrics.push(performanceMetric);

    // Log to console in development
    if (import.meta.env.DEV) {
      console.log('[Performance]', performanceMetric);
    }

    // Send to analytics in production
    if (!import.meta.env.DEV && import.meta.env.PROD) {
      this.sendToAnalytics(performanceMetric);
    }
  };

  private sendToAnalytics(metric: PerformanceMetric) {
    // Send to your analytics service
    // Example: fetch('/api/metrics', { method: 'POST', body: JSON.stringify(metric) });
    console.log('[Analytics] Sending metric:', metric);
  }

  public getMetrics(): PerformanceMetric[] {
    return [...this.metrics];
  }

  public getMetricsByRating(rating: 'good' | 'needs-improvement' | 'poor'): PerformanceMetric[] {
    return this.metrics.filter(m => m.rating === rating);
  }
}

// Export singleton instance
export const performanceMonitor = new PerformanceMonitor();
