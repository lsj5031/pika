import { create } from "zustand";

interface ThinkingState {
  content: string;
  isThinking: boolean;
}

interface DeltaBuffer {
  content: string;
  timeout: ReturnType<typeof setTimeout> | null;
  lastFlush: number;
}

interface ThinkingStore {
  thinkingBySession: Record<string, ThinkingState>;
  // Stored in store state to avoid module-level state persisting across HMR
  deltaBuffers: Map<string, DeltaBuffer>;
  setThinking: (sessionId: string, content: string, isThinking: boolean) => void;
  appendThinking: (sessionId: string, content: string) => void;
  clearThinking: (sessionId: string) => void;
  getThinking: (sessionId: string) => ThinkingState;
}

const BATCH_MS = 50; // Throttle interval - ensures regular UI updates
const MAX_BUFFER_MS = 250; // Safety cap - prevents unbounded buffering during streaming

export const useThinkingStore = create<ThinkingStore>((set, get) => ({
  thinkingBySession: {},
  deltaBuffers: new Map<string, DeltaBuffer>(),

  setThinking: (sessionId, content, isThinking) =>
    set((state) => ({
      thinkingBySession: {
        ...state.thinkingBySession,
        [sessionId]: { content, isThinking },
      },
    })),

  appendThinking: (sessionId, content) => {
    const state = get();
    const existing = state.deltaBuffers.get(sessionId);
    const now = Date.now();

    if (existing) {
      existing.content += content;

      // Throttle: flush based on time since last flush, not just timeout reset
      // This ensures the UI updates regularly during streaming instead of waiting for pauses
      if (now - existing.lastFlush >= BATCH_MS) {
        if (existing.timeout) {
          clearTimeout(existing.timeout);
          existing.timeout = null;
        }

        const finalContent = existing.content;
        existing.content = "";
        existing.lastFlush = now;

        set((state) => {
          const current = state.thinkingBySession[sessionId];
          return {
            thinkingBySession: {
              ...state.thinkingBySession,
              [sessionId]: {
                content: (current?.content ?? "") + finalContent,
                isThinking: true,
              },
            },
          };
        });
      }
    } else {
      const newBuffer: DeltaBuffer = {
        content,
        timeout: null,
        lastFlush: now,
      };

      set((state) => {
        const newBuffers = new Map(state.deltaBuffers);
        newBuffers.set(sessionId, newBuffer);
        return { deltaBuffers: newBuffers };
      });

      const current = state.thinkingBySession[sessionId];
      set((state) => ({
        thinkingBySession: {
          ...state.thinkingBySession,
          [sessionId]: {
            content: (current?.content ?? "") + content,
            isThinking: true,
          },
        },
      }));
    }

    const buffer = get().deltaBuffers.get(sessionId);
    if (buffer && !buffer.timeout) {
      buffer.timeout = setTimeout(() => {
        const state = get();
        const currentBuffer = state.deltaBuffers.get(sessionId);
        if (!currentBuffer || currentBuffer.content === "") return;

        const finalContent = currentBuffer.content;
        currentBuffer.content = "";
        currentBuffer.lastFlush = Date.now();
        currentBuffer.timeout = null;

        set((state) => {
          const current = state.thinkingBySession[sessionId];
          return {
            thinkingBySession: {
              ...state.thinkingBySession,
              [sessionId]: {
                content: (current?.content ?? "") + finalContent,
                isThinking: true,
              },
            },
          };
        });
      }, MAX_BUFFER_MS);
    }
  },

  clearThinking: (sessionId) => {
    set((state) => {
      const buffer = state.deltaBuffers.get(sessionId);
      if (buffer?.timeout) {
        clearTimeout(buffer.timeout);
      }

      const newBuffers = new Map(state.deltaBuffers);
      newBuffers.delete(sessionId);

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [sessionId]: _, ...rest } = state.thinkingBySession;
      return {
        thinkingBySession: rest,
        deltaBuffers: newBuffers,
      };
    });
  },

  getThinking: (sessionId) =>
    get().thinkingBySession[sessionId] ?? { content: "", isThinking: false },
}));
