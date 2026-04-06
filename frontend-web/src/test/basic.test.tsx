import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ChatInput } from "../components/ChatInput";

vi.mock("../hooks/usePikaSettings", () => ({
  usePikaSettings: () => ({
    data: {
      defaultModel: "gpt-test",
      availableModels: [
        {
          id: "gpt-test",
          name: "GPT Test",
          provider: "test-provider",
        },
      ],
    },
    isLoading: false,
  }),
}));

vi.mock("../hooks/useThinkingLevel", () => ({
  useThinkingLevel: () => ({
    currentLevel: "off",
    cycleLevel: vi.fn(),
  }),
}));

vi.mock("../store/appStore", () => ({
  useAppStore: (selector: (state: { needsAuth: boolean; sessionModels: Record<string, unknown>; sessionThinkingLevels: Record<string, string> }) => unknown) =>
    selector({ needsAuth: false, sessionModels: {}, sessionThinkingLevels: {} }),
}));

describe("ChatInput", () => {
  it("disables input actions when no session is selected", () => {
    render(<ChatInput sessionId={null} onSendMessage={vi.fn()} />);

    expect(screen.getByPlaceholderText("Select a session")).toBeDisabled();
    expect(screen.getByTestId("send-button")).toBeDisabled();
  });

  it("sends a message on Enter for an active session", async () => {
    const onSendMessage = vi.fn();
    const user = userEvent.setup();

    render(<ChatInput sessionId="session-1" onSendMessage={onSendMessage} />);

    const input = screen.getByTestId("chat-input");
    await user.type(input, "hello from test{enter}");

    expect(onSendMessage).toHaveBeenCalledTimes(1);
    expect(onSendMessage).toHaveBeenCalledWith("hello from test", []);
  });
});
