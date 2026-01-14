import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError, showSuccess } from "../lib/toast";
import type { StopSessionResponse } from "../types";

export function useStopSession() {
  const queryClient = useQueryClient();

  return useMutation<StopSessionResponse, Error, string>({
    mutationFn: (sessionId) =>
      apiClient.post<StopSessionResponse>(`/sessions/${sessionId}/stop`, {}),
    onSuccess: (_data, sessionId) => {
      // Update the session in the cache
      queryClient.setQueryData(
        ["sessions", sessionId],
        (oldSession: unknown) => {
          if (oldSession && typeof oldSession === "object" && "is_active" in oldSession) {
            return { ...oldSession, is_active: false };
          }
          return oldSession;
        }
      );
      // Invalidate sessions list to get fresh data
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
      showSuccess("Session stopped", "The session has been stopped.");
    },
    onError: (error) => {
      showError("Failed to stop session", error);
    },
  });
}
