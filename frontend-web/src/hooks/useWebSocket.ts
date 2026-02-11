import { useCallback, useEffect, useRef, useState } from "react";
import { config } from "../config/env";
import { clearAuthState } from "../lib/auth";
import { AUTH_ERROR_EVENT } from "../lib/api";
import type { WSEvent } from "../types";

type ConnectionStatus = "connecting" | "connected" | "disconnected";

interface UseWebSocketOptions {
  onMessage?: (event: WSEvent) => void;
  onError?: (error: Event) => void;
  enabled?: boolean;
}

export function useWebSocket({ onMessage, onError, enabled = true }: UseWebSocketOptions = {}) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const maxReconnectDelay = 30000; // 30 seconds max
  const onMessageRef = useRef(onMessage);
  const onErrorRef = useRef(onError);
  const hasConnectedOnceRef = useRef(false);
  const enabledRef = useRef(enabled);
  const createConnectionRef = useRef<(() => void) | null>(null);
  const authCheckInFlightRef = useRef(false);

  // Keep onMessageRef updated
  useEffect(() => {
    onMessageRef.current = onMessage;
  }, [onMessage]);

  // Keep onErrorRef updated
  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  // Track enabled state
  useEffect(() => {
    enabledRef.current = enabled;
  }, [enabled]);

  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>(
    enabled ? "connecting" : "disconnected"
  );

  // Function to create WebSocket connection
  const createConnection = useCallback(() => {
    if (!enabledRef.current) return;

    // Clear any existing reconnect timeout
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Don't reconnect if already connected
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      setConnectionStatus("connected");
      return;
    }

    setConnectionStatus("connecting");

    try {
      const ws = new WebSocket(config.WS_URL);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnectionStatus("connected");
        reconnectAttemptsRef.current = 0;
        hasConnectedOnceRef.current = true;
      };

      ws.onclose = (event) => {
        setConnectionStatus("disconnected");
        wsRef.current = null;

        const triggerAuthFailure = () => {
          clearAuthState();
          window.dispatchEvent(new CustomEvent(AUTH_ERROR_EVENT));
        };

        const scheduleReconnect = () => {
          const delay = Math.min(
            1000 * Math.pow(2, reconnectAttemptsRef.current),
            maxReconnectDelay
          );
          reconnectAttemptsRef.current++;

          reconnectTimeoutRef.current = setTimeout(() => {
            createConnectionRef.current?.();
          }, delay);
        };

        const checkAuthStatus = async (): Promise<boolean> => {
          if (authCheckInFlightRef.current) {
            return false;
          }

          authCheckInFlightRef.current = true;
          try {
            const response = await fetch(`${config.API_URL}/api/auth/status`, {
              credentials: "include",
            });

            if (response.status === 401) {
              triggerAuthFailure();
              return true;
            }

            if (!response.ok) {
              return false;
            }

            const data = (await response.json()) as {
              enabled: boolean;
              authenticated: boolean;
            };

            if (data.enabled && !data.authenticated) {
              triggerAuthFailure();
              return true;
            }
          } catch {
            // Network failures should continue retry behavior.
          } finally {
            authCheckInFlightRef.current = false;
          }

          return false;
        };

        // Check for explicit auth failure
        if (event.code === 4401 || event.reason?.toLowerCase().includes("unauthorized")) {
          triggerAuthFailure();
          return;
        }

        // Show error on unexpected disconnect if we've connected before
        if (hasConnectedOnceRef.current && !event.wasClean) {
          onErrorRef.current?.(event);
        }

        // Attempt to reconnect with exponential backoff if not explicitly closed.
        // For repeated failed handshakes (e.g. browser surfaces 1006), probe auth status.
        if (!event.wasClean && enabledRef.current) {
          const shouldProbeAuth = !hasConnectedOnceRef.current || reconnectAttemptsRef.current >= 2;

          if (shouldProbeAuth) {
            void checkAuthStatus().then((isAuthFailure) => {
              if (!isAuthFailure && enabledRef.current) {
                scheduleReconnect();
              }
            });
            return;
          }

          scheduleReconnect();
        }
      };

      ws.onerror = (error) => {
        console.error("WebSocket error:", error);
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as WSEvent;
          onMessageRef.current?.(data);
        } catch (error) {
          console.error("Failed to parse WebSocket message:", error);
        }
      };
    } catch (error) {
      console.error("Failed to create WebSocket connection:", error);
      setConnectionStatus("disconnected");
    }
  }, [setConnectionStatus]);

  // Keep ref updated with latest createConnection
  useEffect(() => {
    createConnectionRef.current = createConnection;
  }, [createConnection]);

  // Function to disconnect
  const disconnect = () => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close(1000, "Component unmounted");
      wsRef.current = null;
    }

    setConnectionStatus("disconnected");
  };

  // Connect on mount and when enabled changes
  useEffect(() => {
    if (enabled) {
      createConnection();
    }

    return () => {
      disconnect();
    };
  }, [enabled, createConnection]);

  return {
    connectionStatus,
    isConnected: connectionStatus === "connected",
    isConnecting: connectionStatus === "connecting",
    reconnect: createConnection,
  };
}
