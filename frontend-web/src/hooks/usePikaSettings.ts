import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export interface PikaModel {
  id: string;
  name: string;
  reasoning: boolean;
  input: string[];
  contextWindow: number;
  maxTokens: number;
  provider: string;
}

export interface PikaSettings {
  defaultProvider?: string;
  defaultModel?: string;
  defaultThinkingLevel?: string;
  theme?: string;
  hideThinkingBlock?: boolean;
  availableModels?: PikaModel[];
}

export function usePikaSettings(enabled = true) {
  return useQuery<PikaSettings>({
    queryKey: ["pika-settings"],
    queryFn: async () => {
      return apiClient.get<PikaSettings>("/api/settings");
    },
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
