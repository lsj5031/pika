import { useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useProjects } from "../hooks/useProjects";
import { useAppStore } from "../store/appStore";
import { usePullToRefresh } from "../hooks/usePullToRefresh";
import { useRecentSessions } from "../hooks/useRecentSessions";
import { useProjectSessionsPaged } from "../hooks/useProjectSessionsPaged";
import { useSessionsPaged } from "../hooks/useSessionsPaged";
import { useSessionLookup } from "../hooks/useSessionLookup";
import { useResolvedSessions } from "../hooks/useResolvedSessions";
import { ScrollArea } from "./ui/scroll-area";
import { NewSessionDialog } from "./NewSessionDialog";
import { PullToRefreshIndicator } from "./PullToRefreshIndicator";
import { SessionItem } from "./SessionItem";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import {
  Loader2,
  ChevronDown,
  ChevronRight,
  Plus,
  Search,
  X,
  Clock,
  Pin,
  PanelLeftClose,
  Maximize2,
  Minimize2,
} from "lucide-react";
import { cn } from "../lib/utils";
import type { Project, Session } from "../types";

interface SessionListProps {
  className?: string;
  onSelect?: (sessionId: string) => void;
}

const DEFAULT_SESSION_LIMIT = 5;

interface ProjectSessionsSectionProps {
  project: Project;
  isExpanded: boolean;
  onToggle: () => void;
  currentSessionId: string | null;
  activeSessionIds: Set<string>;
  thinkingSessionIds: Set<string>;
  unreadSessions: Set<string>;
  isFavoriteSession: (sessionId: string) => boolean;
  onToggleFavorite: (sessionId: string) => void;
  onSelectSession: (sessionId: string) => void;
  enabled: boolean;
}

function ProjectSessionsSection({
  project,
  isExpanded,
  onToggle,
  currentSessionId,
  activeSessionIds,
  thinkingSessionIds,
  unreadSessions,
  isFavoriteSession,
  onToggleFavorite,
  onSelectSession,
  enabled,
}: ProjectSessionsSectionProps) {
  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
  } = useProjectSessionsPaged(project.id, enabled, DEFAULT_SESSION_LIMIT);

  const sessions = useMemo(() => data?.pages.flatMap((page) => page.data) ?? [], [data]);
  const resolvedSessions = useResolvedSessions(sessions, enabled, { resolveNames: true }) ?? [];

  const displayedSessions = isExpanded
    ? resolvedSessions
    : resolvedSessions.slice(0, DEFAULT_SESSION_LIMIT);

  return (
    <div className="space-y-1">
      <button
        type="button"
        onClick={onToggle}
        className={cn(
          "w-full flex items-center gap-2 px-1 py-1.5 text-sm font-semibold",
          "text-muted-foreground hover:text-foreground transition-colors group"
        )}
      >
        <div className="p-0.5 rounded hover:bg-muted transition-colors">
          {isExpanded ? (
            <ChevronDown className="h-4 w-4" />
          ) : (
            <ChevronRight className="h-4 w-4" />
          )}
        </div>
        <span className="flex-1 text-left truncate">{project.name}</span>
        <Badge variant="outline" className="text-xs font-mono">
          {project.session_count}
        </Badge>
      </button>

      {isLoading && resolvedSessions.length === 0 && (
        <div className="px-8 py-3 text-sm text-muted-foreground italic border rounded-lg opacity-60">
          Loading sessions...
        </div>
      )}

      {!isLoading && displayedSessions.length === 0 && (
        <div className="px-8 py-3 text-sm text-muted-foreground italic border rounded-lg opacity-60">
          No sessions yet
        </div>
      )}

      {displayedSessions.length > 0 && (
        <div className="space-y-1 pl-1">
          {displayedSessions.map((session) => (
            <SessionItem
              key={session.id}
              session={session}
              isSelected={currentSessionId === session.id}
              isActive={activeSessionIds.has(session.id) || session.is_active}
              isThinking={thinkingSessionIds.has(session.id)}
              isUnread={unreadSessions.has(session.id)}
              isFavorite={isFavoriteSession(session.id)}
              onClick={() => onSelectSession(session.id)}
              onToggleFavorite={() => onToggleFavorite(session.id)}
              showFavoriteButton={true}
            />
          ))}
        </div>
      )}

      {hasNextPage && !isExpanded && (
        <Button
          variant="ghost"
          size="sm"
          onClick={onToggle}
          className="w-full justify-center text-xs h-8 font-medium text-muted-foreground hover:text-foreground"
        >
          <span>Show more...</span>
        </Button>
      )}

      {isExpanded && (
        <div className="flex flex-col gap-1">
          {hasNextPage && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => fetchNextPage()}
              disabled={isFetchingNextPage}
              className="w-full justify-center text-xs h-8 font-medium text-muted-foreground hover:text-foreground"
            >
              {isFetchingNextPage ? "Loading..." : "Load more"}
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={onToggle}
            className="w-full justify-center text-xs h-8 font-medium text-muted-foreground hover:text-foreground"
          >
            <span>Show less</span>
          </Button>
        </div>
      )}
    </div>
  );
}

export function SessionList({ className, onSelect }: SessionListProps) {
  const needsAuth = useAppStore((state) => state.needsAuth);
  const queryClient = useQueryClient();
  const { data: projects, isLoading: projectsLoading, refetch: refetchProjects } = useProjects(!needsAuth);
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);
  const activeSessionIds = useAppStore((state) => state.activeSessionIds);
  const thinkingSessionIds = useAppStore((state) => state.thinkingSessionIds);
  const unreadSessions = useAppStore((state) => state.unreadSessions);
  const recentSessionIds = useAppStore((state) => state.recentSessionIds);
  const favoriteSessionIds = useAppStore((state) => state.favoriteSessionIds);
  const sidebarCompactMode = useAppStore((state) => state.sidebarCompactMode);
  const toggleSidebarCompactMode = useAppStore((state) => state.toggleSidebarCompactMode);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);

  const lookupIds = useMemo(
    () => Array.from(new Set([...recentSessionIds, ...favoriteSessionIds, ...activeSessionIds])),
    [recentSessionIds, favoriteSessionIds, activeSessionIds]
  );
  const { data: lookupSessions } = useSessionLookup(lookupIds, !needsAuth);
  const resolvedLookupSessions = useResolvedSessions(lookupSessions, !needsAuth, { resolveNames: true });

  const { recentSessions, favoriteSessions, hasRecentSessions, hasFavoriteSessions, toggleFavoriteSession, isFavoriteSession } =
    useRecentSessions(resolvedLookupSessions);

  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");

  const searchActive = searchQuery.trim().length > 0;
  const {
    data: searchPages,
    isLoading: searchLoading,
    fetchNextPage: fetchNextSearchPage,
    hasNextPage: hasNextSearchPage,
    isFetchingNextPage: isFetchingSearchPage,
  } = useSessionsPaged(searchQuery.trim(), !needsAuth && searchActive);

  const searchSessions = useMemo(
    () => searchPages?.pages.flatMap((page) => page.data) ?? [],
    [searchPages]
  );
  const resolvedSearchSessions = useResolvedSessions(
    searchSessions,
    !needsAuth && searchActive,
    { resolveNames: true }
  );
  const resolvedSearchList = useMemo(
    () => resolvedSearchSessions ?? [],
    [resolvedSearchSessions]
  );

  const searchSessionsByProject = useMemo(() => {
    const grouped = new Map<string, Session[]>();
    for (const session of resolvedSearchList) {
      const list = grouped.get(session.project_id) ?? [];
      list.push(session);
      grouped.set(session.project_id, list);
    }
    return grouped;
  }, [resolvedSearchList]);

  const activeLookupSessions = useMemo(() => {
    if (!resolvedLookupSessions) return [] as Session[];
    return resolvedLookupSessions.filter(
      (session) =>
        (activeSessionIds.has(session.id) || session.is_active) &&
        !recentSessions.find((r) => r.id === session.id) &&
        !favoriteSessions.find((f) => f.id === session.id)
    );
  }, [resolvedLookupSessions, activeSessionIds, recentSessions, favoriteSessions]);

  const isLoading = projectsLoading || (searchActive && searchLoading);

  const handleRefresh = async () => {
    await Promise.all([
      refetchProjects(),
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
    ]);
  };

  const {
    pullDistance,
    isPulling,
    isRefreshing,
    pullProgress,
    pullToRefreshProps,
  } = usePullToRefresh({
    onRefresh: handleRefresh,
    threshold: 80,
  });


  const toggleProjectExpanded = (projectId: string) => {
    setExpandedProjects((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(projectId)) {
        newSet.delete(projectId);
      } else {
        newSet.add(projectId);
      }
      return newSet;
    });
  };

  const handleSessionSelect = (sessionId: string) => {
    setCurrentSession(sessionId);
    onSelect?.(sessionId);
  };

  // Compact mode render
  if (sidebarCompactMode) {
    return (
      <div
        className={cn(
          "flex flex-col h-full bg-background border-r w-16",
          className
        )}
        data-testid="session-list-compact"
      >
        {/* Compact header */}
        <div className="p-2 border-b flex flex-col gap-2">
          <NewSessionDialog
            trigger={
              <Button
                variant="outline"
                size="icon"
                className="h-10 w-10 rounded-lg border-2 shadow-sm"
                title="New Session"
              >
                <Plus className="h-5 w-5" />
              </Button>
            }
          />
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebarCompactMode}
            className="h-8 w-8 rounded-lg"
            title="Expand sidebar"
          >
            <Maximize2 className="h-4 w-4" />
          </Button>
        </div>

        {/* Compact session list */}
        <div className="flex-1 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-2 space-y-2">
              {/* Favorites first */}
              {favoriteSessions.map((session) => (
                <SessionItem
                  key={session.id}
                  session={session}
                  isSelected={currentSessionId === session.id}
                  isActive={activeSessionIds.has(session.id) || session.is_active}
                  isThinking={thinkingSessionIds.has(session.id)}
                  isUnread={unreadSessions.has(session.id)}
                  isFavorite={true}
                  isCompact={true}
                  onClick={() => handleSessionSelect(session.id)}
                />
              ))}

              {/* Recent sessions */}
              {recentSessions
                .filter((s) => !favoriteSessions.find((f) => f.id === s.id))
                .map((session) => (
                  <SessionItem
                    key={session.id}
                    session={session}
                    isSelected={currentSessionId === session.id}
                    isActive={activeSessionIds.has(session.id) || session.is_active}
                    isThinking={thinkingSessionIds.has(session.id)}
                    isUnread={unreadSessions.has(session.id)}
                    isCompact={true}
                    onClick={() => handleSessionSelect(session.id)}
                  />
                ))}

              {/* Active sessions not in recent/favorites */}
              {activeLookupSessions.map((session) => (
                <SessionItem
                  key={session.id}
                  session={session}
                  isSelected={currentSessionId === session.id}
                  isActive={true}
                  isThinking={thinkingSessionIds.has(session.id)}
                  isUnread={unreadSessions.has(session.id)}
                  isCompact={true}
                  onClick={() => handleSessionSelect(session.id)}
                />
              ))}
            </div>
          </ScrollArea>
        </div>

        {/* Compact footer with toggle */}
        <div className="p-2 border-t">
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            className="h-10 w-10 rounded-lg w-full"
            title="Hide sidebar"
          >
            <PanelLeftClose className="h-4 w-4" />
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn("flex flex-col h-full bg-background min-h-0", className)}
      data-testid="session-list"
    >
      {/* Header */}
      <div
        className="p-4 md:pr-4 border-b flex flex-col gap-3 bg-card text-card-foreground shadow-sm z-10"
        data-testid="session-list-header"
      >
        <div className="flex items-center justify-between gap-2 min-w-0">
          <div className="flex items-center gap-2">
            <img
              src="/logo.png"
              alt="Pika Logo"
              className="h-6 w-6 object-contain md:hidden"
            />
            <h2 className="text-xl font-heading font-bold tracking-tight">Sessions</h2>
          </div>
          <div className="flex items-center gap-1">
            <NewSessionDialog
              trigger={
                <Button
                  variant="outline"
                  size="icon"
                  id="new-session-button"
                  data-testid="new-session-button"
                  className="h-9 w-9 rounded-lg border-2 shadow-sm"
                  title="New Session"
                >
                  <Plus className="h-5 w-5" />
                </Button>
              }
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={toggleSidebarCompactMode}
              className="h-9 w-9 rounded-lg hidden md:flex"
              title="Compact mode"
            >
              <Minimize2 className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Search input */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            type="text"
            placeholder="Search sessions..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 pr-9 h-9 rounded-lg border-2 bg-background"
          />
          {searchQuery && (
            <button
              type="button"
              onClick={() => setSearchQuery("")}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>

      {isLoading && (
        <div className="flex-1 flex items-center justify-center text-muted-foreground font-body py-12">
          <Loader2 className="h-6 w-6 animate-spin mr-3" />
          <span className="text-lg">Loading...</span>
        </div>
      )}

      {!isLoading && (
        <div className="flex-1 relative overflow-hidden min-h-0">
          <PullToRefreshIndicator
            pullDistance={pullDistance}
            isPulling={isPulling}
            isRefreshing={isRefreshing}
            pullProgress={pullProgress}
          />
          <ScrollArea className="h-full" {...pullToRefreshProps}>
            <div className="p-3 space-y-4 min-h-full">
              {searchActive && resolvedSearchList.length === 0 && (
                <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                  <Search className="h-12 w-12 mb-3 opacity-50" />
                  <p className="text-lg font-medium">No sessions found</p>
                  <p className="text-sm">Try adjusting your search query</p>
                </div>
              )}

              {searchActive && resolvedSearchList.length > 0 && (
                <div className="space-y-4">
                  {(projects ?? []).map((project) => {
                    const projectSessions = searchSessionsByProject.get(project.id) ?? [];
                    if (projectSessions.length === 0) return null;
                    return (
                      <div key={project.id} className="space-y-1">
                        <div className="flex items-center gap-2 px-1 text-sm font-semibold text-muted-foreground uppercase tracking-wider">
                          <span className="truncate">{project.name}</span>
                          <Badge variant="outline" className="text-xs font-mono">
                            {projectSessions.length}
                          </Badge>
                        </div>
                        <div className="space-y-1 pl-1">
                          {projectSessions.map((session) => (
                            <SessionItem
                              key={session.id}
                              session={session}
                              isSelected={currentSessionId === session.id}
                              isActive={activeSessionIds.has(session.id) || session.is_active}
                              isThinking={thinkingSessionIds.has(session.id)}
                              isUnread={unreadSessions.has(session.id)}
                              isFavorite={isFavoriteSession(session.id)}
                              onClick={() => handleSessionSelect(session.id)}
                              onToggleFavorite={() => toggleFavoriteSession(session.id)}
                              showFavoriteButton={true}
                            />
                          ))}
                        </div>
                      </div>
                    );
                  })}

                  {hasNextSearchPage && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => fetchNextSearchPage()}
                      disabled={isFetchingSearchPage}
                      className="w-full justify-center text-xs h-8 font-medium text-muted-foreground hover:text-foreground"
                    >
                      {isFetchingSearchPage ? "Loading..." : "Load more results"}
                    </Button>
                  )}
                </div>
              )}

              {/* Favorites Section */}
              {!searchActive && hasFavoriteSessions && (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 px-1 text-sm font-semibold text-muted-foreground uppercase tracking-wider">
                    <Pin className="h-3.5 w-3.5" />
                    Favorites
                  </div>
                  <div className="space-y-1">
                    {favoriteSessions.map((session) => (
                      <SessionItem
                        key={session.id}
                        session={session}
                        isSelected={currentSessionId === session.id}
                        isActive={activeSessionIds.has(session.id) || session.is_active}
                        isThinking={thinkingSessionIds.has(session.id)}
                        isUnread={unreadSessions.has(session.id)}
                        isFavorite={true}
                        onClick={() => handleSessionSelect(session.id)}
                        onToggleFavorite={() => toggleFavoriteSession(session.id)}
                        showFavoriteButton={true}
                      />
                    ))}
                  </div>
                </div>
              )}

              {/* Recent Section */}
              {!searchActive && hasRecentSessions && (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 px-1 text-sm font-semibold text-muted-foreground uppercase tracking-wider">
                    <Clock className="h-3.5 w-3.5" />
                    Recent
                  </div>
                  <div className="space-y-1">
                    {recentSessions
                      .filter((s) => !favoriteSessions.find((f) => f.id === s.id))
                      .slice(0, 5)
                      .map((session) => (
                        <SessionItem
                          key={session.id}
                          session={session}
                          isSelected={currentSessionId === session.id}
                          isActive={activeSessionIds.has(session.id) || session.is_active}
                          isThinking={thinkingSessionIds.has(session.id)}
                          isUnread={unreadSessions.has(session.id)}
                          isFavorite={isFavoriteSession(session.id)}
                          onClick={() => handleSessionSelect(session.id)}
                          onToggleFavorite={() => toggleFavoriteSession(session.id)}
                          showFavoriteButton={true}
                        />
                      ))}
                  </div>
                </div>
              )}

              {/* Divider */}
              {!searchActive && (hasFavoriteSessions || hasRecentSessions) && (
                <div className="border-t my-2" />
              )}

              {/* Projects */}
              {!searchActive &&
                projects?.map((project) => (
                  <ProjectSessionsSection
                    key={project.id}
                    project={project}
                    isExpanded={expandedProjects.has(project.id)}
                    onToggle={() => toggleProjectExpanded(project.id)}
                    currentSessionId={currentSessionId}
                    activeSessionIds={activeSessionIds}
                    thinkingSessionIds={thinkingSessionIds}
                    unreadSessions={unreadSessions}
                    isFavoriteSession={isFavoriteSession}
                    onToggleFavorite={toggleFavoriteSession}
                    onSelectSession={handleSessionSelect}
                    enabled={!needsAuth}
                  />
                ))}

              {!searchActive && projects?.length === 0 && (
                <div className="flex flex-col items-center justify-center py-16 text-center space-y-4 text-muted-foreground">
                  <div className="h-16 w-16 rounded-full bg-muted flex items-center justify-center">
                    <Plus className="h-8 w-8 opacity-40" />
                  </div>
                  <div className="space-y-1">
                    <p className="text-lg font-semibold">No projects yet</p>
                    <p className="text-sm px-8">
                      Add a folder or project to start your first coding session.
                    </p>
                  </div>
                  <NewSessionDialog
                    trigger={
                      <Button variant="default" size="sm" className="rounded-lg">
                        Get Started
                      </Button>
                    }
                  />
                </div>
              )}
            </div>
          </ScrollArea>
        </div>
      )}
    </div>
  );
}
