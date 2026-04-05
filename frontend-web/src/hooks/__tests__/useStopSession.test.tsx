import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useStopSession } from "../useStopSession";

const mockPost = vi.fn();

vi.mock("../../lib/api", () => ({
  apiClient: {
    post: (...args: unknown[]) => mockPost(...args),
  },
}));

const { showError: mockShowError } = vi.hoisted(() => ({
  showError: vi.fn(),
}));

vi.mock("../../lib/toast", () => ({
  showError: mockShowError,
  showSuccess: vi.fn(),
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      mutations: {
        retry: false,
      },
    },
  });

  return function TestWrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );
  };
};

describe("useStopSession", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows error toast when stop session fails", async () => {
    mockPost.mockRejectedValue(new Error("Session not found"));

    const { result } = renderHook(() => useStopSession(), {
      wrapper: createWrapper(),
    });

    result.current.mutate("session-123");

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(mockShowError).toHaveBeenCalledWith(
      "Failed to stop session",
      expect.any(Error)
    );
  });
});
