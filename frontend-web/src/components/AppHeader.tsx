import { Menu, Square, Wifi, WifiOff, Loader2 } from "lucide-react";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { SettingsDialog } from "./SettingsDialog";
import { ThemeToggle } from "./ThemeToggle";
import { cn } from "../lib/utils";

interface AppHeaderProps {
  connectionStatus: "connecting" | "connected" | "disconnected";
  isSessionActive: boolean;
  onMenuToggle: () => void;
  onStopSession?: () => void;
}

export function AppHeader({
  connectionStatus,
  isSessionActive,
  onMenuToggle,
  onStopSession,
}: AppHeaderProps) {
  return (
    <header className="flex h-14 items-center justify-between border-b bg-background px-4 sm:px-6">
      {/* Left: Hamburger menu + App name */}
      <div className="flex items-center gap-3">
        <Button
          variant="ghost"
          size="icon"
          className="md:hidden min-w-[44px] min-h-[44px] touch-manipulation"
          onClick={onMenuToggle}
          id="session-list-button"
          data-testid="session-list-button"
          style={{ touchAction: "manipulation" }}
        >
          <Menu className="h-5 w-5 pointer-events-none" />
          <span className="sr-only">Toggle menu</span>
        </Button>
        <h1 className="text-xl md:text-2xl font-heading font-bold tracking-tight whitespace-nowrap">
          Pika
        </h1>
      </div>

      {/* Right: Connection status + Settings + Stop button */}
      <div className="flex items-center gap-1.5 md:gap-4">
        {/* Theme toggle */}
        <ThemeToggle />

        {/* Settings dialog */}
        <SettingsDialog />

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
