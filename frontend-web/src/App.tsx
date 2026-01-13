import { useCallback } from "react";
import { SessionList, SessionHistory, ChatInput } from "./components";
import { useAppStore } from "./store/appStore";
import { useSessions } from "./hooks/useSessions";
import { useSendPrompt } from "./hooks/useSendPrompt";

function App() {
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const { data: sessions } = useSessions();
  const sendPromptMutation = useSendPrompt();

  // Find current session and check if active
  const currentSession = sessions?.find((s) => s.id === currentSessionId);
  const isSessionActive = currentSession?.is_active ?? false;

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
