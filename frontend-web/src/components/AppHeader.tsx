import { Menu, Square, Wifi, WifiOff, Loader2 } from "lucide-react";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";

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
          className="md:hidden min-w-[44px] min-h-[44px]"
          onClick={onMenuToggle}
          id="session-list-button"
          data-testid="session-list-button"
        >
          <Menu className="h-5 w-5" />
          <span className="sr-only">Toggle menu</span>
        </Button>
        <h1 className="text-xl font-heading font-bold">Pika</h1>
      </div>

      {/* Right: Connection status + Stop button */}
      <div className="flex items-center gap-3">
        {/* Connection status indicator */}
        <Badge
          variant="outline"
          className={
            connectionStatus === "connected"
              ? "border-green-500 text-green-700 dark:text-green-400"
              : connectionStatus === "connecting"
                ? "border-yellow-500 text-yellow-700 dark:text-yellow-400"
                : "border-red-500 text-red-700 dark:text-red-400"
          }
        >
          {connectionStatus === "connected" ? (
            <>
              <Wifi className="mr-1 h-3 w-3" />
              Connected
            </>
          ) : connectionStatus === "connecting" ? (
            <>
              <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              Connecting...
            </>
          ) : (
            <>
              <WifiOff className="mr-1 h-3 w-3" />
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
            className="gap-1 min-h-[44px]"
            id="stop-session-button"
            data-testid="stop-session-button"
          >
            <Square className="h-3 w-3 fill-current" />
            Stop Session
          </Button>
        )}
      </div>
    </header>
  );
}
