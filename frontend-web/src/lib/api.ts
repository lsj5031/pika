import { config } from "../config/env";
import { clearAuthState } from "./auth";

// Event for auth failures
export const AUTH_ERROR_EVENT = "auth-error";

// Dispatch auth error event
function dispatchAuthError() {
  window.dispatchEvent(new CustomEvent(AUTH_ERROR_EVENT));
}

// API client using fetch with cookie auth support
export const apiClient = {
  async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${config.API_URL}${endpoint}`;

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      ...(options.headers as Record<string, string>),
    };

    const response = await fetch(url, {
      ...options,
      headers,
      credentials: "include",
    });

    // Handle 401 Unauthorized - trigger auth flow
    if (response.status === 401) {
      clearAuthState();
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

      // Special handling for 404 on session messages - session may have been deleted
      if (response.status === 404 && endpoint.includes("/messages")) {
        // Extract session ID from endpoint
        const match = endpoint.match(/\/api\/sessions\/([^/]+)\/messages/);
        if (match) {
          const sessionId = match[1];
          // Dispatch event to clear invalid session
          window.dispatchEvent(new CustomEvent("session-not-found", { detail: { sessionId } }));
        }
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
