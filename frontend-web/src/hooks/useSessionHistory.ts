import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Message } from "../types";

interface UseSessionHistoryOptions {
  sessionId: string | null;
}

export function useSessionHistory({ sessionId }: UseSessionHistoryOptions) {
  return useQuery<Message[]>({
    queryKey: ["sessions", sessionId, "messages"],
    queryFn: () => apiClient.get<Message[]>(`/sessions/${sessionId}/messages`),
    enabled: !!sessionId,
  });
}
