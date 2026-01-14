import { useCallback, useState, useRef, useEffect } from "react";
import { SessionList, SessionHistory, ChatInput, AppHeader, AuthPrompt } from "./components";
import { useAppStore } from "./store/appStore";
import { useThinkingStore } from "./store/thinkingStore";
import { useSessions } from "./hooks/useSessions";
import { useSendPrompt } from "./hooks/useSendPrompt";
import { useStopSession } from "./hooks/useStopSession";
import { useWebSocket } from "./hooks/useWebSocket";
import { useQueryClient } from "@tanstack/react-query";
import { Sheet, SheetContent } from "./components/ui/sheet";
import { toast } from "sonner";
import { hasCredentials } from "./lib/auth";
import { AUTH_ERROR_EVENT } from "./lib/api";
import type { WSEvent } from "./types";

function App() {
  const currentSessionId = useAppStore((state) => state.currentSessionId);

  // Auth state
  const [needsAuth, setNeedsAuth] = useState(() => !hasCredentials());
  const [authKey, setAuthKey] = useState(0); // Used to force re-render after auth

  const { data: sessions } = useSessions();
  const sendPromptMutation = useSendPrompt();
  const stopSessionMutation = useStopSession();
  const queryClient = useQueryClient();
  const appendThinking = useThinkingStore((state) => state.appendThinking);
  const clearThinking = useThinkingStore((state) => state.clearThinking);

  // Mobile drawer state
  const [mobileDrawerOpen, setMobileDrawerOpen] = useState(false);

  // Listen for auth error events
  useEffect(() => {
    const handleAuthError = () => {
      setNeedsAuth(true);
    };

    window.addEventListener(AUTH_ERROR_EVENT, handleAuthError);
    return () => {
      window.removeEventListener(AUTH_ERROR_EVENT, handleAuthError);
    };
  }, []);

  // Handle successful authentication
  const handleAuthenticated = useCallback(() => {
    setNeedsAuth(false);
    setAuthKey((k) => k + 1); // Force refresh
    // Invalidate all queries to refetch with new credentials
    queryClient.invalidateQueries();
  }, [queryClient]);

  // Find current session and check if active
  const currentSession = sessions?.find((s) => s.id === currentSessionId);
  const isSessionActive = currentSession?.is_active ?? false;

  // WebSocket event handler
  const handleWebSocketMessage = useCallback(
    (event: WSEvent) => {
      switch (event.type) {
        case "SessionStarted": {
          // Update session active status
          queryClient.invalidateQueries({ queryKey: ["sessions"] });
          break;
        }
        case "SessionStopped": {
          // Clear thinking state for stopped session
          clearThinking(event.data.session_id);
          // Update session active status
          queryClient.invalidateQueries({ queryKey: ["sessions"] });
          break;
        }
        case "ThinkingDelta": {
          // Append thinking content
          appendThinking(event.data.session_id, event.data.content);
          break;
        }
        case "MessageAdded": {
          // Clear thinking state when message is added (thinking complete)
          clearThinking(event.data.session_id);
          // Invalidate messages query to fetch new messages
          queryClient.invalidateQueries({
            queryKey: ["sessions", event.data.session_id, "messages"],
          });
          break;
        }
      }
    },
    [queryClient, appendThinking, clearThinking]
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
    (content: string) => {
      if (!currentSessionId) return;

      sendPromptMutation.mutate({
        sessionId: currentSessionId,
        prompt: content,
      });
    },
    [currentSessionId, sendPromptMutation]
  );

  // Handle stopping the current session
  const handleStopSession = useCallback(() => {
    if (!currentSessionId) return;

    stopSessionMutation.mutate(currentSessionId);
  }, [currentSessionId, stopSessionMutation]);

  // Handle mobile menu toggle
  const handleMenuToggle = useCallback(() => {
    setMobileDrawerOpen((prev) => !prev);
  }, []);

  return (
    <>
      {/* Auth prompt modal */}
      <AuthPrompt open={needsAuth} onAuthenticated={handleAuthenticated} />

      <div className="flex h-screen w-full flex-col">
        {/* Header */}
        <AppHeader
          connectionStatus={connectionStatus}
          isSessionActive={isSessionActive}
          onMenuToggle={handleMenuToggle}
          onStopSession={isSessionActive ? handleStopSession : undefined}
        />

        {/* Main layout: Sidebar + Content */}
        <div className="flex flex-1 overflow-hidden">
          {/* Desktop Sidebar */}
          <aside className="hidden w-64 border-r-2 border-dashed border-primary bg-background md:block">
            <SessionList />
          </aside>

          {/* Mobile Drawer Sidebar */}
          <Sheet open={mobileDrawerOpen} onOpenChange={setMobileDrawerOpen}>
            <SheetContent side="left" className="w-[280px] p-0 sm:w-64" id="mobile-drawer-content">
              <SessionList />
            </SheetContent>
          </Sheet>

          {/* Main content area */}
          <main className="flex-1 flex flex-col overflow-hidden">
            <div className="flex-1 overflow-hidden">
              <SessionHistory sessionId={currentSessionId} />
            </div>
            <ChatInput
              sessionId={currentSessionId}
              isSessionActive={isSessionActive}
              onSendMessage={handleSendMessage}
            />
          </main>
        </div>
      </div>
    </>
  );
}

export default App;
