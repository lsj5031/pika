### Product Requirements Document (PRD) v2: Pika



#### 1. Overview & Problem Statement

**Product Name**: Pika



**Problem**:  

Managing multiple pi-coding-agent sessions across projects (git repos/folders) is terminal-bound and not mobile-friendly. Users need a unified way to discover, monitor, resume, and interact with up to 10 concurrent sessions from a phone (via tunnel), without chaos from juggling terminals.



**Solution**:  

A local web app (Rust backend + Makepad WASM frontend) serving a responsive, mobile-optimized UI. It discovers projects and their per-project sessions (leveraging pi's cwd-specific session listing/storage), keeps up to 10 sessions running concurrently in background (using JSON-RPC mode for structured live streaming), allows seamless switching/monitoring, and interaction with one active chat at a time. Maximally exploits pi features: JSON-RPC for live control/monitoring, HTML export for static views where suitable.



**Key Exploitations**:

- **JSON-RPC mode** (`--mode rpc`): Primary for all live sessions—structured events for thinking blocks, deltas, tool calls, responses.

- **HTML export**: For static/offline session views (e.g., full rendered history when not live).

- **Per-project sessions**: Sessions tied to cwd (project root)—running pi in a folder shows only that folder's sessions.

- **Persistence**: File-based JSONL per-project.



#### 2. Goals & Success Metrics

- Enable phone-based access to select projects/sessions, chat, monitor real-time progress on 10 concurrent agents.

- Day-1 usable: Open on phone, pick a project/session, resume if needed, chat, see live thinking/responses.

- Target: Smooth handling of 10 background running sessions.



#### 3. User Persona & Context

- Solo user, local machine (projects on disk).

- Access via browser (local or tunneled for phone).

- Purely local app, no remote/server components.



#### 4. Technical Architecture

- **Backend**: Rust (Axum/Tokio) – HTTP server + WebSockets.

  - Configurable port (for ngrok/Cloudflare Tunnel).

  - Manages up to 10 pi subprocesses (`npx @mariozechner/pi-coding-agent --mode rpc [resume flags]` with cwd = project root).

  - Sessions stay running when UI switches away (background monitoring).

  - On select inactive session: auto-resume/spawn process.

  - JSON-RPC via stdin/stdout piping; forward events via WS to frontend.

  - Optional: Spawn pi for HTML export on demand.

- **Frontend**: Makepad compiled to WASM – responsive web UI (mobile touch-friendly, dark mode, split panes).

  - Sidebar (collapsible on mobile): Projects > Sessions (searchable list/tree).

  - Main panel: Chat view (history + live streaming).

  - Optional log/thinking pane.

- **Session Discovery**: 

  - User configures project root folders.

  - Per project: Auto-detect sessions via filesystem (hidden dir inside project root containing JSONL files—e.g., `.pi/sessions/` based on cwd behavior).

- **Concurrency**: Keep all viewed/recent sessions live (up to 10); manual stop if needed.



#### 5. MVP User Stories (Prioritized)

**Epic 1: Setup & Discovery**

- **Story 1.1**: As a user, I can add/configure project root folders.

  - AC: Config UI/file; roots scanned for sessions.

- **Story 1.2 (Core)**: As a user, I can see a searchable sidebar with projects and their sessions.

  - AC: Tree/list view; sessions auto-discovered per-project (leveraging cwd-specific storage); searchable; indicators for running vs. inactive.



**Epic 2: Session Viewing & Monitoring**

- **Story 2.1 (Core)**: As a user, I can select a session to view its history.

  - AC: For inactive: Load static view (prefer HTML export if seamless, else parsed JSONL rendered as chat).

  - For active: Full history + real-time updates.

- **Story 2.2 (Core)**: As a user, I can monitor live progress (thinking blocks, responses, logs).

  - AC: Streaming via JSON-RPC events; rendered in real-time (auto-scroll, thinking indicators); optional raw log pane.



**Epic 3: Interaction & Lifecycle**

- **Story 3.1 (Core)**: As a user, I can send messages/tasks via chat input.

  - AC: Large touch-friendly input box + send button; sends via JSON-RPC prompt; streams response live.

- **Story 3.2 (Core)**: As a user, I can resume/switch to any session (including old ones).

  - AC: Click in sidebar; if not running, auto-spawn in RPC mode with resume flags + cwd=project root; history loads instantly; previous sessions keep running.

- **Story 3.3**: As a user, I can start a new session in a project.

  - AC: Button in project view; spawns fresh pi RPC process (cwd=root, default settings).

- **Story 3.4**: As a user, I can manually stop a running session.

  - AC: Context button; kills process.



**Epic 4: Mobile Experience**

- **Story 4.1 (Core)**: As a user, the UI works seamlessly on phone browser.

  - AC: Responsive (collapsible sidebar, large inputs/buttons); full MVP features over tunnel.



**Epic 5: Rendering Strategy**

- **Primary**: Custom chat rendering (bubbles, code blocks, thinking sections) for consistency + live streaming.

- **Exploitation**: Button to "View as HTML Export" – backend spawns pi export and displays/serves the pretty static HTML (great for full static review).



#### 6. Priorities & Phasing

- **MVP (Day-1 Phone Usable)**: Stories 1.1, 1.2, 2.1, 2.2, 3.1, 3.2, 4.1 + basic custom rendering + auto-resume on select.

- **Post-MVP**:

  - HTML export integration (view button).

  - Session status indicators (running/thinking/idle/error).

  - Resource limits (auto-pause long-inactive).

  - Voice/transcription input hook.

  - Session naming/editing.



#### 7. Open Implementation Notes (Based on Your Inputs – No Assumptions Beyond)

- Session storage: Per-project (cwd-specific); assumed hidden dir like `.pi/sessions/*.jsonl` inside root – confirm exact path in code if needed.

- Spawning: Always cwd=project root; respect default pi settings (tools, thinking, etc.); use RPC mode.

- No multi-agent coordination.

- Target 10 concurrent running sessions.
Yes, adding a dedicated **References** section with direct links to the pi-coding-agent documentation (and key issues/releases) would make the PRD stronger—especially for Ralph-style autonomous implementation. It removes any lingering ambiguity around pi internals (e.g., exact session paths, recent fixes), lets an implementing agent cross-reference directly, and follows best practices for reproducible builds.


#### 9. References & pi-coding-agent Dependencies

- **Main Repository**: https://github.com/badlogic/pi-mono

- **Coding Agent Package**: https://github.com/badlogic/pi-mono/tree/main/packages/coding-agent

- **Primary README** (covers CLI, sessions, resume, modes): https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/README.md

- **Key Details from Docs**:

  - Sessions persisted as JSONL in `~/.pi/agent/sessions/<encoded-cwd>/` (per-project, based on cwd).

  - Resume via `--session <id/path>`, `/resume` (interactive listing), `--continue`.

  - JSON-RPC mode: `--mode rpc` for headless structured streaming (prompts in, events out—thinking, deltas, tools).

  - HTML export: Built-in CLI flag for pretty static rendering.

- **Recent Relevant Fixes/Issues**:

  - Session directory fixes: https://github.com/badlogic/pi-mono/releases (Dec 2025+)

  - Session tree structure: https://github.com/badlogic/pi-mono/issues/316

- **Installation/Run**: `npx @mariozechner/pi-coding-agent [flags]` (or local install).

- **Note for Implementers**: Always spawn with cwd=project root; respect default pi config; prioritize JSON-RPC for live sessions.
