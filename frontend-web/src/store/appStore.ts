import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AppState {
  // State
  currentSessionId: string | null;
  sidebarCollapsed: boolean;

  // Actions
  setCurrentSession: (sessionId: string | null) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Initial state
      currentSessionId: null,
      sidebarCollapsed: false,

      // Actions
      setCurrentSession: (sessionId) => set({ currentSessionId: sessionId }),

      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
    }),
    {
      name: "pi-agent-manager-storage", // localStorage key
    }
  )
);
