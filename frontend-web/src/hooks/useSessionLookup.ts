import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Session } from "../types";

export function useSessionLookup(sessionIds: string[], enabled = true) {
  const uniqueIds = Array.from(new Set(sessionIds));

  return useQuery<Session[]>({
    queryKey: ["sessions", "lookup", uniqueIds],
    queryFn: () => apiClient.post<Session[]>("/api/sessions/lookup", { ids: uniqueIds }),
    enabled: enabled && uniqueIds.length > 0,
    staleTime: 30000,
  });
}
