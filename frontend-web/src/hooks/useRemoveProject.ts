import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export function useRemoveProject() {
  const queryClient = useQueryClient();

  return useMutation<{ success: boolean }, Error, string>({
    mutationFn: async (projectId) => {
      return apiClient.delete<{ success: boolean }>(`/api/projects/${projectId}`);
    },
    onSuccess: () => {
      // Invalidate projects query to refetch
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      // Also invalidate sessions as they may have changed
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });
}
