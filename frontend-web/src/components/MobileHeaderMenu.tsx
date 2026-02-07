import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Button } from "./ui/button";
import { MoreVertical, Plus, Command, Settings, Sun, Moon } from "lucide-react";
import { useTheme } from "next-themes";

interface MobileHeaderMenuProps {
  onNewSession: () => void;
  onSwitchSession?: () => void;
  onOpenSettings: () => void;
}

export function MobileHeaderMenu({
  onNewSession,
  onSwitchSession,
  onOpenSettings,
}: MobileHeaderMenuProps) {
  const { theme, setTheme } = useTheme();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="md:hidden">
          <MoreVertical className="h-5 w-5" />
          <span className="sr-only">Menu</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuItem onSelect={onNewSession} className="gap-2">
          <Plus className="h-4 w-4" />
          <span>New Session</span>
        </DropdownMenuItem>
        
        {onSwitchSession && (
          <DropdownMenuItem onSelect={onSwitchSession} className="gap-2">
            <Command className="h-4 w-4" />
            <span>Switch Session</span>
          </DropdownMenuItem>
        )}

        <DropdownMenuSeparator />

        <DropdownMenuItem onSelect={onOpenSettings} className="gap-2">
          <Settings className="h-4 w-4" />
          <span>Settings</span>
        </DropdownMenuItem>

        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault(); // Keep menu open to toggle theme? No, let it close.
            setTheme(theme === "dark" ? "light" : "dark");
          }}
          className="gap-2"
        >
          {theme === "dark" ? (
            <Sun className="h-4 w-4" />
          ) : (
            <Moon className="h-4 w-4" />
          )}
          <span>Toggle Theme</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
