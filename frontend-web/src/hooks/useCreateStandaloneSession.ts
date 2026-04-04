import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError } from "../lib/toast";

export interface CreateStandaloneSessionRequest {
  path: string;
  name?: string;
}

export interface CreateStandaloneSessionResponse {
  session_id: string;
  project_id?: string;
}

export function useCreateStandaloneSession() {
  const queryClient = useQueryClient();

  return useMutation<
    CreateStandaloneSessionResponse,
    Error,
    CreateStandaloneSessionRequest
  >({
    mutationFn: async (request) => {
      return apiClient.post<CreateStandaloneSessionResponse>(
        "/api/sessions/create",
        request
      );
    },
    onSuccess: () => {
      // Invalidate sessions query to refetch
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
    onError: (error) => {
      showError("Failed to create standalone session", error);
    },
  });
}
