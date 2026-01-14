import { create } from "zustand";

interface ThinkingState {
  content: string;
  isThinking: boolean;
}

interface ThinkingStore {
  thinkingBySession: Record<string, ThinkingState>;
  setThinking: (sessionId: string, content: string, isThinking: boolean) => void;
  appendThinking: (sessionId: string, content: string) => void;
  clearThinking: (sessionId: string) => void;
  getThinking: (sessionId: string) => ThinkingState;
}

export const useThinkingStore = create<ThinkingStore>((set, get) => ({
  thinkingBySession: {},

  setThinking: (sessionId, content, isThinking) =>
    set((state) => ({
      thinkingBySession: {
        ...state.thinkingBySession,
        [sessionId]: { content, isThinking },
      },
    })),

  appendThinking: (sessionId, content) =>
    set((state) => {
      const current = state.thinkingBySession[sessionId];
      return {
        thinkingBySession: {
          ...state.thinkingBySession,
          [sessionId]: {
            content: (current?.content ?? "") + content,
            isThinking: true,
          },
        },
      };
    }),

  clearThinking: (sessionId) =>
    set((state) => {
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [sessionId]: _, ...rest } = state.thinkingBySession;
      return { thinkingBySession: rest };
    }),

  getThinking: (sessionId) =>
    get().thinkingBySession[sessionId] ?? { content: "", isThinking: false },
}));
