import { useState, useCallback, useRef } from "react";

interface SwipeState {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  isSwiping: boolean;
  direction: "left" | "right" | "up" | "down" | null;
}

interface UseSwipeOptions {
  onSwipeLeft?: () => void;
  onSwipeRight?: () => void;
  onSwipeUp?: () => void;
  onSwipeDown?: () => void;
  threshold?: number;
  preventDefault?: boolean;
}

export function useSwipe(options: UseSwipeOptions = {}) {
  const {
    onSwipeLeft,
    onSwipeRight,
    onSwipeUp,
    onSwipeDown,
    threshold = 50,
    preventDefault = true,
  } = options;

  const [swipeState, setSwipeState] = useState<SwipeState>({
    startX: 0,
    startY: 0,
    currentX: 0,
    currentY: 0,
    isSwiping: false,
    direction: null,
  });

  const touchStartTime = useRef<number>(0);
  const isHorizontalSwipe = useRef<boolean | null>(null);

  const handleTouchStart = useCallback(
    (e: React.TouchEvent) => {
      const touch = e.touches[0];
      touchStartTime.current = Date.now();
      isHorizontalSwipe.current = null;

      setSwipeState({
        startX: touch.clientX,
        startY: touch.clientY,
        currentX: touch.clientX,
        currentY: touch.clientY,
        isSwiping: true,
        direction: null,
      });
    },
    []
  );

  const handleTouchMove = useCallback(
    (e: React.TouchEvent) => {
      if (!swipeState.isSwiping) return;

      const touch = e.touches[0];
      const deltaX = touch.clientX - swipeState.startX;
      const deltaY = touch.clientY - swipeState.startY;

      if (isHorizontalSwipe.current === null) {
        const absX = Math.abs(deltaX);
        const absY = Math.abs(deltaY);

        if (absX > 10 || absY > 10) {
          isHorizontalSwipe.current = absX > absY;
        }
      }

      if (preventDefault && isHorizontalSwipe.current === true) {
        e.preventDefault();
      }

      let direction: SwipeState["direction"] = null;
      if (Math.abs(deltaX) > Math.abs(deltaY)) {
        direction = deltaX > 0 ? "right" : "left";
      } else {
        direction = deltaY > 0 ? "down" : "up";
      }

      setSwipeState((prev) => ({
        ...prev,
        currentX: touch.clientX,
        currentY: touch.clientY,
        direction,
      }));
    },
    [swipeState.isSwiping, swipeState.startX, swipeState.startY, preventDefault]
  );

  const handleTouchEnd = useCallback(
    () => {
      if (!swipeState.isSwiping) return;

      const deltaX = swipeState.currentX - swipeState.startX;
      const deltaY = swipeState.currentY - swipeState.startY;
      const duration = Date.now() - touchStartTime.current;

      const velocity = Math.abs(deltaX) / duration;
      const isFastSwipe = velocity > 0.5;
      const effectiveThreshold = isFastSwipe ? threshold * 0.6 : threshold;

      if (Math.abs(deltaX) > Math.abs(deltaY)) {
        if (Math.abs(deltaX) > effectiveThreshold) {
          if (deltaX < 0 && onSwipeLeft) {
            onSwipeLeft();
          } else if (deltaX > 0 && onSwipeRight) {
            onSwipeRight();
          }
        }
      } else {
        if (Math.abs(deltaY) > effectiveThreshold) {
          if (deltaY < 0 && onSwipeUp) {
            onSwipeUp();
          } else if (deltaY > 0 && onSwipeDown) {
            onSwipeDown();
          }
        }
      }

      setSwipeState({
        startX: 0,
        startY: 0,
        currentX: 0,
        currentY: 0,
        isSwiping: false,
        direction: null,
      });
      isHorizontalSwipe.current = null;
    },
    [swipeState, threshold, onSwipeLeft, onSwipeRight, onSwipeUp, onSwipeDown]
  );

  const handleTouchCancel = useCallback(() => {
    setSwipeState({
      startX: 0,
      startY: 0,
      currentX: 0,
      currentY: 0,
      isSwiping: false,
      direction: null,
    });
    isHorizontalSwipe.current = null;
  }, []);

  const swipeProps = {
    onTouchStart: handleTouchStart,
    onTouchMove: handleTouchMove,
    onTouchEnd: handleTouchEnd,
    onTouchCancel: handleTouchCancel,
  };

  return {
    swipeState,
    swipeProps,
    isSwiping: swipeState.isSwiping,
    direction: swipeState.direction,
  };
}

export function useSwipeToClose(onClose: () => void, options: { direction?: "left" | "right" | "up" | "down"; threshold?: number } = {}) {
  const { direction = "left", threshold = 50 } = options;

  const handlers: Partial<Record<"left" | "right" | "up" | "down", () => void>> = {};
  handlers[direction] = onClose;

  return useSwipe({
    ...handlers,
    threshold,
    preventDefault: true,
  });
}
