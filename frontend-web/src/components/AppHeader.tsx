import { Menu, Square, Wifi, WifiOff, Loader2 } from "lucide-react";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
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
        <h1 className="text-2xl font-heading font-bold tracking-tight">PI Agent Manager</h1>
      </div>

      {/* Right: Connection status + Stop button */}
      <div className="flex items-center gap-4">
        {/* Connection status indicator */}
        <Badge
          variant="outline"
          className={cn(
            "border-2 font-heading font-bold px-3 py-1 rounded-wobblyMd shadow-sm transition-all",
            connectionStatus === "connected"
              ? "border-green-500/50 text-green-700 bg-green-50 shadow-green-100"
              : connectionStatus === "connecting"
                ? "border-yellow-500/50 text-yellow-700 bg-yellow-50 shadow-yellow-100"
                : "border-red-500/50 text-red-700 bg-red-50 shadow-red-100"
          )}
        >
          {connectionStatus === "connected" ? (
            <>
              <Wifi className="mr-1.5 h-3.5 w-3.5" />
              Connected
            </>
          ) : connectionStatus === "connecting" ? (
            <>
              <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              Connecting...
            </>
          ) : (
            <>
              <WifiOff className="mr-1.5 h-3.5 w-3.5" />
              Disconnected
            </>
          )}
        </Badge>

        {/* Stop session button (only shown when session is active) */}
        {isSessionActive && onStopSession && (
          <Button
            variant="destructive"
            size="sm"
            onClick={onStopSession}
            className="gap-2 min-h-[40px] rounded-wobblyMd border-2 shadow-hard-sm font-heading font-bold"
            id="stop-session-button"
            data-testid="stop-session-button"
          >
            <Square className="h-3.5 w-3.5 fill-current" />
            Stop Session
          </Button>
        )}
      </div>
    </header>
  );
}
