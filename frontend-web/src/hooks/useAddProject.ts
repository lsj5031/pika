import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export interface AddProjectRequest {
  path: string;
}

export interface AddProjectResponse {
  id: string;
  name: string;
  path: string;
}

export function useAddProject() {
  const queryClient = useQueryClient();

  return useMutation<AddProjectResponse, Error, AddProjectRequest>({
    mutationFn: async (request) => {
      return apiClient.post<AddProjectResponse>("/api/projects", request);
    },
    onSuccess: () => {
      // Invalidate projects query to refetch
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      // Also invalidate sessions as they may have changed
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });
}
