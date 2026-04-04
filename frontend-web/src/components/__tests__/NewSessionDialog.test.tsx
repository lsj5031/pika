import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { NewSessionDialog } from "../NewSessionDialog";

const mockMutate = vi.fn();

// Return a stable mutation object so the component uses mockMutate
const mockStandaloneMutation = {
  mutate: mockMutate,
  isPending: false,
  isSuccess: false,
  isError: false,
  data: null,
  error: null,
};

const mockProjectMutation = {
  mutate: vi.fn(),
  isPending: false,
};

vi.mock("../../hooks/useCreateStandaloneSession", () => ({
  useCreateStandaloneSession: () => mockStandaloneMutation,
}));

vi.mock("../../hooks/useCreateSession", () => ({
  useCreateSession: () => mockProjectMutation,
}));

vi.mock("../../hooks/useProjects", () => ({
  useProjects: () => ({
    data: [],
    isLoading: false,
  }),
}));

vi.mock("../../store/appStore", () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      currentSessionId: null,
      needsAuth: false,
      activeSessionIds: new Set(),
      thinkingSessionIds: new Set(),
      unreadSessions: new Set(),
      lastSeenMessageCounts: {},
      lastProjectId: null,
      recentSessionIds: [],
      favoriteSessionIds: [],
      sessionThinkingLevels: {},
      setCurrentSession: vi.fn(),
      setLastProject: vi.fn(),
    }),
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return function TestWrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
};

describe("NewSessionDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls createStandaloneMutation.mutate with path='~' directly when Quick Start is clicked (no stale state)", async () => {
    const user = userEvent.setup();

    render(<NewSessionDialog open={true} onOpenChange={vi.fn()} />, {
      wrapper: createWrapper(),
    });

    // Switch to "Any Folder" mode to reveal the Quick Start button
    const anyFolderButton = screen.getByText("Any Folder");
    await user.click(anyFolderButton);

    // Find and click the Quick Start (Home) button
    const quickStartButton = screen.getByText("Quick Start (Home)");
    await user.click(quickStartButton);

    // The mutation should have been called with path="~" directly
    // NOT via setTimeout with stale state
    expect(mockMutate).toHaveBeenCalledTimes(1);
    const firstCallArgs = mockMutate.mock.calls[0];
    expect(firstCallArgs[0]).toEqual(
      expect.objectContaining({
        path: "~",
      })
    );
  });
});
