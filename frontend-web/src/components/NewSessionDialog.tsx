import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select";
import { Plus, FolderOpen, Home, Clock } from "lucide-react";
import { useProjects } from "../hooks/useProjects";
import { useCreateSession } from "../hooks/useCreateSession";
import { useCreateStandaloneSession } from "../hooks/useCreateStandaloneSession";
import { useAppStore } from "../store/appStore";
import { useMemo } from "react";

interface NewSessionDialogProps {
  trigger?: React.ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function NewSessionDialog({ trigger, open: controlledOpen, onOpenChange: setControlledOpen }: NewSessionDialogProps) {
  const needsAuth = useAppStore((state) => state.needsAuth);
  const [internalOpen, setInternalOpen] = useState(false);
  
  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;
  const setOpen = isControlled ? setControlledOpen! : setInternalOpen;

  const [selectedProjectId, setSelectedProjectId] = useState<string>("");
  const [customPath, setCustomPath] = useState<string>("");
  const [sessionName, setSessionName] = useState<string>("");
  const [useCustomPath, setUseCustomPath] = useState(false);

  const { data: projects, isLoading: projectsLoading } = useProjects(!needsAuth);
  const createSessionMutation = useCreateSession();
  const createStandaloneMutation = useCreateStandaloneSession();
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);
  const lastProjectId = useAppStore((state) => state.lastProjectId);
  const setLastProject = useAppStore((state) => state.setLastProject);

  const lastProject = useMemo(() => {
    if (!lastProjectId || !projects) return null;
    return projects.find((p) => p.id === lastProjectId) || null;
  }, [lastProjectId, projects]);

  const isProjectCreateDisabled =
    !selectedProjectId ||
    createSessionMutation.isPending ||
    projectsLoading;

  const isCustomCreateDisabled =
    !customPath.trim() ||
    createStandaloneMutation.isPending;

  const handleCreateFromProject = (projectIdOverride?: string) => {
    const projectId = projectIdOverride || selectedProjectId;
    if (!projectId) return;

    createSessionMutation.mutate(
      {
        projectId: projectId,
        request: sessionName.trim() ? { name: sessionName.trim() } : {},
      },
      {
        onSuccess: (result) => {
          setOpen(false);
          setCurrentSession(result.session_id);
          setLastProject(projectId);
          resetForm();
        },
      }
    );
  };

  const handleQuickStartRecent = () => {
    if (!lastProject) return;
    handleCreateFromProject(lastProject.id);
  };

  const handleCreateFromPath = () => {
    if (!customPath.trim()) return;

    createStandaloneMutation.mutate(
      {
        path: customPath.trim(),
        name: sessionName.trim() || undefined,
      },
      {
        onSuccess: (result) => {
          setOpen(false);
          setCurrentSession(result.session_id);
          resetForm();
        },
      }
    );
  };

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    if (!newOpen) {
      resetForm();
    }
  };

  const resetForm = () => {
    setSelectedProjectId("");
    setCustomPath("");
    setSessionName("");
    setUseCustomPath(false);
  };

  const handleQuickStart = () => {
    createStandaloneMutation.mutate(
      {
        path: "~",
        name: sessionName.trim() || undefined,
      },
      {
        onSuccess: (result) => {
          setOpen(false);
          setCurrentSession(result.session_id);
          resetForm();
        },
      }
    );
  };

  const defaultTrigger = (
    <Button variant="outline" size="sm" id="new-session-button" data-testid="new-session-button" className="min-h-[44px]">
      <Plus className="mr-2 h-4 w-4" />
      New Session
    </Button>
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        {trigger || defaultTrigger}
      </DialogTrigger>
      <DialogContent className="w-full max-w-[calc(100vw-2rem)] sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Create New Session</DialogTitle>
          <DialogDescription>
            Start a new AI coding session in a project or any folder.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2 sm:py-4">
          {/* Quick start with recent project */}
          {lastProject && (
            <Button
              type="button"
              variant="secondary"
              onClick={handleQuickStartRecent}
              disabled={createSessionMutation.isPending}
              className="w-full text-sm sm:text-base"
            >
              <Clock className="mr-2 h-4 w-4 shrink-0" />
              <span className="truncate">Quick Start: {lastProject.name}</span>
            </Button>
          )}

          {/* Toggle between project and custom path */}
          <div className="grid grid-cols-2 gap-2">
            <Button
              type="button"
              variant={!useCustomPath ? "default" : "outline"}
              onClick={() => setUseCustomPath(false)}
              className="text-sm sm:text-base px-2 sm:px-4"
            >
              <FolderOpen className="mr-1 sm:mr-2 h-4 w-4 shrink-0" />
              <span className="truncate">From Project</span>
            </Button>
            <Button
              type="button"
              variant={useCustomPath ? "default" : "outline"}
              onClick={() => setUseCustomPath(true)}
              className="text-sm sm:text-base px-2 sm:px-4"
            >
              <Home className="mr-1 sm:mr-2 h-4 w-4 shrink-0" />
              <span className="truncate">Any Folder</span>
            </Button>
          </div>

          {!useCustomPath ? (
            // Project-based creation
            <>
              <div className="grid gap-2">
                <Label htmlFor="project" className="font-heading font-bold text-base">Project</Label>
                <Select
                  value={selectedProjectId}
                  onValueChange={setSelectedProjectId}
                  disabled={projectsLoading}
                >
                  <SelectTrigger id="project" data-testid="project-select" className="h-auto min-h-[44px] [&>span]:line-clamp-none">
                    <SelectValue
                      placeholder={
                        projectsLoading
                          ? "Loading projects..."
                          : "Select a project"
                      }
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {projects?.map((project) => (
                      <SelectItem key={project.id} value={project.id}>
                        <div className="flex flex-col py-1">
                          <span className="font-heading font-bold">{project.name}</span>
                          <span className="text-muted-foreground text-xs font-mono opacity-80 mt-0.5">
                            {project.path}
                          </span>
                        </div>
                      </SelectItem>
                    ))}
                    {!projectsLoading && projects?.length === 0 && (
                      <div className="p-2 text-sm text-muted-foreground text-center">
                        No projects found. Use "Any Folder" instead.
                      </div>
                    )}
                  </SelectContent>
                </Select>
              </div>

              <div className="grid gap-2">
                <Label htmlFor="session-name-project" className="font-heading font-bold text-base">Session Name (Optional)</Label>
                <Input
                  id="session-name-project"
                  placeholder="e.g., Debugging, Feature work"
                  value={sessionName}
                  onChange={(e) => setSessionName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !isProjectCreateDisabled) {
                      handleCreateFromProject();
                    }
                  }}
                />
              </div>

              <DialogFooter className="flex-row justify-start gap-3 pt-4">
                <Button
                  variant="outline"
                  onClick={() => setOpen(false)}
                  disabled={createSessionMutation.isPending}
                  className="flex-1 sm:flex-none"
                >
                  Cancel
                </Button>
                <Button
                  variant="accent"
                  onClick={() => handleCreateFromProject()}
                  disabled={isProjectCreateDisabled}
                  id="create-session-button"
                  data-testid="create-session-button"
                  className="flex-1 sm:flex-none"
                >
                  {createSessionMutation.isPending ? "Creating..." : "Create"}
                </Button>
              </DialogFooter>
            </>
          ) : (
            // Custom path creation
            <>
              <div className="grid gap-2">
                <Label htmlFor="custom-path" className="font-heading font-bold text-base">Folder Path</Label>
                <Input
                  id="custom-path"
                  placeholder="e.g., ~/code/my-project"
                  value={customPath}
                  onChange={(e) => setCustomPath(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !isCustomCreateDisabled) {
                      handleCreateFromPath();
                    }
                  }}
                  className="font-mono text-sm"
                />
                <p className="text-xs text-muted-foreground">
                  Use ~ for home directory, or provide an absolute/relative path.
                  The AI will have access to this folder and its subdirectories.
                </p>
              </div>

              <div className="grid gap-2">
                <Label htmlFor="session-name-custom" className="font-heading font-bold text-base">Session Name (Optional)</Label>
                <Input
                  id="session-name-custom"
                  placeholder="e.g., Quick chat"
                  value={sessionName}
                  onChange={(e) => setSessionName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !isCustomCreateDisabled) {
                      handleCreateFromPath();
                    }
                  }}
                />
              </div>

              {/* Quick start buttons */}
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleQuickStart}
                  disabled={createStandaloneMutation.isPending}
                  className="text-xs"
                >
                  <Home className="mr-1 h-3 w-3" />
                  Quick Start (Home)
                </Button>
              </div>

              <DialogFooter className="flex-row justify-start gap-3 pt-4">
                <Button
                  variant="outline"
                  onClick={() => setOpen(false)}
                  disabled={createStandaloneMutation.isPending}
                  className="flex-1 sm:flex-none"
                >
                  Cancel
                </Button>
                <Button
                  variant="accent"
                  onClick={handleCreateFromPath}
                  disabled={isCustomCreateDisabled}
                  id="create-custom-session-button"
                  data-testid="create-custom-session-button"
                  className="flex-1 sm:flex-none"
                >
                  {createStandaloneMutation.isPending ? "Creating..." : "Create"}
                </Button>
              </DialogFooter>
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
