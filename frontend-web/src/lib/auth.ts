// In-memory auth session helpers.
// No credentials are persisted to browser storage.

let authenticatedInMemory = false;

/**
 * Mark auth as established for the current tab lifetime.
 */
export function markAuthenticated(): void {
  authenticatedInMemory = true;
}

/**
 * Clear in-memory auth state.
 */
export function clearAuthState(): void {
  authenticatedInMemory = false;
}

/**
 * Check whether this tab has authenticated in current runtime.
 */
export function hasAuthState(): boolean {
  return authenticatedInMemory;
}
