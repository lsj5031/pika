import { useEffect, useRef, useCallback } from "react";

interface PerformanceMetrics {
  longTasks: number;
  totalLongTaskTime: number;
  frameDrops: number;
  averageFps: number;
  memoryGrowth: number;
}

interface MonitorOptions {
  onLongTask?: (duration: number, entry: PerformanceEntry) => void;
  onFrameDrop?: (fps: number) => void;
  onMemoryWarning?: (usedMB: number, growthMB: number) => void;
  longTaskThreshold?: number;
  frameDropThreshold?: number;
  memoryGrowthThreshold?: number;
  enableLogging?: boolean;
}

/**
 * Hook to monitor performance metrics and detect UI freezing issues
 * 
 * Usage:
 * ```tsx
 * usePerformanceMonitor({
 *   onLongTask: (duration) => console.warn(`Long task: ${duration}ms`),
 *   onFrameDrop: (fps) => console.warn(`Frame drop: ${fps} FPS`),
 *   enableLogging: true,
 * });
 * ```
 */
export function usePerformanceMonitor(options: MonitorOptions = {}) {
  const {
    onLongTask,
    onFrameDrop,
    onMemoryWarning,
    longTaskThreshold = 50,
    frameDropThreshold = 30,
    memoryGrowthThreshold = 50,
    enableLogging = false,
  } = options;

  const metricsRef = useRef<PerformanceMetrics>({
    longTasks: 0,
    totalLongTaskTime: 0,
    frameDrops: 0,
    averageFps: 60,
    memoryGrowth: 0,
  });

  const lastMemoryRef = useRef<number>(0);
  const frameCountRef = useRef<number>(0);
  const lastFrameTimeRef = useRef<number>(performance.now());
  const rafIdRef = useRef<number | null>(null);
  const observerRef = useRef<PerformanceObserver | null>(null);

  const log = useCallback((...args: unknown[]) => {
    if (enableLogging) {
      console.log("[PerformanceMonitor]", ...args);
    }
  }, [enableLogging]);

  useEffect(() => {
    // Skip in SSR
    if (typeof window === "undefined") return;

    // Monitor long tasks
    if ("PerformanceObserver" in window) {
      try {
        observerRef.current = new PerformanceObserver((list) => {
          const entries = list.getEntries();
          entries.forEach((entry) => {
            if (entry.duration > longTaskThreshold) {
              metricsRef.current.longTasks++;
              metricsRef.current.totalLongTaskTime += entry.duration;

              log(`Long task detected: ${Math.round(entry.duration)}ms`, entry.name);
              onLongTask?.(entry.duration, entry);
            }
          });
        });

        observerRef.current.observe({ entryTypes: ["longtask"] });
      } catch (e) {
        // Long task observer not supported
        log("Long task observer not supported");
      }
    }

    // Monitor frame rate
    const measureFrameRate = () => {
      const now = performance.now();
      frameCountRef.current++;

      if (now - lastFrameTimeRef.current >= 1000) {
        const fps = frameCountRef.current;
        metricsRef.current.averageFps = fps;

        if (fps < frameDropThreshold) {
          metricsRef.current.frameDrops++;
          log(`Frame drop detected: ${fps} FPS`);
          onFrameDrop?.(fps);
        }

        frameCountRef.current = 0;
        lastFrameTimeRef.current = now;
      }

      rafIdRef.current = requestAnimationFrame(measureFrameRate);
    };

    rafIdRef.current = requestAnimationFrame(measureFrameRate);

    // Monitor memory (if available)
    const memoryInterval = setInterval(() => {
      if ("memory" in performance) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const mem = (performance as any).memory;
        const usedMB = mem.usedJSHeapSize / 1024 / 1024;
        const growth = usedMB - lastMemoryRef.current;

        if (lastMemoryRef.current > 0 && growth > memoryGrowthThreshold) {
          metricsRef.current.memoryGrowth += growth;
          log(`Memory growth: +${Math.round(growth)}MB (total: ${Math.round(usedMB)}MB)`);
          onMemoryWarning?.(usedMB, growth);
        }

        lastMemoryRef.current = usedMB;
      }
    }, 5000);

    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
      if (rafIdRef.current) {
        cancelAnimationFrame(rafIdRef.current);
      }
      clearInterval(memoryInterval);
    };
  }, [
    longTaskThreshold,
    frameDropThreshold,
    memoryGrowthThreshold,
    onLongTask,
    onFrameDrop,
    onMemoryWarning,
    log,
  ]);

  // Return current metrics for inspection
  return {
    getMetrics: () => ({ ...metricsRef.current }),
    resetMetrics: () => {
      metricsRef.current = {
        longTasks: 0,
        totalLongTaskTime: 0,
        frameDrops: 0,
        averageFps: 60,
        memoryGrowth: 0,
      };
      lastMemoryRef.current = 0;
    },
  };
}

/**
 * Utility to measure render time of a component
 * 
 * Usage:
 * ```tsx
 * const { startMeasure, endMeasure } = useRenderMeasure('MyComponent');
 * 
 * useEffect(() => {
 *   startMeasure();
 *   return () => endMeasure();
 * }, [deps]);
 * ```
 */
export function useRenderMeasure(componentName: string, enabled = true) {
  const startTimeRef = useRef<number>(0);

  const startMeasure = useCallback(() => {
    if (!enabled) return;
    startTimeRef.current = performance.now();
  }, [enabled]);

  const endMeasure = useCallback(() => {
    if (!enabled || startTimeRef.current === 0) return;
    const duration = performance.now() - startTimeRef.current;
    if (duration > 16) { // Log if render takes more than one frame
      console.warn(`[RenderMeasure] ${componentName} rendered in ${Math.round(duration)}ms`);
    }
  }, [componentName, enabled]);

  return { startMeasure, endMeasure };
}

/**
 * Utility to throttle high-frequency updates
 * 
 * Usage:
 * ```tsx
 * const throttledUpdate = useThrottle((value) => {
 *   setState(value);
 * }, 50);
 * ```
 */
export function useThrottle<T extends (...args: unknown[]) => void>(
  callback: T,
  delay: number
): T {
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastArgsRef = useRef<Parameters<T> | null>(null);

  return useCallback((...args: Parameters<T>) => {
    lastArgsRef.current = args;

    if (timeoutRef.current === null) {
      timeoutRef.current = setTimeout(() => {
        if (lastArgsRef.current) {
          callback(...lastArgsRef.current);
        }
        timeoutRef.current = null;
      }, delay);
    }
  }, [callback, delay]) as T;
}

/**
 * Utility to batch rapid updates
 * 
 * Usage:
 * ```tsx
 * const batcher = useUpdateBatch<string>((values) => {
 *   setState(values.join(''));
 * }, 50);
 * 
 * batcher.add('a');
 * batcher.add('b'); // Both batched and applied together
 * ```
 */
export function useUpdateBatch<T>(
  flushCallback: (values: T[]) => void,
  batchWindowMs: number
) {
  const bufferRef = useRef<T[]>([]);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = useCallback(() => {
    if (bufferRef.current.length > 0) {
      flushCallback(bufferRef.current);
      bufferRef.current = [];
    }
    timeoutRef.current = null;
  }, [flushCallback]);

  const add = useCallback((value: T) => {
    bufferRef.current.push(value);

    if (timeoutRef.current === null) {
      timeoutRef.current = setTimeout(flush, batchWindowMs);
    }
  }, [flush, batchWindowMs]);

  const clear = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    bufferRef.current = [];
  }, []);

  return { add, flush, clear };
}
