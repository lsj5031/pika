import { useSessions } from "../hooks/useSessions";
import { useProjects } from "../hooks/useProjects";
import { useAppStore } from "../store/appStore";
import { useStartSession } from "../hooks/useStartSession";
import { ScrollArea } from "./ui/scroll-area";
import { NewSessionDialog } from "./NewSessionDialog";
import { cn } from "../lib/utils";

interface SessionListProps {
  className?: string;
}

export function SessionList({ className }: SessionListProps) {
  const { data: sessions, isLoading: sessionsLoading } = useSessions();
  const { data: projects, isLoading: projectsLoading } = useProjects();
  const currentSessionId = useAppStore((state) => state.currentSessionId);
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);
  const startSessionMutation = useStartSession();

  const isLoading = sessionsLoading || projectsLoading;

  // Group sessions by project
  const sessionsByProject = projects?.map((project) => ({
    ...project,
    sessions: sessions?.filter((session) => session.project_id === project.id) ?? [],
  }));

  const handleSessionSelect = (sessionId: string) => {
    setCurrentSession(sessionId);

    // Auto-start the session if it's not already active
    const session = sessions?.find((s) => s.id === sessionId);
    if (session && !session.is_active) {
      startSessionMutation.mutate(sessionId);
    }
  };

  return (
    <div className={cn("flex flex-col h-full", className)} data-testid="session-list">
      {/* Header */}
      <div className="p-4 border-b flex items-center justify-between" data-testid="session-list-header">
        <h2 className="text-lg font-semibold">Sessions</h2>
        <NewSessionDialog />
      </div>

      {isLoading && (
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          Loading...
        </div>
      )}

      {!isLoading && (
        /* Sessions list */
        <ScrollArea className="flex-1">
          <div className="p-2">
            {sessionsByProject?.map((project) => (
              <div key={project.id} className="mb-4">
                {/* Project header */}
                <div className="px-2 py-1 text-xs font-medium text-muted-foreground uppercase">
                  {project.name}
                </div>

                {/* Project sessions */}
                {project.sessions.length === 0 ? (
                  <div className="px-2 py-1 text-sm text-muted-foreground">
                    No sessions
                  </div>
                ) : (
                  <ul className="space-y-1">
                    {project.sessions.map((session) => (
                      <li key={session.id}>
                        <button
                          onClick={() => handleSessionSelect(session.id)}
                          className={cn(
                            "w-full flex items-center gap-2 px-3 py-3 text-sm rounded-wobbly transition-all min-h-[44px]",
                            "hover:bg-accent hover:text-accent-foreground hover:rotate-1",
                            currentSessionId === session.id &&
                            "bg-accent text-accent-foreground rotate-1 shadow-sm"
                          )}
                          data-testid={`session-item-${session.id}`}
                        >
                          {/* Active status indicator */}
                          {session.is_active && (
                            <span
                              className="h-2 w-2 rounded-full bg-green-500"
                              aria-label="Active session"
                            />
                          )}
                          {!session.is_active && (
                            <span className="h-2 w-2" aria-hidden="true" />
                          )}

                          {/* Session name */}
                          <span className="flex-1 text-left truncate">
                            {session.name || "Untitled Session"}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ))}

            {sessionsByProject?.length === 0 && (
              <div className="px-2 py-4 text-sm text-muted-foreground text-center">
                No projects found
              </div>
            )}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
