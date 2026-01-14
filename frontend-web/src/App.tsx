import { useCallback, useState } from "react";
import { SessionList, SessionHistory, ChatInput, AppHeader } from "./components";
import { useAppStore } from "./store/appStore";
import { useThinkingStore } from "./store/thinkingStore";
import { useSessions } from "./hooks/useSessions";
import { useSendPrompt } from "./hooks/useSendPrompt";
import { useStopSession } from "./hooks/useStopSession";
import { useWebSocket } from "./hooks/useWebSocket";
import { useQueryClient } from "@tanstack/react-query";
import { Sheet, SheetContent } from "./components/ui/sheet";
import type { WSEvent } from "./types";

function App() {
  const currentSessionId = useAppStore((state) => state.currentSessionId);

  const { data: sessions } = useSessions();
  const sendPromptMutation = useSendPrompt();
  const stopSessionMutation = useStopSession();
  const queryClient = useQueryClient();
  const appendThinking = useThinkingStore((state) => state.appendThinking);
  const clearThinking = useThinkingStore((state) => state.clearThinking);

  // Mobile drawer state
  const [mobileDrawerOpen, setMobileDrawerOpen] = useState(false);

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

  // Establish WebSocket connection and get connection status
  const { connectionStatus } = useWebSocket({ onMessage: handleWebSocketMessage });

  // Handle sending messages
  const handleSendMessage = useCallback(
    async (content: string) => {
      if (!currentSessionId) return;

      try {
        await sendPromptMutation.mutateAsync({
          sessionId: currentSessionId,
          prompt: content,
        });
      } catch (error) {
        console.error("Failed to send prompt:", error);
        // TODO: Show toast error in US-014
      }
    },
    [currentSessionId, sendPromptMutation]
  );

  // Handle stopping the current session
  const handleStopSession = useCallback(async () => {
    if (!currentSessionId) return;

    try {
      await stopSessionMutation.mutateAsync(currentSessionId);
    } catch (error) {
      console.error("Failed to stop session:", error);
      // TODO: Show toast error in US-014
    }
  }, [currentSessionId, stopSessionMutation]);

  // Handle mobile menu toggle
  const handleMenuToggle = useCallback(() => {
    setMobileDrawerOpen((prev) => !prev);
  }, []);

  return (
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
        <aside className="hidden w-64 border-r bg-background md:block">
          <SessionList />
        </aside>

        {/* Mobile Drawer Sidebar */}
        <Sheet open={mobileDrawerOpen} onOpenChange={setMobileDrawerOpen}>
          <SheetContent side="left" className="w-64 p-0">
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
  );
}

export default App;
