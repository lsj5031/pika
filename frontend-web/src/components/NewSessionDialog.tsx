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
import { Plus } from "lucide-react";
import { useProjects } from "../hooks/useProjects";
import { useCreateSession } from "../hooks/useCreateSession";
import { useAppStore } from "../store/appStore";

interface NewSessionDialogProps {
  trigger?: React.ReactNode;
}

export function NewSessionDialog({ trigger }: NewSessionDialogProps) {
  const [open, setOpen] = useState(false);
  const [selectedProjectId, setSelectedProjectId] = useState<string>("");
  const [sessionName, setSessionName] = useState<string>("");

  const { data: projects, isLoading: projectsLoading } = useProjects();
  const createSessionMutation = useCreateSession();
  const setCurrentSession = useAppStore((state) => state.setCurrentSession);

  const isCreateDisabled =
    !selectedProjectId ||
    createSessionMutation.isPending ||
    projectsLoading;

  const handleCreate = () => {
    if (!selectedProjectId) return;

    createSessionMutation.mutate(
      {
        projectId: selectedProjectId,
        request: sessionName.trim() ? { name: sessionName.trim() } : {},
      },
      {
        onSuccess: (result) => {
          // Close the dialog
          setOpen(false);

          // Select the newly created session
          setCurrentSession(result.session_id);

          // Reset form
          setSelectedProjectId("");
          setSessionName("");
        },
      }
    );
  };

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    // Reset form when closing
    if (!newOpen) {
      setSelectedProjectId("");
      setSessionName("");
    }
  };

  const defaultTrigger = (
    <Button variant="outline" size="sm">
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
            Create a new session in a project to start a conversation.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="project">Project</Label>
            <Select
              value={selectedProjectId}
              onValueChange={setSelectedProjectId}
              disabled={projectsLoading}
            >
              <SelectTrigger id="project">
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
                    {project.name}
                    <span className="ml-2 text-muted-foreground text-xs">
                      ({project.path})
                    </span>
                  </SelectItem>
                ))}
                {!projectsLoading && projects?.length === 0 && (
                  <div className="p-2 text-sm text-muted-foreground text-center">
                    No projects found
                  </div>
                )}
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="session-name">Session Name (Optional)</Label>
            <Input
              id="session-name"
              placeholder="e.g., Debug session, Feature discussion"
              value={sessionName}
              onChange={(e) => setSessionName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !isCreateDisabled) {
                  handleCreate();
                }
              }}
            />
          </div>
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => setOpen(false)}
            disabled={createSessionMutation.isPending}
          >
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={isCreateDisabled}>
            {createSessionMutation.isPending ? "Creating..." : "Create"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
