import { create } from "zustand";
import { persist } from "zustand/middleware";
import { hasAuthState } from "../lib/auth";

interface AppState {
  // State
  currentSessionId: string | null;
  needsAuth: boolean;
  activeSessionIds: Set<string>; // Track active sessions
  thinkingSessionIds: Set<string>; // Track sessions with thinking in progress
  unreadSessions: Set<string>; // Track sessions with unread messages
  lastSeenMessageCounts: Record<string, number>; // Track last seen message count per session
  lastProjectId: string | null; // Track most recently used project
  recentSessionIds: string[]; // NEW: Recently accessed sessions (max 5)
  favoriteSessionIds: string[]; // NEW: Favorite/pinned sessions
  sessionThinkingLevels: Record<string, string>; // Per-session thinking level
  sessionModels: Record<string, { id: string; name: string; provider: string }>; // Per-session model

  // Actions
  setCurrentSession: (sessionId: string | null) => void;
  setNeedsAuth: (needsAuth: boolean) => void;
  setActiveSession: (sessionId: string, isActive: boolean) => void;
  setActiveSessionIds: (ids: Set<string>) => void;
  setThinkingSession: (sessionId: string, isThinking: boolean) => void;
  markSessionAsRead: (sessionId: string, messageCount: number) => void;
  incrementUnreadCount: (sessionId: string) => void;
  clearInvalidSession: (sessionId: string) => void; // Clear session that doesn't exist
  setLastProject: (projectId: string) => void; // Track last used project
  addRecentSession: (sessionId: string) => void; // NEW: Add to recent sessions
  removeRecentSession: (sessionId: string) => void; // NEW: Remove from recent sessions
  toggleFavoriteSession: (sessionId: string) => void; // NEW: Toggle favorite status
  isFavoriteSession: (sessionId: string) => boolean; // NEW: Check if favorite
  setSessionThinkingLevel: (sessionId: string, level: string) => void;
  getSessionThinkingLevel: (sessionId: string) => string | null;
  setSessionModel: (sessionId: string, model: { id: string; name: string; provider: string }) => void;
  getSessionModel: (sessionId: string) => { id: string; name: string; provider: string } | null;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // Initial state
      currentSessionId: null,
      needsAuth: !hasAuthState(),
      activeSessionIds: new Set<string>(),
      thinkingSessionIds: new Set<string>(),
      unreadSessions: new Set<string>(),
      lastSeenMessageCounts: {},
      lastProjectId: null,
      recentSessionIds: [],
      favoriteSessionIds: [],
      sessionThinkingLevels: {},
      sessionModels: {},

      // Actions
      setCurrentSession: (sessionId) => {
        // Add to recent sessions when switching
        if (sessionId) {
          get().addRecentSession(sessionId);
        }
        set({ currentSessionId: sessionId });
      },

      setNeedsAuth: (needsAuth) => set({ needsAuth }),

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

      setActiveSessionIds: (ids) => set({ activeSessionIds: ids }),

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

      clearInvalidSession: (sessionId) =>
        set((state) => {
          // Clear the session if it's the current one
          if (state.currentSessionId === sessionId) {
            return { currentSessionId: null };
          }
          return state;
        }),

      setLastProject: (projectId) => set({ lastProjectId: projectId }),

      addRecentSession: (sessionId) =>
        set((state) => {
          // Remove if already exists, add to front, keep max 5
          const filtered = state.recentSessionIds.filter((id) => id !== sessionId);
          const newRecent = [sessionId, ...filtered].slice(0, 5);
          return { recentSessionIds: newRecent };
        }),

      removeRecentSession: (sessionId) =>
        set((state) => ({
          recentSessionIds: state.recentSessionIds.filter((id) => id !== sessionId),
        })),

      toggleFavoriteSession: (sessionId) =>
        set((state) => {
          const isFav = state.favoriteSessionIds.includes(sessionId);
          if (isFav) {
            return {
              favoriteSessionIds: state.favoriteSessionIds.filter((id) => id !== sessionId),
            };
          } else {
            return {
              favoriteSessionIds: [...state.favoriteSessionIds, sessionId],
            };
          }
        }),

      isFavoriteSession: (sessionId) => {
        return get().favoriteSessionIds.includes(sessionId);
      },

      setSessionThinkingLevel: (sessionId, level) =>
        set((state) => ({
          sessionThinkingLevels: { ...state.sessionThinkingLevels, [sessionId]: level },
        })),

      getSessionThinkingLevel: (sessionId) => {
        return get().sessionThinkingLevels[sessionId] || null;
      },

      setSessionModel: (sessionId, model) =>
        set((state) => ({
          sessionModels: { ...state.sessionModels, [sessionId]: model },
        })),

      getSessionModel: (sessionId) => {
        return get().sessionModels[sessionId] || null;
      },
    }),
    {
      name: "pika-storage",
      // Don't persist Sets (they'll be re-synced from WebSocket)
      partialize: (state) => ({
        currentSessionId: state.currentSessionId,
        lastSeenMessageCounts: state.lastSeenMessageCounts,
        lastProjectId: state.lastProjectId,
        recentSessionIds: state.recentSessionIds,
        favoriteSessionIds: state.favoriteSessionIds,
        sessionThinkingLevels: state.sessionThinkingLevels,
        sessionModels: state.sessionModels,
      }),
    }
  )
);
