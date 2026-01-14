# Web Frontend Replacement

## Overview

Replace the Makepad WASM frontend with a modern React + TypeScript + Vite frontend using shadcn/ui components. The new frontend will connect to the existing backend API via WebSocket and HTTP, with no backend changes required beyond static file serving.

## Goals

- Eliminate WASM build complexity and slow development cycle
- Enable fast hot-reload development with standard web tooling
- Maintain feature parity with existing Makepad frontend
- Improve accessibility, error handling, and UX

## User Stories

### US-001: Initialize Vite + React + TypeScript project

**As a** developer, **I want** a Vite project scaffolded with React and TypeScript **so that** I can use standard web development tooling.

**Acceptance Criteria:**
- [ ] `frontend-web/` directory created with `npm create vite@latest`
- [ ] React 19 and TypeScript configured
- [ ] `package.json` includes dependencies: `react`, `react-dom`, `@tanstack/react-query`, `zustand`, `lucide-react`
- [ ] Dev server runs on port 5173 with `npm run dev`
- [ ] TypeScript compiles without errors

### US-002: Configure Tailwind CSS and shadcn/ui

**As a** developer, **I want** Tailwind CSS and shadcn/ui installed **so that** I can use pre-built accessible components.

**Acceptance Criteria:**
- [ ] Tailwind CSS configured with `tailwind.config.js`
- [ ] shadcn/ui initialized with `init` command
- [ ] Light theme as default (per user preference)
- [ ] Global CSS includes shadcn base styles
- [ ] TypeScript compiles without errors

### US-003: Create TypeScript types for API

**As a** developer, **I want** TypeScript types matching the backend API **so that** I have type safety across the frontend.

**Acceptance Criteria:**
- [ ] `types/api.ts` defines `Session`, `Project`, `Message`, `WSEvent` types
- [ ] `WSEvent` type matches backend serde enum (SessionStarted, SessionStopped, ThinkingDelta, MessageAdded)
- [ ] All types export from `types/index.ts`
- [ ] TypeScript compiles without errors

### US-004: Create API client with React Query

**As a** developer, **I want** an API client using React Query **so that** API state is managed efficiently.

**Acceptance Criteria:**
- [ ] `lib/api.ts` creates `apiClient` with base URL from `VITE_API_URL` env var
- [ ] `hooks/useSessions.ts` uses `useQuery` for fetching sessions list
- [ ] `hooks/useProjects.ts` uses `useQuery` for fetching projects list
- [ ] `hooks/useSessionHistory.ts` uses `useQuery` for fetching session messages
- [ ] `hooks/useCreateSession.ts` uses `useMutation` for creating sessions
- [ ] `hooks/useStartSession.ts` uses `useMutation` for starting sessions
- [ ] `hooks/useStopSession.ts` uses `useMutation` for stopping sessions
- [ ] TypeScript compiles without errors

### US-005: Create WebSocket hook

**As a** developer, **I want** a React hook for WebSocket connections **so that** real-time updates work across the app.

**Acceptance Criteria:**
- [ ] `hooks/useWebSocket.ts` creates WebSocket connection to `VITE_WS_URL`
- [ ] Hook exposes `connectionStatus` state ('connecting', 'connected', 'disconnected')
- [ ] Hook accepts `onMessage` callback for handling events
- [ ] Auto-reconnect on disconnect with exponential backoff
- [ ] Cleanup on unmount
- [ ] TypeScript compiles without errors

### US-006: Create session list sidebar component

**As a** user, **I want** a sidebar showing all sessions grouped by project **so that** I can navigate between sessions.

**Acceptance Criteria:**
- [ ] `components/SessionList.tsx` renders sessions grouped by project
- [ ] Sessions show active status indicator (green dot)
- [ ] Clicking a session calls `onSessionSelect` callback
- [ ] Uses shadcn Scroll area for overflow
- [ ] Responsive: collapses to drawer on mobile (< 768px)
- [ ] TypeScript compiles without errors

### US-007: Create session history view component

**As a** user, **I want** to see the conversation history for the selected session **so that** I can read previous messages.

**Acceptance Criteria:**
- [ ] `components/SessionHistory.tsx` renders list of messages
- [ ] User messages align right, assistant messages align left
- [ ] Messages use shadcn Card component for styling
- [ ] Timestamp displayed below each message
- [ ] Scroll to bottom when new messages arrive
- [ ] Empty state shows "No messages yet"
- [ ] TypeScript compiles without errors

### US-008: Create chat input component

**As a** user, **I want** an input field to send messages **so that** I can communicate with the agent.

**Acceptance Criteria:**
- [ ] `components/ChatInput.tsx` has textarea and send button
- [ ] Uses shadcn Textarea and Button components
- [ ] Send button disabled when input is empty or session is inactive
- [ ] Shift+Enter for new line, Enter to send
- [ ] Calls `onSendMessage` with content and clears input
- [ ] Auto-resize textarea based on content
- [ ] TypeScript compiles without errors

### US-009: Create app layout with header and sidebar

**As a** user, **I want** a responsive layout with sidebar and main content area **so that** the app works on desktop and mobile.

**Acceptance Criteria:**
- [ ] `App.tsx` layouts sidebar (left) and main panel (right)
- [ ] Header shows app name and connection status
- [ ] Hamburger menu toggles sidebar on mobile
- [ ] Stop session button in header when session is active
- [ ] Uses Zustand store for sidebar collapse state
- [ ] TypeScript compiles without errors

### US-010: Wire WebSocket events to UI updates

**As a** developer, **I want** WebSocket events to update the UI **so that** changes reflect in real-time.

**Acceptance Criteria:**
- [ ] `SessionStarted` event updates session active status
- [ ] `SessionStopped` event clears session active status
- [ ] `ThinkingDelta` appends to current thinking message
- [ ] `MessageAdded` appends new message to history
- [ ] Events use React Query cache updates
- [ ] TypeScript compiles without errors

### US-011: Create Zustand store for global state

**As a** developer, **I want** a Zustand store for global app state **so that** components can share state easily.

**Acceptance Criteria:**
- [ ] `store/appStore.ts` defines store with `currentSessionId`, `sidebarCollapsed`
- [ ] Store includes actions: `setCurrentSession`, `toggleSidebar`, `setSidebarCollapsed`
- [ ] Store persists to localStorage
- [ ] TypeScript compiles without errors

### US-012: Add static file serving to backend

**As a** developer, **I want** the backend to serve the frontend build **so that** the app can be deployed as a single binary.

**Acceptance Criteria:**
- [ ] `src/static_files.rs` module serves files from `frontend-web/dist/`
- [ ] `src/main.rs` adds `fallback` route for SPA routing
- [ ] Static files served at `/` path
- [ ] CORS configured for development (Vite dev server on port 5173)
- [ ] `cargo check` passes

### US-013: Create new session dialog

**As a** user, **I want** a dialog to create new sessions **so that** I can start new conversations.

**Acceptance Criteria:**
- [ ] `components/NewSessionDialog.tsx` uses shadcn Dialog
- [ ] Project dropdown selector (populated from API)
- [ ] Optional session name input
- [ ] Create button calls API mutation
- [ ] Dialog closes on successful creation
- [ ] TypeScript compiles without errors

### US-014: Add error handling with toasts

**As a** user, **I want** to see error messages when something goes wrong **so that** I understand what happened.

**Acceptance Criteria:**
- [ ] `components/Toaster.tsx` integrates shadcn Toast
- [ ] API errors trigger toast notifications
- [ ] WebSocket errors trigger toast notifications
- [ ] Success toasts for session creation/start
- [ ] TypeScript compiles without errors

### US-015: Implement thinking state display

**As a** user, **I want** to see when the agent is thinking **so that** I know a response is coming.

**Acceptance Criteria:**
- [ ] `components/ThinkingIndicator.tsx` shows animated loader
- [ ] Streaming thinking content displays in real-time
- [ ] Thinking message styled differently from regular messages
- [ ] Clears when assistant message is complete
- [ ] TypeScript compiles without errors

### US-016: Delete old Makepad frontend

**As a** developer, **I want** the old frontend removed **so that** it doesn't cause confusion.

**Acceptance Criteria:**
- [ ] `frontend/` directory deleted after new frontend is functional
- [ ] No references to Makepad in repository
- [ ] `.gitignore` updated to exclude `frontend-web/node_modules/` and `frontend-web/dist/`
- [ ] README updated with new development instructions

### US-017: Update build and deployment scripts

**As a** developer, **I want** build scripts for the new frontend **so that** deployment is automated.

**Acceptance Criteria:**
- [ ] `frontend-web/package.json` has `build` script for production
- [ ] Root `Makefile` or script builds frontend then backend
- [ ] Production build outputs to `frontend-web/dist/`
- [ ] README documents build process

### US-018: Add environment variable configuration

**As a** developer, **I want** environment variables for API URLs **so that** development and production use different endpoints.

**Acceptance Criteria:**
- [ ] `.env.example` documents `VITE_API_URL` and `VITE_WS_URL`
- [ ] Default values: `VITE_API_URL=http://localhost:8080/api`, `VITE_WS_URL=ws://localhost:8080/ws`
- [ ] Vite loads env variables from `.env` file
- [ ] TypeScript compiles without errors

## Non-Goals

- Server-side rendering (SSR) - this is a client-side SPA
- Multi-user authentication - single-user application
- Offline support - requires active WebSocket connection
- Message editing or deletion - read-only message history

## Technical Considerations

- **WebSocket URL format**: Must match backend `ws://localhost:8080/ws`
- **CORS**: Backend must allow requests from Vite dev server (port 5173) during development
- **SPA routing**: Backend must serve index.html for all non-API routes
- **Message timestamps**: Backend sends ISO 8601 strings, format for display
- **Session status**: Track `is_running` state for UI controls
- **Mobile breakpoint**: 768px for sidebar collapse behavior

## Open Questions

- None at this time
