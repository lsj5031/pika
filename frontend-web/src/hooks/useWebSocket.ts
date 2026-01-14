import { useEffect, useRef, useState } from "react";
import { config } from "../config/env";
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

  // Function to create WebSocket connection
  const createConnection = () => {
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
            createConnection();
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
  };

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
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (enabled) {
      createConnection();
    }

    return () => {
      disconnect();
    };
  }, [enabled]);

  return {
    connectionStatus,
    isConnected: connectionStatus === "connected",
    isConnecting: connectionStatus === "connecting",
  };
}
