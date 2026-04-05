import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError } from "../lib/toast";

export function useStopSession() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: async (sessionId) => {
      return apiClient.post<void>(`/api/sessions/${sessionId}/stop`, {});
    },
    onSuccess: (_, sessionId) => {
      // Invalidate session queries
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
      queryClient.invalidateQueries({ queryKey: ["sessions", sessionId, "status"] });
    },
    onError: (error) => {
      showError("Failed to stop session", error);
    },
  });
}
