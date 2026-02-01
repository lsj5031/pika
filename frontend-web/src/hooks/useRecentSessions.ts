import { useMemo } from "react";
import { useAppStore } from "../store/appStore";
import type { Session } from "../types";

interface UseRecentSessionsResult {
  recentSessions: Session[];
  favoriteSessions: Session[];
  hasRecentSessions: boolean;
  hasFavoriteSessions: boolean;
  addRecentSession: (sessionId: string) => void;
  removeRecentSession: (sessionId: string) => void;
  toggleFavoriteSession: (sessionId: string) => void;
  isFavoriteSession: (sessionId: string) => boolean;
  clearRecentSessions: () => void;
}

export function useRecentSessions(allSessions: Session[] | undefined): UseRecentSessionsResult {
  const recentSessionIds = useAppStore((state) => state.recentSessionIds);
  const favoriteSessionIds = useAppStore((state) => state.favoriteSessionIds);
  const addRecent = useAppStore((state) => state.addRecentSession);
  const removeRecent = useAppStore((state) => state.removeRecentSession);
  const toggleFavorite = useAppStore((state) => state.toggleFavoriteSession);
  const isFavorite = useAppStore((state) => state.isFavoriteSession);

  // Create a map for O(1) lookup
  const sessionMap = useMemo(() => {
    const map = new Map<string, Session>();
    allSessions?.forEach((session) => map.set(session.id, session));
    return map;
  }, [allSessions]);

  // Get recent sessions in order (filtering out deleted ones)
  const recentSessions = useMemo(() => {
    return recentSessionIds
      .map((id) => sessionMap.get(id))
      .filter((session): session is Session => session !== undefined);
  }, [recentSessionIds, sessionMap]);

  // Get favorite sessions
  const favoriteSessions = useMemo(() => {
    return favoriteSessionIds
      .map((id) => sessionMap.get(id))
      .filter((session): session is Session => session !== undefined);
  }, [favoriteSessionIds, sessionMap]);

  const clearRecentSessions = () => {
    // Keep only favorites in recent if they exist
    const filtered = recentSessionIds.filter((id) => favoriteSessionIds.includes(id));
    useAppStore.setState({ recentSessionIds: filtered });
  };

  return {
    recentSessions,
    favoriteSessions,
    hasRecentSessions: recentSessions.length > 0,
    hasFavoriteSessions: favoriteSessions.length > 0,
    addRecentSession: addRecent,
    removeRecentSession: removeRecent,
    toggleFavoriteSession: toggleFavorite,
    isFavoriteSession: isFavorite,
    clearRecentSessions,
  };
}
