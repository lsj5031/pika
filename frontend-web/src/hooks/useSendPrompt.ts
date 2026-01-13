import { useMutation } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

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
  });
}
