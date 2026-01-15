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
import { ScrollArea } from "./ui/scroll-area";
import { Badge } from "./ui/badge";
import { Settings, Trash2, FolderOpen, Plus, Home } from "lucide-react";
import { useProjects } from "../hooks/useProjects";
import { useAddProject } from "../hooks/useAddProject";
import { useRemoveProject } from "../hooks/useRemoveProject";


interface ProjectManagerProps {
  trigger?: React.ReactNode;
}

export function ProjectManager({ trigger }: ProjectManagerProps) {
  const [open, setOpen] = useState(false);
  const [newPath, setNewPath] = useState("");
  const { data: projects, isLoading } = useProjects();
  const addProjectMutation = useAddProject();
  const removeProjectMutation = useRemoveProject();

  const handleAddProject = () => {
    if (!newPath.trim()) return;

    addProjectMutation.mutate(
      { path: newPath.trim() },
      {
        onSuccess: () => {
          setNewPath("");
        },
      }
    );
  };

  const handleRemoveProject = (projectId: string, projectName: string) => {
    if (window.confirm(`Remove project "${projectName}" from config.toml?`)) {
      removeProjectMutation.mutate(projectId);
    }
  };

  const defaultTrigger = (
    <Button variant="ghost" size="sm" className="gap-2">
      <Settings className="h-4 w-4" />
      Manage Projects
    </Button>
  );

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {trigger || defaultTrigger}
      </DialogTrigger>
      <DialogContent className="sm:max-w-md max-h-[90vh] overflow-y-auto w-[calc(100%-2rem)]">
        <DialogHeader>
          <DialogTitle>Manage Projects</DialogTitle>
          <DialogDescription>
            Add or remove projects from config.toml. Changes are saved automatically.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Add new project */}
          <div className="space-y-2">
            <Label htmlFor="new-project-path">Add Project</Label>
            <div className="flex gap-2">
              <Input
                id="new-project-path"
                placeholder="e.g., ~/code/my-project"
                value={newPath}
                onChange={(e) => setNewPath(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newPath.trim()) {
                    handleAddProject();
                  }
                }}
                className="flex-1 font-mono text-sm"
                disabled={addProjectMutation.isPending}
              />
              <Button
                onClick={handleAddProject}
                disabled={!newPath.trim() || addProjectMutation.isPending}
                size="sm"
              >
                <Plus className="h-4 w-4" />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Use ~ for home directory, or provide an absolute path.
            </p>
          </div>

          {/* Quick add buttons */}
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setNewPath("~");
                setTimeout(() => handleAddProject(), 100);
              }}
              disabled={addProjectMutation.isPending}
              className="text-xs"
            >
              <Home className="mr-1 h-3 w-3" />
              Add Home Directory
            </Button>
          </div>

          {/* Project list */}
          <div className="space-y-2">
            <Label>Configured Projects ({projects?.length || 0})</Label>
            <ScrollArea className="h-[200px] border rounded-md p-2">
              {isLoading ? (
                <div className="text-sm text-muted-foreground text-center py-4">
                  Loading projects...
                </div>
              ) : !projects || projects.length === 0 ? (
                <div className="text-sm text-muted-foreground text-center py-4">
                  No projects configured yet
                </div>
              ) : (
                <div className="space-y-2">
                  {projects.map((project) => (
                    <div
                      key={project.id}
                      className="flex flex-col sm:flex-row sm:items-center gap-2 p-3 rounded border bg-card hover:bg-accent/50 transition-colors group"
                    >
                      <div className="flex items-center gap-2 flex-1 min-w-0">
                        <FolderOpen className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                        <div className="flex-1 min-w-0">
                          <div className="font-medium text-sm truncate">
                            {project.name}
                          </div>
                          <div className="text-xs text-muted-foreground truncate font-mono">
                            {project.path}
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-2 justify-between sm:justify-end pl-6 sm:pl-0">
                        <Badge variant="secondary" className="flex-shrink-0">
                          {project.session_count} sessions
                        </Badge>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() =>
                            handleRemoveProject(project.id, project.name)
                          }
                          disabled={removeProjectMutation.isPending}
                          className="flex-shrink-0 opacity-100 sm:opacity-0 sm:group-hover:opacity-100 transition-opacity hover:text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </ScrollArea>
          </div>
        </div>

        <DialogFooter>
          <Button onClick={() => setOpen(false)}>Close</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
