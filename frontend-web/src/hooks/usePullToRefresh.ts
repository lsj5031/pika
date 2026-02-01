import { useState, useCallback, useRef } from "react";

interface PullToRefreshState {
  pullDistance: number;
  isPulling: boolean;
  isRefreshing: boolean;
  pullProgress: number;
}

interface UsePullToRefreshOptions {
  onRefresh: () => Promise<void>;
  threshold?: number;
  maxPullDistance?: number;
  resistance?: number;
}

export function usePullToRefresh(options: UsePullToRefreshOptions) {
  const {
    onRefresh,
    threshold = 80,
    maxPullDistance = 150,
    resistance = 2.5,
  } = options;

  const [state, setState] = useState<PullToRefreshState>({
    pullDistance: 0,
    isPulling: false,
    isRefreshing: false,
    pullProgress: 0,
  });

  const startY = useRef<number>(0);
  const currentY = useRef<number>(0);
  const scrollContainerRef = useRef<HTMLElement | null>(null);
  const isAtTop = useRef<boolean>(true);

  const checkIsAtTop = useCallback(() => {
    if (!scrollContainerRef.current) return true;
    return scrollContainerRef.current.scrollTop <= 0;
  }, []);

  const setScrollContainer = useCallback((element: HTMLElement | null) => {
    scrollContainerRef.current = element;
  }, []);

  const handleTouchStart = useCallback(
    (e: React.TouchEvent) => {
      isAtTop.current = checkIsAtTop();
      if (!isAtTop.current || state.isRefreshing) return;

      startY.current = e.touches[0].clientY;
      currentY.current = startY.current;
    },
    [checkIsAtTop, state.isRefreshing]
  );

  const handleTouchMove = useCallback(
    (e: React.TouchEvent) => {
      if (!isAtTop.current || state.isRefreshing) return;

      currentY.current = e.touches[0].clientY;
      const deltaY = currentY.current - startY.current;

      if (deltaY > 0) {
        e.preventDefault();

        const resistedDistance = Math.min(deltaY / resistance, maxPullDistance);
        const progress = Math.min(resistedDistance / threshold, 1);

        setState({
          pullDistance: resistedDistance,
          isPulling: true,
          isRefreshing: false,
          pullProgress: progress,
        });
      }
    },
    [maxPullDistance, resistance, threshold, state.isRefreshing]
  );

  const handleTouchEnd = useCallback(async () => {
    if (!state.isPulling || state.isRefreshing) return;

    if (state.pullProgress >= 1) {
      setState((prev) => ({
        ...prev,
        isRefreshing: true,
        pullDistance: threshold,
        pullProgress: 1,
      }));

      try {
        await onRefresh();
      } finally {
        setState({
          pullDistance: 0,
          isPulling: false,
          isRefreshing: false,
          pullProgress: 0,
        });
      }
    } else {
      setState({
        pullDistance: 0,
        isPulling: false,
        isRefreshing: false,
        pullProgress: 0,
      });
    }
  }, [state.isPulling, state.isRefreshing, state.pullProgress, threshold, onRefresh]);

  const handleTouchCancel = useCallback(() => {
    if (state.isRefreshing) return;

    setState({
      pullDistance: 0,
      isPulling: false,
      isRefreshing: false,
      pullProgress: 0,
    });
  }, [state.isRefreshing]);

  const pullToRefreshProps = {
    onTouchStart: handleTouchStart,
    onTouchMove: handleTouchMove,
    onTouchEnd: handleTouchEnd,
    onTouchCancel: handleTouchCancel,
    style: {
      transform: state.isPulling || state.isRefreshing
        ? `translateY(${state.pullDistance}px)`
        : undefined,
      transition: !state.isPulling && !state.isRefreshing
        ? "transform 0.3s ease-out"
        : undefined,
    } as React.CSSProperties,
  };

  return {
    ...state,
    pullToRefreshProps,
    setScrollContainer,
  };
}
