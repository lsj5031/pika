import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export interface UpdatePiSettingsRequest {
  defaultModel?: string;
  defaultThinkingLevel?: string;
  defaultProvider?: string;
  hideThinkingBlock?: boolean;
}

export function useUpdatePiSettings() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, UpdatePiSettingsRequest>({
    mutationFn: async (request) => {
      return apiClient.post<void>("/settings", request);
    },
    onSuccess: () => {
      // Invalidate settings query to refetch
      queryClient.invalidateQueries({ queryKey: ["pi-settings"] });
    },
  });
}
