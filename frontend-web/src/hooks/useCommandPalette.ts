import { useState, useCallback, useEffect, useRef } from "react";

interface UseCommandPaletteOptions {
  onOpen?: () => void;
  onClose?: () => void;
}

export function useCommandPalette(options: UseCommandPaletteOptions = {}) {
  const [isOpen, setIsOpen] = useState(false);
  const { onOpen, onClose } = options;
  const onOpenRef = useRef(onOpen);
  const onCloseRef = useRef(onClose);

  // Keep refs up to date
  useEffect(() => {
    onOpenRef.current = onOpen;
    onCloseRef.current = onClose;
  }, [onOpen, onClose]);

  const open = useCallback(() => {
    setIsOpen(true);
    onOpenRef.current?.();
  }, []);

  const close = useCallback(() => {
    setIsOpen(false);
    onCloseRef.current?.();
  }, []);

  const toggle = useCallback(() => {
    setIsOpen((prev) => {
      const next = !prev;
      if (next) {
        onOpenRef.current?.();
      } else {
        onCloseRef.current?.();
      }
      return next;
    });
  }, []);

  // Register keyboard shortcut (Cmd+K or Ctrl+K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+K or Ctrl+K
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        toggle();
      }
      // Escape to close
      if (e.key === "Escape" && isOpen) {
        e.preventDefault();
        close();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, toggle, close]);

  return {
    isOpen,
    open,
    close,
    toggle,
  };
}

// Hook for keyboard navigation in lists
interface UseKeyboardNavigationOptions<T> {
  items: T[];
  onSelect: (item: T) => void;
  getItemId: (item: T) => string;
  enabled?: boolean;
}

export function useKeyboardNavigation<T>(options: UseKeyboardNavigationOptions<T>) {
  const { items, onSelect, enabled = true } = options;
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    if (!enabled) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (items.length === 0) return;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((prev) => (prev + 1) % items.length);
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((prev) => (prev - 1 + items.length) % items.length);
          break;
        case "Enter":
          e.preventDefault();
          if (items[selectedIndex]) {
            onSelect(items[selectedIndex]);
          }
          break;
        case "Home":
          e.preventDefault();
          setSelectedIndex(0);
          break;
        case "End":
          e.preventDefault();
          setSelectedIndex(items.length - 1);
          break;
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [items, onSelect, selectedIndex, enabled]);

  // Reset selection when items change
  useEffect(() => {
    setSelectedIndex(0);
  }, [items.length]);

  return {
    selectedIndex,
    setSelectedIndex,
    selectedItem: items[selectedIndex],
  };
}

// Hook for session switching shortcuts (Cmd+[ and Cmd+])
export function useSessionSwitchingShortcuts(
  sessions: { id: string }[],
  currentSessionId: string | null,
  onSwitch: (sessionId: string) => void
) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!sessions.length) return;

      const currentIndex = sessions.findIndex((s) => s.id === currentSessionId);
      if (currentIndex === -1) return;

      // Cmd+[ or Cmd+Shift+Tab - previous session
      if ((e.metaKey || e.ctrlKey) && (e.key === "[" || (e.shiftKey && e.key === "Tab"))) {
        e.preventDefault();
        const prevIndex = currentIndex === 0 ? sessions.length - 1 : currentIndex - 1;
        onSwitch(sessions[prevIndex].id);
      }

      // Cmd+] or Cmd+Tab - next session
      if ((e.metaKey || e.ctrlKey) && (e.key === "]" || (e.key === "Tab" && !e.shiftKey))) {
        // Only trigger if not part of Cmd+Shift+Tab
        if (e.key === "Tab" && e.shiftKey) return;
        e.preventDefault();
        const nextIndex = currentIndex === sessions.length - 1 ? 0 : currentIndex + 1;
        onSwitch(sessions[nextIndex].id);
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [sessions, currentSessionId, onSwitch]);
}
