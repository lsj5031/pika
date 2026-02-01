// Performance Diagnostics Script
// Paste this in your browser console when the freeze occurs

(function() {
  'use strict';

  console.log('%c🔍 PI Performance Diagnostics', 'font-size: 20px; font-weight: bold; color: #3b82f6;');
  console.log('Run each check to identify the freeze cause:\n');

  // Check 1: Memory usage
  window.checkMemory = function() {
    if (!performance.memory) {
      console.log('❌ Memory API not available');
      return;
    }
    const mem = performance.memory;
    const used = Math.round(mem.usedJSHeapSize / 1024 / 1024);
    const total = Math.round(mem.totalJSHeapSize / 1024 / 1024);
    const limit = Math.round(mem.jsHeapSizeLimit / 1024 / 1024);
    console.log('%c📊 Memory Usage:', 'font-weight: bold;');
    console.log(`  Used: ${used}MB / ${total}MB (Limit: ${limit}MB)`);
    if (used > 500) console.warn('  ⚠️  High memory usage!');
    return { used, total, limit };
  };

  // Check 2: DOM size
  window.checkDOM = function() {
    const total = document.querySelectorAll('*').length;
    const messages = document.querySelectorAll('[class*="message"], [class*="Message"]').length;
    const sessions = document.querySelectorAll('[data-session-id]').length;
    console.log('%c🌳 DOM Elements:', 'font-weight: bold;');
    console.log(`  Total: ${total}`);
    console.log(`  Messages: ${messages}`);
    console.log(`  Sessions: ${sessions}`);
    if (total > 3000) console.warn('  ⚠️  Large DOM!');
    return { total, messages, sessions };
  };

  // Check 3: React Query cache size
  window.checkQueryCache = function() {
    const cache = window.__REACT_QUERY_CACHE__;
    if (!cache) {
      console.log('❌ React Query cache not exposed. Try:');
      console.log('   const qc = queryClient || window.queryClient;');
      console.log('   console.log(qc.getQueryCache().getAll().length)');
      return;
    }
    const queries = cache.getAll();
    console.log('%c🔄 React Query Cache:', 'font-weight: bold;');
    console.log(`  Active queries: ${queries.length}`);
    queries.slice(0, 10).forEach(q => {
      console.log(`    - ${q.queryKey.join('/')}: ${q.state.data?.length || 0} items`);
    });
    return queries;
  };

  // Check 4: WebSocket status
  window.checkWebSocket = function() {
    // Look for any WebSocket connections
    const ws = window.__WS_INSTANCE__;
    if (!ws) {
      console.log('❌ WebSocket instance not found in window.__WS_INSTANCE__');
      // Try to find it elsewhere
      console.log('Checking for ws refs...');
      return;
    }
    console.log('%c🔌 WebSocket:', 'font-weight: bold;');
    console.log(`  State: ${ws.readyState} (0=CONNECTING, 1=OPEN, 2=CLOSING, 3=CLOSED)`);
    console.log(`  Buffered: ${ws.bufferedAmount} bytes`);
    return ws;
  };

  // Check 5: Long tasks
  window.checkLongTasks = function() {
    const entries = performance.getEntriesByType('longtask');
    console.log('%c⏱️ Long Tasks:', 'font-weight: bold;');
    console.log(`  Count: ${entries.length}`);
    entries.slice(-5).forEach(e => {
      console.log(`    - ${Math.round(e.duration)}ms at ${Math.round(e.startTime)}ms`);
    });
    return entries;
  };

  // Check 6: Event listeners
  window.checkEventListeners = function() {
    // This requires Chrome DevTools API, but we can estimate
    const elements = document.querySelectorAll('*');
    console.log('%c👂 Event Listeners (estimate):', 'font-weight: bold;');
    console.log(`  Total elements: ${elements.length}`);
    console.log('  Use Chrome DevTools > Performance > Event Log for details');
  };

  // Check 7: Active intervals/timeouts
  window.checkTimers = function() {
    console.log('%c⏰ Timers:', 'font-weight: bold;');
    console.log('  Cannot directly inspect. Look for:');
    console.log('  - setInterval in useEffect without cleanup');
    console.log('  - Rapid setTimeout chains');
  };

  // Check 8: Network requests
  window.checkNetwork = function() {
    const resources = performance.getEntriesByType('resource');
    const recent = resources.filter(r => r.startTime > performance.now() - 5000);
    console.log('%c🌐 Recent Network Activity:', 'font-weight: bold;');
    console.log(`  Requests in last 5s: ${recent.length}`);
    recent.slice(-5).forEach(r => {
      console.log(`    - ${r.name.split('/').pop()}: ${Math.round(r.duration)}ms`);
    });
    return recent;
  };

  // Check 9: Animation frames
  window.checkAnimationFrames = function() {
    let count = 0;
    const start = performance.now();
    const check = () => {
      count++;
      if (performance.now() - start < 1000) {
        requestAnimationFrame(check);
      } else {
        console.log('%c🎬 Frame Rate:', 'font-weight: bold;');
        console.log(`  FPS: ${count} (target: 60)`);
        if (count < 30) console.warn('  ⚠️  Low FPS!');
      }
    };
    requestAnimationFrame(check);
  };

  // Check 10: Full report
  window.fullDiagnostic = function() {
    console.log('%c\n=== FULL DIAGNOSTIC REPORT ===\n', 'font-size: 16px; font-weight: bold; color: #10b981;');
    window.checkMemory();
    window.checkDOM();
    window.checkLongTasks();
    window.checkNetwork();
    window.checkAnimationFrames();
    console.log('%c\n=== END REPORT ===', 'font-size: 16px; font-weight: bold; color: #10b981;');
  };

  // Auto-run basic checks
  console.log('%cAvailable commands:', 'font-weight: bold;');
  console.log('  checkMemory()       - Check memory usage');
  console.log('  checkDOM()          - Check DOM size');
  console.log('  checkLongTasks()    - Check for blocking tasks');
  console.log('  checkNetwork()      - Check recent network activity');
  console.log('  checkAnimationFrames() - Measure FPS');
  console.log('  fullDiagnostic()    - Run all checks');
  console.log('\n%cRun fullDiagnostic() now for complete analysis', 'color: #f59e0b;');

  // Expose to window
  window.PI_DIAGNOSTICS = {
    checkMemory: window.checkMemory,
    checkDOM: window.checkDOM,
    checkLongTasks: window.checkLongTasks,
    checkNetwork: window.checkNetwork,
    checkAnimationFrames: window.checkAnimationFrames,
    fullDiagnostic: window.fullDiagnostic,
  };
})();
