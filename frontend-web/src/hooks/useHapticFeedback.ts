type HapticPattern = "light" | "medium" | "heavy" | "success" | "warning" | "error";

export function useHapticFeedback() {
  const triggerHaptic = (pattern: HapticPattern = "light") => {
    if (!navigator.vibrate && !(window as any).navigator.vibrate) {
      return;
    }

    const patterns: Record<HapticPattern, number[]> = {
      light: [10],
      medium: [20],
      heavy: [30],
      success: [10, 50, 10],
      warning: [20, 30, 20],
      error: [40, 30, 40, 30, 40],
    };

    const vibrationPattern = patterns[pattern] || patterns.light;

    try {
      navigator.vibrate(vibrationPattern);
    } catch {
    }
  };

  return { triggerHaptic };
}

export function withHaptic<T extends (...args: unknown[]) => unknown>(
  fn: T,
  pattern: HapticPattern = "light"
): T {
  return ((...args: unknown[]) => {
    if (navigator.vibrate) {
      const patterns: Record<HapticPattern, number[]> = {
        light: [10],
        medium: [20],
        heavy: [30],
        success: [10, 50, 10],
        warning: [20, 30, 20],
        error: [40, 30, 40, 30, 40],
      };

      try {
        navigator.vibrate(patterns[pattern] || patterns.light);
      } catch {
      }
    }

    return fn(...args);
  }) as T;
}
