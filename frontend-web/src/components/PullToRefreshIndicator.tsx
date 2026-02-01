interface PullToRefreshIndicatorProps {
  pullDistance: number;
  isPulling: boolean;
  isRefreshing: boolean;
  pullProgress: number;
}

export function PullToRefreshIndicator({
  isRefreshing,
  pullProgress,
}: PullToRefreshIndicatorProps) {
  if (!isRefreshing && pullProgress === 0) return null;

  return (
    <div
      className="absolute top-0 left-0 right-0 flex items-center justify-center h-16 -mt-16 z-10"
      style={{
        opacity: Math.min(pullProgress * 1.5, 1),
        transform: `translateY(${pullProgress * 20}px)`,
      }}
    >
      <div className="flex items-center gap-2 text-muted-foreground">
        {isRefreshing ? (
          <>
            <div className="h-5 w-5 border-2 border-primary border-t-transparent rounded-full animate-spin" />
            <span className="text-sm font-medium">Refreshing...</span>
          </>
        ) : (
          <>
            <svg
              className="h-5 w-5 transition-transform"
              style={{
                transform: `rotate(${pullProgress * 180}deg)`,
              }}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 14l-7 7m0 0l-7-7m7 7V3"
              />
            </svg>
            <span className="text-sm font-medium">
              {pullProgress >= 1 ? "Release to refresh" : "Pull to refresh"}
            </span>
          </>
        )}
      </div>
    </div>
  );
}
