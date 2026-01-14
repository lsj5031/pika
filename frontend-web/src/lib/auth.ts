// Authentication utilities for localStorage and credential management

const AUTH_KEY = "pi-agent-auth";

export interface AuthCredentials {
  username: string;
  password: string;
}

/**
 * Store auth credentials in localStorage
 */
export function storeCredentials(credentials: AuthCredentials): void {
  localStorage.setItem(AUTH_KEY, JSON.stringify(credentials));
}

/**
 * Retrieve auth credentials from localStorage
 */
export function getCredentials(): AuthCredentials | null {
  const stored = localStorage.getItem(AUTH_KEY);
  if (!stored) return null;

  try {
    const parsed = JSON.parse(stored) as AuthCredentials;
    if (parsed.username && parsed.password) {
      return parsed;
    }
    return null;
  } catch {
    return null;
  }
}

/**
 * Clear stored auth credentials
 */
export function clearCredentials(): void {
  localStorage.removeItem(AUTH_KEY);
}

/**
 * Check if credentials are stored
 */
export function hasCredentials(): boolean {
  return getCredentials() !== null;
}

/**
 * Encode credentials to Base64 for Basic Auth header
 */
export function encodeBasicAuth(credentials: AuthCredentials): string {
  const combined = `${credentials.username}:${credentials.password}`;
  return btoa(combined);
}

/**
 * Get the Authorization header value for Basic Auth
 */
export function getAuthHeader(credentials: AuthCredentials | null): string | null {
  if (!credentials) return null;
  return `Basic ${encodeBasicAuth(credentials)}`;
}
