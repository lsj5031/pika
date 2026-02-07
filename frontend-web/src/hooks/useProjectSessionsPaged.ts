import { useInfiniteQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import type { PagedResponse, Session } from "../types";

const DEFAULT_PAGE_LIMIT = 20;

function buildPagedUrl(
  projectId: string,
  options: { limit: number; cursor?: string; q?: string }
) {
  const params = new URLSearchParams();
  params.set("limit", String(options.limit));
  if (options.cursor) params.set("cursor", options.cursor);
  if (options.q) params.set("q", options.q);
  return `/api/projects/${projectId}/sessions/paged?${params.toString()}`;
}

export function useProjectSessionsPaged(
  projectId: string,
  enabled = true,
  limit = DEFAULT_PAGE_LIMIT,
  query = ""
) {
  return useInfiniteQuery<PagedResponse<Session>>({
    queryKey: ["projects", projectId, "sessions", "paged", query, limit],
    queryFn: ({ pageParam }) =>
      apiClient.get<PagedResponse<Session>>(
        buildPagedUrl(projectId, {
          limit,
          cursor: pageParam as string | undefined,
          q: query || undefined,
        })
      ),
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    enabled,
    staleTime: 30000,
  });
}
