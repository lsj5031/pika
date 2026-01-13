import { useCallback } from "react";
import { SessionList, SessionHistory, ChatInput } from "./components";
import { useAppStore } from "./store/appStore";
import { useThinkingStore } from "./store/thinkingStore";
import { useSessions } from "./hooks/useSessions";
import { useSendPrompt } from "./hooks/useSendPrompt";
import { useWebSocket } from "./hooks/useWebSocket";
import { useQueryClient } from "@tanstack/react-query";
import type { WSEvent } from "./types";

function App() {
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const { data: sessions } = useSessions();
  const sendPromptMutation = useSendPrompt();
  const queryClient = useQueryClient();
  const appendThinking = useThinkingStore((state) => state.appendThinking);
  const clearThinking = useThinkingStore((state) => state.clearThinking);

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

  // Establish WebSocket connection
  useWebSocket({ onMessage: handleWebSocketMessage });

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

  return (
    <div className="flex h-screen w-full">
      {/* Sidebar - SessionList */}
      <aside className="w-64 border-r bg-background">
        <SessionList />
      </aside>

      {/* Main content area */}
      <main className="flex-1 flex flex-col">
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
  );
}

export default App;
