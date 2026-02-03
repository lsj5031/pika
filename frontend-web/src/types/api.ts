// Project types
export interface Project {
  id: string;
  path: string;
  name: string;
  session_count: number;
}

// Session types
export interface Session {
  id: string;
  name: string;
  project_id: string;
  project_path: string;
  created_at: string;
  is_active: boolean;
}

export interface StartSessionResponse {
  process_id: string;
  newly_spawned: boolean;
}

export interface SessionStatus {
  session_id: string;
  is_running: boolean;
  process_id: string | null;
}

export interface StopSessionResponse {
  session_id: string;
  process_id: string | null;
  was_running: boolean;
}

export interface CreateSessionRequest {
  name?: string;
}

export interface CreateSessionResponse {
  session_id: string;
  name: string;
  project_id: string;
  project_path: string;
  created_at: string;
  newly_spawned: boolean;
  process_id: string | null;
}

// Message types
export interface Message {
  role: "user" | "assistant";
  content: string;
  timestamp: string | null;
}

// Code diff types
export interface CodeDiff {
  id: string;
  sessionId: string;
  messageId: string;
  oldCode: string;
  newCode: string;
  language: string;
  filePath: string;
  createdAt: string;
}

// API request types
export interface PromptRequest {
  prompt: string;
}

// Error types
export interface ErrorResponse {
  error: string;
  message: string;
}

// WebSocket event types - uses internally tagged enum format
export type WSEvent =
  | { type: "SessionStarted"; data: { session_id: string; project_path: string } }
  | { type: "SessionStopped"; data: { session_id: string } }
  | { type: "ThinkingDelta"; data: { session_id: string; content: string } }
  | { type: "MessageAdded"; data: { session_id: string; role: string; content: string; timestamp: string } };

// Helper type for WebSocket event data extraction
export type WSEventData = WSEvent extends { data: infer D } ? D : never;
