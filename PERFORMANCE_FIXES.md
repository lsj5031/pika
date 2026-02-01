# Performance Optimizations

This document tracks performance optimizations applied to Pika.

## Summary

All known performance bottlenecks have been addressed. The application should handle large session lists and long conversation histories without UI freezing.

---

## Backend Optimizations

### 1. Async Session File Reading ✅
**Files**: `src/sessions.rs`

- Converted `scan_sessions()` and `scan_project_sessions()` from blocking to async using `tokio::fs`
- Prevents blocking the async runtime during session discovery

### 2. Optimized Last Message Timestamp Extraction ✅
**File**: `src/sessions.rs` → `get_last_message_timestamp()`

**Before**: Read entire session file line-by-line to find the last timestamp
**After**: Seek to last 4KB of file and parse only that portion

- Reduces I/O from O(file_size) to O(1) constant ~4KB
- Critical for sessions with thousands of messages

### 3. Efficient Process HashMap Access ✅
**File**: `src/pi.rs` → `send_prompt()`

**Before**: Remove process from HashMap, use it, reinsert
**After**: Use `get_mut()` directly

- Eliminates unnecessary HashMap operations

---

## Frontend Optimizations

### 4. Session Name Resolution Batching ✅
**File**: `frontend-web/src/hooks/useSessions.ts`

**Before**: 10 staggered requests with 50ms delays (500ms waterfall)
**After**: 5 parallel requests via `Promise.all`

- Reduced from 10 to 5 sessions for name resolution
- Eliminated request waterfall entirely
- Names cached in component state

### 5. Delta Batching for WebSocket Updates ✅
**File**: `frontend-web/src/stores/thinkingStore.ts`

- Buffers ThinkingDelta events
- Flushes every 50ms instead of immediate updates
- Reduces state updates from 50+/sec to ~20/sec

### 6. Throttled Auto-Scroll ✅
**File**: `frontend-web/src/components/SessionHistory.tsx`

- 100ms throttle on scroll operations
- Prevents layout thrashing during rapid message updates

### 7. Memoized Message Rendering ✅
**File**: `frontend-web/src/components/MessageBubble.tsx`

- Wrapped with `React.memo()`
- Messages only re-render when content changes

### 8. Limited Initial Message Loading ✅
**File**: `frontend-web/src/hooks/useSessionHistory.ts`

- Loads max 50 messages initially (`MAX_INITIAL_MESSAGES = 50`)
- Prevents UI freeze from rendering 500+ messages

### 9. Reduced API Polling Frequency ✅
**File**: `frontend-web/src/hooks/useSessions.ts`

- `staleTime: 30000` (30 seconds)
- `refetchInterval: 60000` (1 minute polling)
- Reduces unnecessary API calls

---

## Performance Monitoring

**File**: `frontend-web/src/hooks/usePerformanceMonitor.ts`

Development-only monitoring that detects:
- Long tasks (>50ms)
- Frame rate drops
- Memory growth

---

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| Session scan I/O | O(n × file_size) | O(n × 4KB) |
| Name resolution requests | 10 staggered | 5 parallel |
| Name resolution latency | ~500ms waterfall | ~1 round-trip |
| WebSocket state updates | 50+/sec | ~20/sec |
| Initial messages loaded | All | Max 50 |

---

## Future Considerations

If performance issues arise with very large datasets:

1. **Virtual scrolling** for session list and message history (react-window)
2. **Pagination** for `/api/sessions` endpoint
3. **Session file indexing** to avoid scanning directories
4. **Message streaming** for incremental loading
