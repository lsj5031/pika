import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError, showSuccess } from "../lib/toast";
import type { StartSessionResponse } from "../types";

export function useStartSession() {
  const queryClient = useQueryClient();

  return useMutation<StartSessionResponse, Error, string>({
    mutationFn: (sessionId) =>
      apiClient.post<StartSessionResponse>(`/api/sessions/${sessionId}/start`, {}),
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
      showSuccess("Session started", "The session is now active.");
    },
    onError: (error) => {
      showError("Failed to start session", error);
    },
  });
}
