import { useCallback, useState, useRef, useEffect, lazy, Suspense } from "react";
import { ChatInput, AppHeader, AuthPrompt } from "./components";
import { NewSessionDialog } from "./components/NewSessionDialog";
import { CommandPalette } from "./components/CommandPalette";
import { Loader2, Plus } from "lucide-react";
import { useAppStore } from "./store/appStore";
import { useThinkingStore } from "./store/thinkingStore";
import { useSessions } from "./hooks/useSessions";
import { useSendPrompt } from "./hooks/useSendPrompt";
import { useStopSession } from "./hooks/useStopSession";
import { useWebSocket } from "./hooks/useWebSocket";
import { useCommandPalette, useSessionSwitchingShortcuts } from "./hooks/useCommandPalette";
import { usePerformanceMonitor } from "./hooks/usePerformanceMonitor";
import { useQueryClient, type InfiniteData } from "@tanstack/react-query";
import { toast } from "sonner";
import { AUTH_ERROR_EVENT } from "./lib/api";
import { clearAuthState, markAuthenticated } from "./lib/auth";
import { config } from "./config/env";
import type { WSEvent, Message, Session } from "./types";

// Lazy load heavy components
const SessionHistory = lazy(() => import("./components/SessionHistory").then(module => ({ default: module.SessionHistory })));

function App() {
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);
  const markSessionAsRead = useAppStore((state) => state.markSessionAsRead);
  const clearInvalidSession = useAppStore((state) => state.clearInvalidSession);

  // Auth state
  const needsAuth = useAppStore((state) => state.needsAuth);
  const setNeedsAuth = useAppStore((state) => state.setNeedsAuth);
  const [authKey, setAuthKey] = useState(0); // Used to force re-render after auth

  const chatInputWrapperRef = useRef<HTMLDivElement>(null);
  const [footerHeight, setFooterHeight] = useState(140);

  useEffect(() => {
    const element = chatInputWrapperRef.current;
    if (!element) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setFooterHeight(entry.contentRect.height);
      }
    });

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Performance monitoring - logs warnings to console
  usePerformanceMonitor({
    onLongTask: (duration) => {
      console.warn(`[Performance] Long task detected: ${Math.round(duration)}ms`);
    },
    onFrameDrop: (fps) => {
      console.warn(`[Performance] Frame drop: ${fps} FPS`);
    },
    onMemoryWarning: (used, growth) => {
      console.warn(`[Performance] Memory growth: +${Math.round(growth)}MB (total: ${Math.round(used)}MB)`);
    },
    enableLogging: import.meta.env.DEV,
    longTaskThreshold: 50,
    frameDropThreshold: 30,
    memoryGrowthThreshold: 50,
  });

  // Command palette
  const {
    isOpen: commandPaletteOpen,
    open: openCommandPalette,
    close: closeCommandPalette,
  } = useCommandPalette();

  const { data: sessions } = useSessions(!needsAuth, { resolveNames: commandPaletteOpen });
  const sessionsRef = useRef(sessions);
  useEffect(() => { sessionsRef.current = sessions; }, [sessions]);
  const sendPromptMutation = useSendPrompt();
  const stopSessionMutation = useStopSession();
  const queryClient = useQueryClient();
  const appendThinking = useThinkingStore((state) => state.appendThinking);
  const clearThinking = useThinkingStore((state) => state.clearThinking);

  // Session switching keyboard shortcuts
  useSessionSwitchingShortcuts(
    sessions ?? [],
    currentSessionId,
    useCallback((sessionId: string) => {
      setCurrentSession(sessionId);
    }, [setCurrentSession])
  );


  // Listen for auth error events
  useEffect(() => {
    const handleAuthError = () => {
      clearAuthState();
      setNeedsAuth(true);
    };

    window.addEventListener(AUTH_ERROR_EVENT, handleAuthError);
    return () => {
      window.removeEventListener(AUTH_ERROR_EVENT, handleAuthError);
    };
  }, [setNeedsAuth]);

  // Listen for session not found events (404s)
  useEffect(() => {
    const handleSessionNotFound = ((event: CustomEvent<{ sessionId: string }>) => {
      const { sessionId } = event.detail;
      console.warn(`Session ${sessionId} not found, clearing from state`);
      clearInvalidSession(sessionId);
    }) as EventListener;

    window.addEventListener("session-not-found", handleSessionNotFound);
    return () => {
      window.removeEventListener("session-not-found", handleSessionNotFound);
    };
  }, [clearInvalidSession]);

  // Handle successful authentication
  const handleAuthenticated = useCallback(() => {
    markAuthenticated();
    setNeedsAuth(false);
    setAuthKey((k) => k + 1); // Force refresh
    // Invalidate all queries to refetch after authentication
    queryClient.invalidateQueries();
  }, [queryClient, setNeedsAuth]);

  // Check auth status on load so valid session cookies skip the login prompt.
  useEffect(() => {
    const checkAuthStatus = async () => {
      try {
        const response = await fetch(`${config.API_URL}/api/auth/status`, {
          credentials: "include",
        });
        if (!response.ok) return;

        const data = (await response.json()) as {
          enabled: boolean;
          authenticated: boolean;
        };

        if (!data.enabled) {
          setNeedsAuth(false);
          return;
        }

        if (data.authenticated) {
          markAuthenticated();
          setNeedsAuth(false);
        } else {
          clearAuthState();
          setNeedsAuth(true);
        }
      } catch {
        // Ignore network errors; auth prompt will show if needed.
      }
    };

    checkAuthStatus();
  }, [setNeedsAuth]);

  // Find current session and check if active
  const currentSession = sessions?.find((s) => s.id === currentSessionId);
  const isSessionActive = currentSession?.is_active ?? false;

  // Get recent and favorite sessions for command palette
  const recentSessionIds = useAppStore((state) => state.recentSessionIds);
  const favoriteSessionIds = useAppStore((state) => state.favoriteSessionIds);
  const activeSessionIds = useAppStore((state) => state.activeSessionIds);
  const thinkingSessionIds = useAppStore((state) => state.thinkingSessionIds);
  const unreadSessions = useAppStore((state) => state.unreadSessions);

  // Mark current session as read when selected
  useEffect(() => {
    if (currentSessionId) {
      // Mark current session as read when selected
      const session = sessions?.find((s) => s.id === currentSessionId);
      if (session) {
        // Mark as read without forcing a full history fetch
        markSessionAsRead(currentSessionId, 0);
      }
    }
  }, [currentSessionId, sessions, queryClient, markSessionAsRead]);

  // WebSocket event handler
  const handleWebSocketMessage = useCallback(
    (event: WSEvent) => {
      switch (event.type) {
        case "SessionStarted": {
          // Update session active status
          useAppStore.getState().setActiveSession(event.data.session_id, true);
          const sessions = queryClient.getQueryData<Session[]>(["sessions"]);
          if (sessions?.some((session) => session.id === event.data.session_id)) {
            queryClient.setQueryData<Session[]>(["sessions"], (current) =>
              current?.map((session) =>
                session.id === event.data.session_id
                  ? { ...session, is_active: true }
                  : session
              ) ?? current
            );
          } else {
            queryClient.invalidateQueries({ queryKey: ["sessions"] });
          }
          break;
        }
        case "SessionStopped": {
          // Clear thinking state and active status
          clearThinking(event.data.session_id);
          useAppStore.getState().setActiveSession(event.data.session_id, false);
          const sessions = queryClient.getQueryData<Session[]>(["sessions"]);
          if (sessions?.some((session) => session.id === event.data.session_id)) {
            queryClient.setQueryData<Session[]>(["sessions"], (current) =>
              current?.map((session) =>
                session.id === event.data.session_id
                  ? { ...session, is_active: false }
                  : session
              ) ?? current
            );
          } else {
            queryClient.invalidateQueries({ queryKey: ["sessions"] });
          }
          break;
        }
        case "ThinkingDelta": {
          // Set thinking state
          useAppStore.getState().setThinkingSession(event.data.session_id, true);
          appendThinking(event.data.session_id, event.data.content);
          break;
        }
        case "MessageAdded": {
          // Mark as unread if not current session
          if (currentSessionId !== event.data.session_id) {
            useAppStore.getState().incrementUnreadCount(event.data.session_id);

            if (document.hidden) {
              const sessionName = sessionsRef.current?.find(s => s.id === event.data.session_id)?.name;
              const displayName = sessionName && !/^\d{4}-\d{2}-\d{2}T/.test(sessionName)
                ? sessionName
                : `Session ${event.data.session_id.substring(0, 8)}`;

              if (Notification.permission === "granted") {
                new Notification(`Pika — ${displayName}`, {
                  body: event.data.content.substring(0, 100),
                  tag: `pika-${event.data.session_id}`,
                  icon: "/logo.png",
                });
              } else if (Notification.permission === "default") {
                Notification.requestPermission();
              }
            }
          }
          // Clear thinking state when message is added
          useAppStore.getState().setThinkingSession(event.data.session_id, false);
          clearThinking(event.data.session_id);

          // Append message directly to cache for real-time updates
          const newMessage: Message = {
            role: event.data.role as "user" | "assistant",
            content: event.data.content,
            timestamp: event.data.timestamp,
            images: event.data.images,
          };
          queryClient.setQueryData<InfiniteData<Message[]>>(
            ["sessions", event.data.session_id, "messages", "paged"],
            (old) => {
              if (!old) {
                return { pages: [[newMessage]], pageParams: [undefined] };
              }
              const pages = [...old.pages];
              pages[0] = [...pages[0], newMessage];
              return { ...old, pages };
            }
          );
          break;
        }
        case "Error": {
          toast.error("Agent Error", {
            description: event.data.message,
          });
          break;
        }
      }
    },
    [queryClient, appendThinking, clearThinking, currentSessionId]
  );

  // WebSocket error handler with debounce to prevent toast spam
  const wsErrorToastTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const handleWebSocketError = useCallback(() => {
    // Debounce error toasts - only show one every 5 seconds
    if (wsErrorToastTimeoutRef.current) return;

    toast.error("Connection lost", {
      description: "WebSocket connection lost. Reconnecting...",
    });

    wsErrorToastTimeoutRef.current = setTimeout(() => {
      wsErrorToastTimeoutRef.current = null;
    }, 5000);
  }, []);

  // Establish WebSocket connection and get connection status
  // Only connect if authenticated
  const { connectionStatus, reconnect } = useWebSocket({
    onMessage: handleWebSocketMessage,
    onError: handleWebSocketError,
    enabled: !needsAuth,
  });

  // Reconnect WebSocket when auth changes
  useEffect(() => {
    if (!needsAuth && authKey > 0) {
      // Small delay to ensure credentials are stored
      setTimeout(() => {
        reconnect();
      }, 100);
    }
  }, [needsAuth, authKey, reconnect]);

  // Handle sending messages
  const handleSendMessage = useCallback(
    (content: string, images?: import("./types/api").ImageUploadRequest[]) => {
      if (!currentSessionId) return;

      // Optimistic: immediately show user message in chat (including images)
      const optimisticImages = images?.map((img, i) => ({
        id: `optimistic-${i}`,
        filename: img.filename,
        content_type: img.content_type,
        size: Math.round((img.data.length * 3) / 4),
        url: `data:${img.content_type};base64,${img.data}`,
      }));
      const optimisticMessage: Message = {
        role: "user",
        content,
        timestamp: new Date().toISOString(),
        images: optimisticImages,
        _optimistic: true,
      };
      queryClient.setQueryData<InfiniteData<Message[]>>(
        ["sessions", currentSessionId, "messages", "paged"],
        (old) => {
          if (!old) {
            return { pages: [[optimisticMessage]], pageParams: [undefined] };
          }
          const pages = [...old.pages];
          pages[0] = [...pages[0], optimisticMessage];
          return { ...old, pages };
        }
      );

      // Send prompt, rolling back optimistic message on error
      sendPromptMutation.mutate(
        { sessionId: currentSessionId, prompt: content, images },
        {
          onError: () => {
            queryClient.setQueryData<InfiniteData<Message[]>>(
              ["sessions", currentSessionId, "messages", "paged"],
              (old) => {
                if (!old) return old;
                const pages = old.pages.map((page) =>
                  page.filter(
                    (msg) =>
                      !(
                        msg._optimistic &&
                        msg.content === content &&
                        msg.timestamp === optimisticMessage.timestamp
                      )
                  )
                );
                return { ...old, pages };
              }
            );
          },
        }
      );
    },
    [currentSessionId, sendPromptMutation, queryClient]
  );

  // Handle stopping the current session
  const handleStopSession = useCallback(() => {
    if (!currentSessionId) return;

    stopSessionMutation.mutate(currentSessionId);
  }, [currentSessionId, stopSessionMutation]);

  return (
    <>
      {/* Auth prompt modal */}
      <AuthPrompt open={needsAuth} onAuthenticated={handleAuthenticated} />

      <div className="flex h-dvh min-h-screen w-full flex-col overflow-hidden">
        {/* Header */}
        <AppHeader
          connectionStatus={connectionStatus}
          isSessionActive={isSessionActive}
          onStopSession={isSessionActive ? handleStopSession : undefined}
          onOpenCommandPalette={openCommandPalette}
        />

        {/* Main content area */}
        <main className="flex-1 flex flex-col overflow-hidden min-w-0">
          <div className="flex-1 overflow-hidden">
            <Suspense
              fallback={
                <div className="flex items-center justify-center h-full text-muted-foreground">
                  <Loader2 className="h-6 w-6 animate-spin" />
                </div>
              }
            >
              <SessionHistory sessionId={currentSessionId} />
            </Suspense>
          </div>
          <div ref={chatInputWrapperRef}>
            <ChatInput sessionId={currentSessionId} onSendMessage={handleSendMessage} />
          </div>
          <NewSessionDialog
            trigger={
              <button 
                className="hidden md:flex fixed h-14 w-14 rounded-full border-2 border-border shadow-lg hover:bg-accent hover:text-accent-foreground items-center justify-center z-50 active:scale-95 touch-manipulation transition-all duration-200 bg-card text-card-foreground"
                style={{  
                  bottom: `calc(env(safe-area-inset-bottom) + ${footerHeight}px + 1rem)`,
                  right: `calc(env(safe-area-inset-right) + 1rem)`,
                }}
              >
                <Plus className="h-6 w-6" />
                <span className="sr-only">New Session</span>
              </button>
            }
          />
        </main>

        {/* Command Palette for quick session switching */}
        <CommandPalette
          key={commandPaletteOpen ? "open" : "closed"}
          isOpen={commandPaletteOpen}
          onClose={closeCommandPalette}
          sessions={sessions ?? []}
          recentSessionIds={recentSessionIds}
          favoriteSessionIds={favoriteSessionIds}
          currentSessionId={currentSessionId}
          activeSessionIds={activeSessionIds}
          thinkingSessionIds={thinkingSessionIds}
          unreadSessions={unreadSessions}
          onSelectSession={setCurrentSession}
        />
      </div>
    </>
  );
}

export default App;
