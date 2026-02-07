import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Session } from "../types";
import { useResolvedSessions } from "./useResolvedSessions";

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

  const enhancedSessions = useResolvedSessions(sessionsQuery.data, enabled, {
    resolveNames,
  });

  return {
    ...sessionsQuery,
    data: enhancedSessions,
  };
}
