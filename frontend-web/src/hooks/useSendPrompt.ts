import { useMutation } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError } from "../lib/toast";

interface SendPromptVariables {
  sessionId: string;
  prompt: string;
}

interface SendPromptResponse {
  message: string;
}

export function useSendPrompt() {
  return useMutation<SendPromptResponse, Error, SendPromptVariables>({
    mutationFn: ({ sessionId, prompt }) =>
      apiClient.post<SendPromptResponse>(`/sessions/${sessionId}/prompt`, { prompt }),
    onError: (error) => {
      showError("Failed to send message", error);
    },
  });
}
