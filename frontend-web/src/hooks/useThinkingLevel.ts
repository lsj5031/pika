import { useCallback } from "react";
import { apiClient } from "../lib/api";
import { useAppStore } from "../store/appStore";
import { usePiSettings } from "./usePiSettings";

const THINKING_LEVELS = ["off", "minimal", "low", "medium", "high", "xhigh"] as const;
export type ThinkingLevel = (typeof THINKING_LEVELS)[number];

export function useThinkingLevel(sessionId: string | null) {
  const needsAuth = useAppStore((state) => state.needsAuth);
  const { data: settings } = usePiSettings(!needsAuth);
  const storedLevel = useAppStore((state) =>
    sessionId ? state.sessionThinkingLevels[sessionId] : null
  );
  const setSessionThinkingLevel = useAppStore((state) => state.setSessionThinkingLevel);

  const currentLevel: ThinkingLevel =
    (storedLevel as ThinkingLevel) ||
    (settings?.defaultThinkingLevel as ThinkingLevel) ||
    "off";

  const cycleLevel = useCallback(() => {
    if (!sessionId) return;

    const idx = THINKING_LEVELS.indexOf(currentLevel);
    const next = THINKING_LEVELS[(idx + 1) % THINKING_LEVELS.length];

    setSessionThinkingLevel(sessionId, next);

    apiClient
      .post(`/api/sessions/${sessionId}/set-thinking-level`, { level: next })
      .catch(() => {});
  }, [sessionId, currentLevel, setSessionThinkingLevel]);

  return { currentLevel, cycleLevel, levels: THINKING_LEVELS };
}
