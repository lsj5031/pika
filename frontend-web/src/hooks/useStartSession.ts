import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { StartSessionResponse } from "../types";

export function useStartSession() {
  const queryClient = useQueryClient();

  return useMutation<StartSessionResponse, Error, string>({
    mutationFn: (sessionId) =>
      apiClient.post<StartSessionResponse>(`/sessions/${sessionId}/start`, {}),
    onSuccess: (_data, sessionId) => {
      // Update the session in the cache
      queryClient.setQueryData(
        ["sessions", sessionId],
        (oldSession: unknown) => {
          if (oldSession && typeof oldSession === "object" && "is_active" in oldSession) {
            return { ...oldSession, is_active: true };
          }
          return oldSession;
        }
      );
      // Invalidate sessions list to get fresh data
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });
}
