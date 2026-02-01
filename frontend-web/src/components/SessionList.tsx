import { useState } from "react";
import { useSessions } from "../hooks/useSessions";
import { useProjects } from "../hooks/useProjects";
import { useAppStore } from "../store/appStore";
import { usePullToRefresh } from "../hooks/usePullToRefresh";
import { useRecentSessions } from "../hooks/useRecentSessions";
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

interface SessionListProps {
  className?: string;
  onSelect?: (sessionId: string) => void;
}

const DEFAULT_SESSION_LIMIT = 5;

export function SessionList({ className, onSelect }: SessionListProps) {
  const { data: sessions, isLoading: sessionsLoading, refetch: refetchSessions } = useSessions();
  const { data: projects, isLoading: projectsLoading, refetch: refetchProjects } = useProjects();
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);
  const activeSessionIds = useAppStore((state) => state.activeSessionIds);
  const thinkingSessionIds = useAppStore((state) => state.thinkingSessionIds);
  const unreadSessions = useAppStore((state) => state.unreadSessions);
  const sidebarCompactMode = useAppStore((state) => state.sidebarCompactMode);
  const toggleSidebarCompactMode = useAppStore((state) => state.toggleSidebarCompactMode);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);

  const { recentSessions, favoriteSessions, hasRecentSessions, hasFavoriteSessions, toggleFavoriteSession, isFavoriteSession } =
    useRecentSessions(sessions);

  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");

  const isLoading = sessionsLoading || projectsLoading;

  const handleRefresh = async () => {
    // refetch already triggers a fresh fetch; no need for additional invalidation
    await Promise.all([refetchSessions(), refetchProjects()]);
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

  // Filter sessions based on search query
  const filteredSessions =
    sessions?.filter((session) => {
      if (!searchQuery.trim()) return true;
      const query = searchQuery.toLowerCase();
      return (
        session.name.toLowerCase().includes(query) ||
        session.project_path.toLowerCase().includes(query) ||
        session.id.toLowerCase().includes(query)
      );
    }) ?? [];

  // Group sessions by project and limit to DEFAULT_SESSION_LIMIT
  const sessionsByProject = projects?.map((project) => {
    const projectSessions = filteredSessions.filter(
      (session) => session.project_id === project.id
    );
    // Sort sessions by creation date desc (assuming newer is more relevant)
    projectSessions.sort(
      (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );

    const isExpanded = expandedProjects.has(project.id);
    const displayedSessions = isExpanded
      ? projectSessions
      : projectSessions.slice(0, DEFAULT_SESSION_LIMIT);
    const hasMore = projectSessions.length > DEFAULT_SESSION_LIMIT;

    return {
      ...project,
      sessions: projectSessions,
      displayedSessions,
      hasMore,
      isExpanded,
    };
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
              {sessions
                ?.filter(
                  (s) =>
                    (activeSessionIds.has(s.id) || s.is_active) &&
                    !recentSessions.find((r) => r.id === s.id) &&
                    !favoriteSessions.find((f) => f.id === s.id)
                )
                .map((session) => (
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
      className={cn("flex flex-col h-full bg-background", className)}
      data-testid="session-list"
    >
      {/* Header */}
      <div
        className="p-4 pr-16 md:pr-4 border-b flex flex-col gap-3 bg-card text-card-foreground shadow-sm z-10"
        data-testid="session-list-header"
      >
        <div className="flex items-center justify-between gap-2">
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
        <div className="flex-1 relative overflow-hidden">
          <PullToRefreshIndicator
            pullDistance={pullDistance}
            isPulling={isPulling}
            isRefreshing={isRefreshing}
            pullProgress={pullProgress}
          />
          <ScrollArea className="h-full" {...pullToRefreshProps}>
            <div className="p-3 space-y-4 min-h-full">
              {filteredSessions.length === 0 && searchQuery.trim() && (
                <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                  <Search className="h-12 w-12 mb-3 opacity-50" />
                  <p className="text-lg font-medium">No sessions found</p>
                  <p className="text-sm">Try adjusting your search query</p>
                </div>
              )}

              {/* Favorites Section */}
              {!searchQuery.trim() && hasFavoriteSessions && (
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
              {!searchQuery.trim() && hasRecentSessions && (
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
              {!searchQuery.trim() && (hasFavoriteSessions || hasRecentSessions) && (
                <div className="border-t my-2" />
              )}

              {/* Projects */}
              {sessionsByProject?.map((project) => (
                <div key={project.id} className="space-y-1">
                  {/* Project header */}
                  <button
                    type="button"
                    onClick={() => toggleProjectExpanded(project.id)}
                    className={cn(
                      "w-full flex items-center gap-2 px-1 py-1.5 text-sm font-semibold",
                      "text-muted-foreground hover:text-foreground transition-colors group"
                    )}
                  >
                    <div className="p-0.5 rounded hover:bg-muted transition-colors">
                      {project.isExpanded ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                    </div>
                    <span className="flex-1 text-left truncate">{project.name}</span>
                    <Badge variant="outline" className="text-xs font-mono">
                      {project.sessions.length}
                    </Badge>
                  </button>

                  {/* Project sessions */}
                  {project.displayedSessions.length === 0 ? (
                    <div className="px-8 py-3 text-sm text-muted-foreground italic border rounded-lg opacity-60">
                      No sessions yet
                    </div>
                  ) : (
                    <div className="space-y-1 pl-1">
                      {project.displayedSessions.map((session) => (
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
                  )}

                  {/* Show more/less button */}
                  {project.hasMore && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => toggleProjectExpanded(project.id)}
                      className="w-full justify-center text-xs h-8 font-medium text-muted-foreground hover:text-foreground"
                    >
                      {project.isExpanded ? (
                        <span>Show less</span>
                      ) : (
                        <span>Show {project.sessions.length - DEFAULT_SESSION_LIMIT} more...</span>
                      )}
                    </Button>
                  )}
                </div>
              ))}

              {sessionsByProject?.length === 0 && (
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
