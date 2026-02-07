import { useState } from "react";
import { Square, Wifi, WifiOff, Loader2, Command } from "lucide-react";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { SettingsDialog } from "./SettingsDialog";
import { NewSessionDialog } from "./NewSessionDialog";
import { ThemeToggle } from "./ThemeToggle";
import { MobileHeaderMenu } from "./MobileHeaderMenu";
import { cn } from "../lib/utils";

interface AppHeaderProps {
  connectionStatus: "connecting" | "connected" | "disconnected";
  isSessionActive: boolean;
  onStopSession?: () => void;
  onOpenCommandPalette?: () => void;
}

export function AppHeader({
  connectionStatus,
  isSessionActive,
  onStopSession,
  onOpenCommandPalette,
}: AppHeaderProps) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [newSessionOpen, setNewSessionOpen] = useState(false);

  return (
    <header className="sticky top-0 z-50 flex h-14 items-center justify-between border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 px-4 sm:px-6 safe-top">
      {/* Left: App name */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <img src="/logo.png" alt="Pika Logo" className="h-8 w-8 object-contain" />
          <h1 className="text-xl md:text-2xl font-heading font-bold tracking-tight whitespace-nowrap">
            Pika
          </h1>
        </div>
      </div>

      {/* Right: Connection status + Settings + Stop button */}
      <div className="flex items-center gap-1.5 md:gap-3">
        {/* Desktop Actions */}
        <div className="hidden md:flex items-center gap-3">
          <ThemeToggle />

          {/* Command palette trigger */}
          <Button
            variant="outline"
            size="sm"
            onClick={onOpenCommandPalette}
            className="items-center gap-2 h-9 px-3 text-muted-foreground hover:text-foreground"
            data-testid="command-palette-button"
          >
            <Command className="h-4 w-4" />
            <span className="text-sm">Switch Session</span>
            <kbd className="hidden lg:inline-flex h-5 items-center rounded border bg-muted px-1.5 font-mono text-xs text-muted-foreground">
              ⌘K
            </kbd>
          </Button>

          {/* Settings dialog */}
          <SettingsDialog />
        </div>

        {/* Mobile Menu */}
        <div className="md:hidden">
          <MobileHeaderMenu 
            onNewSession={() => setNewSessionOpen(true)}
            onSwitchSession={onOpenCommandPalette}
            onOpenSettings={() => setSettingsOpen(true)}
          />
          {/* Controlled Dialogs for Mobile Menu */}
          <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} trigger={<span />} />
          <NewSessionDialog open={newSessionOpen} onOpenChange={setNewSessionOpen} trigger={<span />} />
        </div>

        {/* Connection status indicator */}
        <Badge
          variant="outline"
          className={cn(
            "border-2 font-heading font-bold px-2 py-1 md:px-3 rounded-wobblyMd shadow-sm transition-all text-xs md:text-sm",
            connectionStatus === "connected"
              ? "border-success/50 text-success-foreground bg-success/20"
              : connectionStatus === "connecting"
                ? "border-warning/50 text-warning-foreground bg-warning/20"
                : "border-error/50 text-error-foreground bg-error/20"
          )}
        >
          {connectionStatus === "connected" ? (
            <>
              <Wifi className="md:mr-1.5 h-3.5 w-3.5" />
              <span className="hidden sm:inline">Connected</span>
            </>
          ) : connectionStatus === "connecting" ? (
            <>
              <Loader2 className="md:mr-1.5 h-3.5 w-3.5 animate-spin" />
              <span className="hidden sm:inline">Connecting...</span>
            </>
          ) : (
            <>
              <WifiOff className="md:mr-1.5 h-3.5 w-3.5" />
              <span className="hidden sm:inline">Disconnected</span>
            </>
          )}
        </Badge>

        {/* Stop session button (only shown when session is active) */}
        {isSessionActive && onStopSession && (
          <Button
            variant="destructive"
            size="sm"
            onClick={onStopSession}
            className="gap-2 min-h-[40px] px-2 md:px-3 rounded-wobblyMd border-2 shadow-hard-sm font-heading font-bold"
            id="stop-session-button"
            data-testid="stop-session-button"
          >
            <Square className="h-3.5 w-3.5 fill-current" />
            <span className="hidden xs:inline">Stop Session</span>
            <span className="xs:hidden">Stop</span>
          </Button>
        )}
      </div>
    </header>
  );
}
