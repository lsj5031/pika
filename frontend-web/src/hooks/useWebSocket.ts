import { useCallback, useEffect, useRef, useState } from "react";
import { config } from "../config/env";
import { getCredentials, encodeBasicAuth, clearCredentials } from "../lib/auth";
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
  const connectionStatusRef = useRef<ConnectionStatus>(
    enabled ? "connecting" : "disconnected"
  );
  const createConnectionRef = useRef<(() => void) | null>(null);

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

  // Keep ref in sync with state
  useEffect(() => {
    connectionStatusRef.current = connectionStatus;
  }, [connectionStatus]);

  /**
   * Build WebSocket URL with auth query params
   * WebSocket doesn't support custom headers, so we use query params for auth
   */
  const buildWsUrl = useCallback(() => {
    const credentials = getCredentials();
    const baseUrl = config.WS_URL;

    if (credentials) {
      // Use subprotocol or query param for auth
      // Query param approach: append encoded credentials
      const url = new URL(baseUrl);
      url.searchParams.set("auth", encodeBasicAuth(credentials));
      return url.toString();
    }

    return baseUrl;
  }, []);

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
      const wsUrl = buildWsUrl();
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnectionStatus("connected");
        reconnectAttemptsRef.current = 0;
        hasConnectedOnceRef.current = true;
      };

      ws.onclose = (event) => {
        setConnectionStatus("disconnected");
        wsRef.current = null;

        // Check for auth failure (code 4401 or close reason contains "unauthorized")
        if (event.code === 4401 || event.reason?.toLowerCase().includes("unauthorized")) {
          clearCredentials();
          window.dispatchEvent(new CustomEvent(AUTH_ERROR_EVENT));
          return; // Don't reconnect on auth failure
        }

        // Show error on unexpected disconnect if we've connected before
        if (hasConnectedOnceRef.current && !event.wasClean) {
          onErrorRef.current?.(event);
        }

        // Attempt to reconnect with exponential backoff if not explicitly closed
        if (!event.wasClean && enabledRef.current) {
          const delay = Math.min(
            1000 * Math.pow(2, reconnectAttemptsRef.current),
            maxReconnectDelay
          );
          reconnectAttemptsRef.current++;

          reconnectTimeoutRef.current = setTimeout(() => {
            createConnectionRef.current?.();
          }, delay);
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
  }, [buildWsUrl, setConnectionStatus]);

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
      // eslint-disable-next-line react-hooks/set-state-in-effect
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
