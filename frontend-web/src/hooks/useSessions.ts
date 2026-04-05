import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Session } from "../types";
import { useResolvedSessions } from "./useResolvedSessions";
import { useAppStore } from "../store/appStore";
import { useEffect } from "react";

export function useSessions(
  enabled = true,
  options: { resolveNames?: boolean } = {}
) {
  const { resolveNames = true } = options;

  const sessionsQuery = useQuery<Session[]>({
    queryKey: ["sessions"],
    queryFn: () => apiClient.get<Session[]>("/api/sessions"),
    enabled,
    // Reduce refetch frequency to prevent re-render storms
    staleTime: 30000, // 30 seconds
    refetchInterval: 60000, // Only poll every minute
    refetchIntervalInBackground: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });

  // Sync activeSessionIds from API response
  useEffect(() => {
    if (sessionsQuery.data) {
      const activeIds = new Set(
        sessionsQuery.data.filter((s) => s.is_active).map((s) => s.id)
      );
      useAppStore.getState().setActiveSessionIds(activeIds);
    }
  }, [sessionsQuery.data]);

  const enhancedSessions = useResolvedSessions(sessionsQuery.data, enabled, {
    resolveNames,
  });

  return {
    ...sessionsQuery,
    data: enhancedSessions,
  };
}
