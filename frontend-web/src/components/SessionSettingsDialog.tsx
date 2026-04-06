import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "./ui/sheet";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Loader2 } from "lucide-react";
import { usePikaSettings } from "../hooks/usePikaSettings";
import { useSwipeToClose } from "../hooks/useSwipe";
import { useAppStore } from "../store/appStore";
import { apiClient } from "../lib/api";
import { toast } from "sonner";

const THINKING_LEVELS = [
  { value: "off", label: "Off" },
  { value: "minimal", label: "Minimal" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "xhigh", label: "Extra High" },
];

function useIsMobile() {
  const [isMobile, setIsMobile] = useState(window.innerWidth < 640);

  useEffect(() => {
    const handleResize = () => {
      setIsMobile(window.innerWidth < 640);
    };

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  return isMobile;
}

interface SessionSettingsDialogProps {
  sessionId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SessionSettingsDialog({ sessionId, open, onOpenChange }: SessionSettingsDialogProps) {
  const needsAuth = useAppStore((state) => state.needsAuth);
  const isMobile = useIsMobile();

  const { data: settings, isLoading } = usePikaSettings(!needsAuth);

  const sessionModel = useAppStore((state) => state.sessionModels?.[sessionId]);
  const storedThinkingLevel = useAppStore((state) => state.sessionThinkingLevels[sessionId]);
  const setSessionModel = useAppStore((state) => state.setSessionModel);
  const setSessionThinkingLevel = useAppStore((state) => state.setSessionThinkingLevel);

  const [localModel, setLocalModel] = useState("");
  const [localThinkingLevel, setLocalThinkingLevel] = useState("");
  const [saving, setSaving] = useState(false);

  const { swipeProps: sheetSwipeProps } = useSwipeToClose(
    () => onOpenChange(false),
    { direction: "down", threshold: 80 }
  );

  const effectiveModel = localModel || sessionModel?.id || settings?.defaultModel || "";
  const effectiveThinkingLevel =
    localThinkingLevel || storedThinkingLevel || settings?.defaultThinkingLevel || "off";

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);

    try {
      const selectedModel = settings.availableModels?.find(m => m.id === effectiveModel);

      if (selectedModel && effectiveModel !== (sessionModel?.id || settings.defaultModel)) {
        await apiClient.post(`/api/sessions/${sessionId}/set-model`, {
          provider: selectedModel.provider,
          modelId: selectedModel.id,
        });
        setSessionModel(sessionId, {
          id: selectedModel.id,
          name: selectedModel.name,
          provider: selectedModel.provider,
        });
      }

      const currentThinking = storedThinkingLevel || settings.defaultThinkingLevel || "off";
      if (effectiveThinkingLevel !== currentThinking) {
        await apiClient.post(`/api/sessions/${sessionId}/set-thinking-level`, {
          level: effectiveThinkingLevel,
        });
        setSessionThinkingLevel(sessionId, effectiveThinkingLevel);
      }

      onOpenChange(false);
    } catch {
      toast.error("Failed to update session settings");
    } finally {
      setSaving(false);
    }
  };

  const handleOpenChange = (newOpen: boolean) => {
    onOpenChange(newOpen);
    if (newOpen) {
      setLocalModel(sessionModel?.id || settings?.defaultModel || "");
      setLocalThinkingLevel(storedThinkingLevel || settings?.defaultThinkingLevel || "off");
    } else {
      setLocalModel("");
      setLocalThinkingLevel("");
    }
  };

  const currentSelectedModel = settings?.availableModels?.find(m => m.id === effectiveModel);

  const SettingsContent = (
    <>
      {isLoading ? (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <div className="space-y-6 py-4">
          <div className="space-y-2">
            <Label htmlFor="session-model" className="font-heading font-bold text-base">Model</Label>
            <Select value={effectiveModel} onValueChange={setLocalModel}>
              <SelectTrigger id="session-model" className="min-h-[44px]">
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
            {currentSelectedModel && (
              <p className="text-xs text-muted-foreground">
                {currentSelectedModel.provider} • {currentSelectedModel.contextWindow?.toLocaleString()} tokens
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="session-thinking-level" className="font-heading font-bold text-base">Thinking Level</Label>
            <Select
              value={effectiveThinkingLevel}
              onValueChange={setLocalThinkingLevel}
            >
              <SelectTrigger id="session-thinking-level" className="min-h-[44px]">
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
              Changes apply to this session only.
            </p>
          </div>

          <div className="flex gap-2 justify-end pt-4 sm:flex-row flex-col-reverse">
            <Button variant="outline" onClick={() => onOpenChange(false)} className="sm:w-auto w-full min-h-[44px]">
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={saving} className="sm:w-auto w-full min-h-[44px]">
              {saving ? "Saving..." : "Apply"}
            </Button>
          </div>
        </div>
      )}
    </>
  );

  if (isMobile) {
    return (
      <Sheet open={open} onOpenChange={handleOpenChange}>
        <SheetContent
          side="bottom"
          className="h-[60vh] rounded-t-2xl p-0"
          {...sheetSwipeProps}
        >
          <div className="w-12 h-1.5 bg-muted-foreground/30 rounded-full mx-auto mt-3 mb-2" />
          <div className="px-6 pb-6 h-[calc(60vh-2rem)] overflow-y-auto">
            <SheetHeader className="text-left pb-4">
              <SheetTitle className="text-xl">Session Settings</SheetTitle>
              <SheetDescription>
                Change model and thinking level for this session.
              </SheetDescription>
            </SheetHeader>
            {SettingsContent}
          </div>
        </SheetContent>
      </Sheet>
    );
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Session Settings</DialogTitle>
          <DialogDescription>
            Change model and thinking level for this session.
          </DialogDescription>
        </DialogHeader>
        {SettingsContent}
      </DialogContent>
    </Dialog>
  );
}
