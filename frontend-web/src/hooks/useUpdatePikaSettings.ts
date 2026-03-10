import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export interface UpdatePikaSettingsRequest {
  defaultModel?: string;
  defaultThinkingLevel?: string;
  defaultProvider?: string;
  hideThinkingBlock?: boolean;
}

export function useUpdatePikaSettings() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, UpdatePikaSettingsRequest>({
    mutationFn: async (request) => {
      return apiClient.post<void>("/api/settings", request);
    },
    onSuccess: () => {
      // Invalidate settings query to refetch
      queryClient.invalidateQueries({ queryKey: ["pika-settings"] });
    },
  });
}
