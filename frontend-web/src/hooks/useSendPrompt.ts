import { useMutation } from "@tanstack/react-query";
import { apiClient } from "../lib/api";
import { showError } from "../lib/toast";
import type { ImageUploadRequest } from "../types/api";

interface SendPromptVariables {
  sessionId: string;
  prompt: string;
  images?: ImageUploadRequest[];
}

interface SendPromptResponse {
  message: string;
}

export function useSendPrompt() {
  return useMutation<SendPromptResponse, Error, SendPromptVariables>({
    mutationFn: ({ sessionId, prompt, images }) =>
      apiClient.post<SendPromptResponse>(`/api/sessions/${sessionId}/prompt`, {
        prompt,
        images,
      }),
    onError: (error) => {
      showError("Failed to send message", error);
    },
  });
}
