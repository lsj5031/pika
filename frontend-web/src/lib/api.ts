import { config } from "../config/env";
import { getCredentials, getAuthHeader, clearCredentials } from "./auth";

// Event for auth failures
export const AUTH_ERROR_EVENT = "auth-error";

// Dispatch auth error event
function dispatchAuthError() {
  window.dispatchEvent(new CustomEvent(AUTH_ERROR_EVENT));
}

// API client using fetch with auth support
export const apiClient = {
  async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${config.API_URL}${endpoint}`;

    // Get auth header if credentials exist
    const credentials = getCredentials();
    const authHeader = getAuthHeader(credentials);

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      ...options.headers as Record<string, string>,
    };

    if (authHeader) {
      headers["Authorization"] = authHeader;
    }

    const response = await fetch(url, {
      ...options,
      headers,
    });

    // Handle 401 Unauthorized - trigger auth flow
    if (response.status === 401) {
      clearCredentials();
      dispatchAuthError();
      throw new Error("Authentication required");
    }

    if (!response.ok) {
      let errorMessage = `API error: ${response.status}`;
      try {
        const error: { error: string; message: string } = await response.json();
        errorMessage = error.message || errorMessage;
      } catch {
        // Could not parse error response
      }
      throw new Error(errorMessage);
    }

    return response.json();
  },

  get<T>(endpoint: string): Promise<T> {
    return this.request<T>(endpoint, { method: "GET" });
  },

  post<T>(endpoint: string, data: unknown): Promise<T> {
    return this.request<T>(endpoint, {
      method: "POST",
      body: JSON.stringify(data),
    });
  },

  delete<T>(endpoint: string): Promise<T> {
    return this.request<T>(endpoint, { method: "DELETE" });
  },
};
