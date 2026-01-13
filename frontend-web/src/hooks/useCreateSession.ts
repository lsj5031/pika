import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
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
    },
  });
}
