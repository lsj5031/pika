import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "./ui/sheet";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Settings, Loader2 } from "lucide-react";
import { usePiSettings } from "../hooks/usePiSettings";
import { useUpdatePiSettings } from "../hooks/useUpdatePiSettings";
import { useSwipeToClose } from "../hooks/useSwipe";
import { ProjectManager } from "./ProjectManager";
import { useAppStore } from "../store/appStore";

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

interface SettingsDialogProps {
  trigger?: React.ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function SettingsDialog({ trigger, open: controlledOpen, onOpenChange: setControlledOpen }: SettingsDialogProps) {
  const needsAuth = useAppStore((state) => state.needsAuth);
  const [internalOpen, setInternalOpen] = useState(false);
  const isMobile = useIsMobile();
  
  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;
  const setOpen = isControlled ? setControlledOpen! : setInternalOpen;

  const { data: settings, isLoading } = usePiSettings(!needsAuth);
  const updateSettingsMutation = useUpdatePiSettings();

  const [localModel, setLocalModel] = useState<string>("");
  const [localThinkingLevel, setLocalThinkingLevel] = useState<string>("off");

  const { swipeProps: sheetSwipeProps } = useSwipeToClose(
    () => setOpen(false),
    { direction: "down", threshold: 80 }
  );

  const handleSave = () => {
    if (!settings) return; // Guard against saving before settings load
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

  const SettingsContent = (
    <Tabs defaultValue="ai" className="w-full">
      <TabsList className={isMobile ? "grid w-full grid-cols-2 h-12" : "grid w-full grid-cols-2"}>
        <TabsTrigger value="ai" className={isMobile ? "text-base py-3" : ""}>AI Settings</TabsTrigger>
        <TabsTrigger value="projects" className={isMobile ? "text-base py-3" : ""}>Projects</TabsTrigger>
      </TabsList>

      <TabsContent value="ai" className="space-y-4">
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <div className="space-y-6 py-4">
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

            <div className="flex gap-2 justify-end pt-4">
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
          </div>
        )}
      </TabsContent>

      <TabsContent value="projects" className="space-y-4">
        <ProjectManager mode="inline" />
      </TabsContent>
    </Tabs>
  );

  if (isMobile) {
    return (
      <Sheet open={open} onOpenChange={handleOpenChange}>
        <SheetTrigger asChild>
          {trigger || defaultTrigger}
        </SheetTrigger>
        <SheetContent
          side="bottom"
          className="h-[85vh] rounded-t-2xl p-0"
          {...sheetSwipeProps}
        >
          <div className="w-12 h-1.5 bg-muted-foreground/30 rounded-full mx-auto mt-3 mb-2" />
          <div className="px-6 pb-6 h-[calc(85vh-2rem)] overflow-y-auto">
            <SheetHeader className="text-left pb-4">
              <SheetTitle className="text-xl">Settings</SheetTitle>
              <SheetDescription>
                Configure AI agent settings and manage projects.
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
      <DialogTrigger asChild>
        {trigger || defaultTrigger}
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>
            Configure AI agent settings and manage projects.
          </DialogDescription>
        </DialogHeader>
        {SettingsContent}
      </DialogContent>
    </Dialog>
  );
}
