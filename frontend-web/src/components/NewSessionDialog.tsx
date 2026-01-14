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
import { Plus, FolderOpen, Home } from "lucide-react";
import { useProjects } from "../hooks/useProjects";
import { useCreateSession } from "../hooks/useCreateSession";
import { useCreateStandaloneSession } from "../hooks/useCreateStandaloneSession";
import { useAppStore } from "../store/appStore";

interface NewSessionDialogProps {
  trigger?: React.ReactNode;
}

export function NewSessionDialog({ trigger }: NewSessionDialogProps) {
  const [open, setOpen] = useState(false);
  const [selectedProjectId, setSelectedProjectId] = useState<string>("");
  const [customPath, setCustomPath] = useState<string>("");
  const [sessionName, setSessionName] = useState<string>("");
  const [useCustomPath, setUseCustomPath] = useState(false);

  const { data: projects, isLoading: projectsLoading } = useProjects();
  const createSessionMutation = useCreateSession();
  const createStandaloneMutation = useCreateStandaloneSession();
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);

  const isProjectCreateDisabled =
    !selectedProjectId ||
    createSessionMutation.isPending ||
    projectsLoading;

  const isCustomCreateDisabled =
    !customPath.trim() ||
    createStandaloneMutation.isPending;

  const handleCreateFromProject = () => {
    if (!selectedProjectId) return;

    createSessionMutation.mutate(
      {
        projectId: selectedProjectId,
        request: sessionName.trim() ? { name: sessionName.trim() } : {},
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
    setCustomPath("~"); // Use home directory
    setUseCustomPath(true);
    // Auto-create with home directory
    setTimeout(() => {
      handleCreateFromPath();
    }, 100);
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
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Create New Session</DialogTitle>
          <DialogDescription>
            Start a new AI coding session in a project or any folder.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Toggle between project and custom path */}
          <div className="flex gap-2">
            <Button
              type="button"
              variant={!useCustomPath ? "default" : "outline"}
              onClick={() => setUseCustomPath(false)}
              className="flex-1"
            >
              <FolderOpen className="mr-2 h-4 w-4" />
              From Project
            </Button>
            <Button
              type="button"
              variant={useCustomPath ? "default" : "outline"}
              onClick={() => setUseCustomPath(true)}
              className="flex-1"
            >
              <Home className="mr-2 h-4 w-4" />
              Any Folder
            </Button>
          </div>

          {!useCustomPath ? (
            // Project-based creation
            <>
              <div className="grid gap-2">
                <Label htmlFor="project">Project</Label>
                <Select
                  value={selectedProjectId}
                  onValueChange={setSelectedProjectId}
                  disabled={projectsLoading}
                >
                  <SelectTrigger id="project" data-testid="project-select" className="min-h-[44px]">
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
                        <div className="flex flex-col">
                          <span>{project.name}</span>
                          <span className="text-muted-foreground text-xs">
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
                <Label htmlFor="session-name-project">Session Name (Optional)</Label>
                <Input
                  id="session-name-project"
                  placeholder="e.g., Debug session, Feature discussion"
                  value={sessionName}
                  onChange={(e) => setSessionName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !isProjectCreateDisabled) {
                      handleCreateFromProject();
                    }
                  }}
                />
              </div>

              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => setOpen(false)}
                  disabled={createSessionMutation.isPending}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleCreateFromProject}
                  disabled={isProjectCreateDisabled}
                  id="create-session-button"
                  data-testid="create-session-button"
                  className="min-h-[44px]"
                >
                  {createSessionMutation.isPending ? "Creating..." : "Create"}
                </Button>
              </DialogFooter>
            </>
          ) : (
            // Custom path creation
            <>
              <div className="grid gap-2">
                <Label htmlFor="custom-path">Folder Path</Label>
                <Input
                  id="custom-path"
                  placeholder="e.g., ~/code/my-project or /absolute/path/to/project"
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
                <Label htmlFor="session-name-custom">Session Name (Optional)</Label>
                <Input
                  id="session-name-custom"
                  placeholder="e.g., Quick chat, Code review"
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

              <DialogFooter>
                <Button
                  variant="outline"
                  onClick={() => setOpen(false)}
                  disabled={createStandaloneMutation.isPending}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleCreateFromPath}
                  disabled={isCustomCreateDisabled}
                  id="create-custom-session-button"
                  data-testid="create-custom-session-button"
                  className="min-h-[44px]"
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
