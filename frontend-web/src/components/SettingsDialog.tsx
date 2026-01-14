import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Settings, Loader2 } from "lucide-react";
import { usePiSettings } from "../hooks/usePiSettings";
import { useUpdatePiSettings } from "../hooks/useUpdatePiSettings";

const THINKING_LEVELS = [
  { value: "off", label: "Off" },
  { value: "minimal", label: "Minimal" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "xhigh", label: "Extra High" },
];



interface SettingsDialogProps {
  trigger?: React.ReactNode;
}

export function SettingsDialog({ trigger }: SettingsDialogProps) {
  const [open, setOpen] = useState(false);
  const { data: settings, isLoading } = usePiSettings();
  const updateSettingsMutation = useUpdatePiSettings();

  const [localModel, setLocalModel] = useState<string>(settings?.defaultModel || "");
  const [localThinkingLevel, setLocalThinkingLevel] = useState<string>(
    settings?.defaultThinkingLevel || "off"
  );

  const handleSave = () => {
    updateSettingsMutation.mutate(
      {
        defaultModel: localModel,
        defaultThinkingLevel: localThinkingLevel,
      },
      {
        onSuccess: () => {
          setOpen(false);
        },
      }
    );
  };

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    if (newOpen && settings) {
      setLocalModel(settings.defaultModel || "");
      setLocalThinkingLevel(settings.defaultThinkingLevel || "off");
    }
  };

  const defaultTrigger = (
    <Button variant="ghost" size="sm" className="gap-2 px-2 md:px-3">
      <Settings className="h-4 w-4" />
      <span className="hidden md:inline">Settings</span>
      <span className="sr-only">Settings</span>
    </Button>
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        {trigger || defaultTrigger}
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>AI Agent Settings</DialogTitle>
          <DialogDescription>
            Configure default model and thinking level for new sessions.
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <div className="space-y-6 py-4">
            {/* Model Selection */}
            <div className="space-y-2">
              <Label htmlFor="model" className="font-heading font-bold text-base">Default Model</Label>
              <Select value={localModel} onValueChange={setLocalModel}>
                <SelectTrigger id="model" className="min-h-[44px]">
                  <SelectValue placeholder="Select a model" />
                </SelectTrigger>
                <SelectContent>
                  {settings?.availableModels?.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      {model.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {localModel && (
                <p className="text-xs text-muted-foreground">
                  {settings?.availableModels?.find(m => m.id === localModel)?.provider} • {settings?.availableModels?.find(m => m.id === localModel)?.contextWindow?.toLocaleString()} tokens
                </p>
              )}
              <p className="text-xs text-muted-foreground">
                This model will be used for all new sessions by default.
              </p>
            </div>

            {/* Thinking Level */}
            <div className="space-y-2">
              <Label htmlFor="thinking-level" className="font-heading font-bold text-base">Thinking Level</Label>
              <Select
                value={localThinkingLevel}
                onValueChange={setLocalThinkingLevel}
              >
                <SelectTrigger id="thinking-level" className="min-h-[44px]">
                  <SelectValue placeholder="Select thinking level" />
                </SelectTrigger>
                <SelectContent>
                  {THINKING_LEVELS.map((level) => (
                    <SelectItem key={level.value} value={level.value}>
                      {level.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Higher thinking levels produce more detailed reasoning but use
                more tokens and take longer.
              </p>
            </div>

            {/* Current Settings Display */}
            {settings && (
              <div className="rounded-lg border bg-muted/50 p-3 space-y-1 text-sm">
                <div className="font-medium">Current Settings:</div>
                <div className="text-muted-foreground">
                  Provider: {settings.defaultProvider || "Not set"}
                </div>
                <div className="text-muted-foreground">
                  Model: {settings.defaultModel || "Not set"}
                </div>
                <div className="text-muted-foreground">
                  Thinking: {settings.defaultThinkingLevel || "off"}
                </div>
              </div>
            )}
          </div>
        )}

        <div className="flex gap-2 justify-end">
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={updateSettingsMutation.isPending}
          >
            {updateSettingsMutation.isPending ? "Saving..." : "Save Changes"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
