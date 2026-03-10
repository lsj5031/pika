# Codebase Improvements

Identified improvements grouped by priority. Check off as completed.

## 🔴 High Priority (Correctness / Robustness)

- [x] **1. `metrics.rs`: O(n) eviction** — `record_timing` used `Vec::remove(0)`. Switched to `VecDeque` for O(1) front removal.
- [x] **2. `static_files.rs`: Blocking I/O** — `std::fs::metadata()` was synchronous on the async request path. Now uses `tokio::fs::metadata`.
- [x] **3. `main.rs`: Duplicated message parsing** — Extracted `extract_message_content()` in `sessions.rs`, shared by both `parse_session_message_line` and the live `event_bridge_task`. Removed ~120 lines of duplication.
- [x] **4. `auth.rs`: `constant_time_compare` leaks length** — Now iterates over the longer string's full length and seeds result with length XOR to prevent timing side-channel.

## 🟡 Medium Priority (Code Quality)

- [x] **5. `pi.rs`: No-op `.env_clear().envs(std::env::vars())`** — Removed redundant environment round-trip.
- [x] **6. `sessions.rs`: Repeated pi sessions base dir** — Extracted `pi_sessions_base_dir()` helper, replaced 6 call sites across `sessions.rs` and `file_watcher.rs`.
- [x] **7. Duplicate `health_check`** — Made `health_check` public in `lib.rs`, removed duplicate from `main.rs`, imported from lib.

## 🟢 Lower Priority (Polish)

- [x] **10. `static_files.rs`: No cache-busting for hashed assets** — Files under `/assets/` now get `max-age=31536000, immutable`; others keep `max-age=3600`.
- [x] **8. `api.rs`: 1700+ line monolith** — Split API module into `src/api/routes.rs`, `src/api/types.rs`, and `src/api/settings.rs`; `src/api.rs` now focuses on shared state + core handlers.
- [x] **9. No structured logging** — Added `tracing` + `tracing-subscriber`, initialized subscriber in `main.rs`, and replaced backend `println!`/`eprintln!` calls with structured logging macros.
- ~~**11. Frontend: `web-vitals` unused**~~ — Actually used in `lib/performance.ts`. Not an issue.

## Review Summary

- All originally tracked actionable items (`1` through `10`) are now completed.
- Validation run: `cargo test` passes (unit + integration tests).
