import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError, showSuccess } from "../lib/toast";
import type { CreateSessionRequest, CreateSessionResponse } from "../types";

interface CreateSessionVariables {
  projectId: string;
  request: CreateSessionRequest;
}

export function useCreateSession() {
  const queryClient = useQueryClient();

  return useMutation<CreateSessionResponse, Error, CreateSessionVariables>({
    mutationFn: ({ projectId, request }) =>
      apiClient.post<CreateSessionResponse>(
        `/projects/${projectId}/sessions`,
        request
      ),
    onSuccess: () => {
      // Invalidate and refetch sessions list
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      showSuccess("Session created", "New session has been created successfully.");
    },
    onError: (error) => {
      showError("Failed to create session", error);
    },
  });
}
