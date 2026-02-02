import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../lib/api";

export interface PiModel {
  id: string;
  name: string;
  reasoning: boolean;
  input: string[];
  contextWindow: number;
  maxTokens: number;
  provider: string;
}

export interface PiSettings {
  defaultProvider?: string;
  defaultModel?: string;
  defaultThinkingLevel?: string;
  theme?: string;
  hideThinkingBlock?: boolean;
  availableModels?: PiModel[];
}

export function usePiSettings(enabled = true) {
  return useQuery<PiSettings>({
    queryKey: ["pi-settings"],
    queryFn: async () => {
      return apiClient.get<PiSettings>("/api/settings");
    },
    enabled,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
