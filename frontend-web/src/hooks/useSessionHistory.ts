import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { apiClient } from "../lib/api";
import type { Message } from "../types";

interface UseSessionHistoryOptions {
  sessionId: string | null;
  enabled?: boolean;
}

// Maximum messages to load initially to prevent UI freeze
const MAX_INITIAL_MESSAGES = 50;

export function useSessionHistory({ sessionId, enabled = true }: UseSessionHistoryOptions) {
  const query = useInfiniteQuery<Message[]>({
    queryKey: ["sessions", sessionId, "messages", "paged"],
    queryFn: ({ pageParam }) => {
      const params = new URLSearchParams();
      params.set("limit", String(MAX_INITIAL_MESSAGES));
      if (pageParam) {
        params.set("before", String(pageParam));
      }
      return apiClient.get<Message[]>(
        `/api/sessions/${sessionId}/messages/paged?${params.toString()}`
      );
    },
    enabled: !!sessionId && enabled,
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => {
      if (!lastPage || lastPage.length < MAX_INITIAL_MESSAGES) return undefined;
      return lastPage[0]?.timestamp ?? undefined;
    },
    staleTime: 5000,
    placeholderData: (previousData) => previousData,
  });

  const messages = useMemo(() => {
    if (!query.data) return undefined;
    return query.data.pages.slice().reverse().flat();
  }, [query.data]);

  return {
    ...query,
    data: messages,
    fetchOlder: query.fetchNextPage,
    hasOlder: query.hasNextPage,
    isFetchingOlder: query.isFetchingNextPage,
  };
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
