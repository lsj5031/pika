import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export function useStopSession() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: async (sessionId) => {
      return apiClient.post<void>(`/sessions/${sessionId}/stop`, {});
    },
    onSuccess: (_, sessionId) => {
      // Invalidate session queries
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
      queryClient.invalidateQueries({ queryKey: ["sessions", sessionId, "status"] });
    },
  });
}
