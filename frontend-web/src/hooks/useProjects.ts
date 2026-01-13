import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Project } from "../types";

export function useProjects() {
  return useQuery<Project[]>({
    queryKey: ["projects"],
    queryFn: () => apiClient.get<Project[]>("/projects"),
  });
}
