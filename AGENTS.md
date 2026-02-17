# AGENTS.md - Pika (Pika)

## Build & Test Commands
- **Build all**: `make build` (frontend + backend)
- **Backend only**: `cargo build --release`
- **Frontend only**: `cd frontend-web && npm run build`
- **Run dev backend**: `cargo run` (port 7847)
- **Run dev frontend**: `cd frontend-web && npm run dev`
- **Rust tests**: `cargo test` | Single: `cargo test test_name`
- **Frontend unit tests**: `cd frontend-web && npm test` | Single: `npm test -- test_name`
- **E2E tests**: `cd frontend-web && npm run test:e2e`
- **Lint frontend**: `cd frontend-web && npm run lint`

## Design Decisions
- **No sidebar/session list**: Intentional. Session switching is done via Command Palette (⌘K) only.

## Architecture
- **Backend**: Rust/Axum web server with WebSocket support, JSON-RPC, basic auth
- **Frontend**: React 19 + TypeScript + Vite + TailwindCSS 4 + Radix UI + Zustand
- **Key modules**: `src/api.rs` (routes), `src/websocket.rs` (WS), `src/auth.rs`, `src/config.rs`
- **Other modules**: `src/pi.rs` (ProcessManager), `src/sessions.rs`, `src/rate_limit.rs`, `src/metrics.rs`, `src/file_watcher.rs`, `src/static_files.rs`
- **State**: `AppState` combines `WSState`, `ApiState`, `ProcessManager`, `SessionIndex`, `AuthContext`, `RateLimitState`
- **Tests**: Integration tests in `tests/`, use `pika::create_test_app()` with `test-utils` feature

## Code Style
- Rust: Use `thiserror` for errors, async/await with Tokio, `Arc<Mutex<>>` / `Arc<RwLock<>>` for shared state
- TypeScript: ESLint + strict mode, React hooks, @tanstack/react-query for data fetching
- Frontend uses shadcn/ui (class-variance-authority + tailwind-merge + clsx), lucide-react icons
- Prefer descriptive names, minimal comments, handle errors explicitly
