import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Message } from "../types";

interface UseSessionHistoryOptions {
  sessionId: string | null;
  enabled?: boolean;
}

// Maximum messages to load initially to prevent UI freeze
const MAX_INITIAL_MESSAGES = 50;

export function useSessionHistory({ sessionId, enabled = true }: UseSessionHistoryOptions) {
  return useQuery<Message[]>({
    queryKey: ["sessions", sessionId, "messages"],
    queryFn: async () => {
      return apiClient.get<Message[]>(
        `/api/sessions/${sessionId}/messages?limit=${MAX_INITIAL_MESSAGES}&direction=tail`
      );
    },
    enabled: !!sessionId && enabled,
    // Reduce refetch frequency
    staleTime: 5000, // 5 seconds
    // Keep previous data while fetching to avoid flash of empty content
    placeholderData: (previousData) => previousData,
  });
}

// Hook to load all messages (for export/full history)
export function useFullSessionHistory({ sessionId, enabled = true }: UseSessionHistoryOptions) {
  return useQuery<Message[]>({
    queryKey: ["sessions", sessionId, "messages", "full"],
    queryFn: () => apiClient.get<Message[]>(`/api/sessions/${sessionId}/messages?direction=tail`),
    enabled: !!sessionId && enabled,
    staleTime: 30000,
  });
}
