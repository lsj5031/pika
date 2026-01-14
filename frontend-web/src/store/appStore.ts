import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AppState {
  // State
  currentSessionId: string | null;
  sidebarCollapsed: boolean;
  activeSessionIds: Set<string>; // NEW: Track active sessions
  thinkingSessionIds: Set<string>; // NEW: Track sessions with thinking in progress
  unreadSessions: Set<string>; // NEW: Track sessions with unread messages
  lastSeenMessageCounts: Record<string, number>; // NEW: Track last seen message count per session

  // Actions
  setCurrentSession: (sessionId: string | null) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActiveSession: (sessionId: string, isActive: boolean) => void; // NEW
  setThinkingSession: (sessionId: string, isThinking: boolean) => void; // NEW
  markSessionAsRead: (sessionId: string, messageCount: number) => void; // NEW
  incrementUnreadCount: (sessionId: string) => void; // NEW
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Initial state
      currentSessionId: null,
      sidebarCollapsed: false,
      activeSessionIds: new Set<string>(),
      thinkingSessionIds: new Set<string>(),
      unreadSessions: new Set<string>(),
      lastSeenMessageCounts: {},

      // Actions
      setCurrentSession: (sessionId) => set({ currentSessionId: sessionId }),

      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

      setActiveSession: (sessionId, isActive) =>
        set((state) => {
          const newSet = new Set(state.activeSessionIds);
          if (isActive) {
            newSet.add(sessionId);
          } else {
            newSet.delete(sessionId);
          }
          return { activeSessionIds: newSet };
        }),

      setThinkingSession: (sessionId, isThinking) =>
        set((state) => {
          const newSet = new Set(state.thinkingSessionIds);
          if (isThinking) {
            newSet.add(sessionId);
          } else {
            newSet.delete(sessionId);
          }
          return { thinkingSessionIds: newSet };
        }),

      markSessionAsRead: (sessionId, messageCount) =>
        set((state) => {
          const newUnread = new Set(state.unreadSessions);
          newUnread.delete(sessionId);
          return {
            unreadSessions: newUnread,
            lastSeenMessageCounts: {
              ...state.lastSeenMessageCounts,
              [sessionId]: messageCount,
            },
          };
        }),

      incrementUnreadCount: (sessionId) =>
        set((state) => {
          // Only mark as unread if it's not the current session
          if (state.currentSessionId === sessionId) {
            return state;
          }
          const newUnread = new Set(state.unreadSessions);
          newUnread.add(sessionId);
          return { unreadSessions: newUnread };
        }),
    }),
    {
      name: "pika-storage",
      // Don't persist Sets (they'll be re-synced from WebSocket)
      partialize: (state) => ({
        currentSessionId: state.currentSessionId,
        sidebarCollapsed: state.sidebarCollapsed,
        lastSeenMessageCounts: state.lastSeenMessageCounts,
      }),
    }
  )
);
