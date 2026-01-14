import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { Session } from "../types";

export function useSessions() {
  return useQuery<Session[]>({
    queryKey: ["sessions"],
    queryFn: () => apiClient.get<Session[]>("/api/sessions"),
  });
}
