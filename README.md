# Pika

<div align="center">
  <img src="frontend-web/public/logo.png" alt="Pika Logo" width="200" />
</div>

[![Status: Production Ready](https://img.shields.io/badge/status-production%20ready-brightgreen)](https://your-domain.example)
[![Deployment](https://img.shields.io/badge/deployment-your-domain.example-blue)](https://your-domain.example)

Pika is your cute AI Coding Companion. A web application for managing multiple Pika sessions across projects. Built with Rust (Axum) backend and React + TypeScript + Vite frontend.

**Status**: ✅ **Production Ready** - Deployed at https://your-domain.example

## Features

### Core Functionality
- **Pika Companion**: Your friendly rodent manager for AI sessions
- **Session Management**: View, create, start, and stop Pika sessions
- **Project Organization**: Sessions grouped by project folder
- **Real-time Updates**: WebSocket integration for live status updates
- **Chat Interface**: Send prompts and view conversation history
- **Authentication**: Environment-backed login with signed HttpOnly session cookies (cookie-only protected routes)

### User Interface
- **Session List**: View all sessions with status indicators
- **Session Detail**: Full conversation history with diff viewer
- **Create Sessions**: Easy wizard for creating new sessions
- **Project Management**: Add and manage project folders
- **Settings**: Configure model/runtime settings and connection options
- **Mobile Responsive**: Optimized for mobile devices (including narrow screens)

### Technical Features
- **WebSocket Support**: Real-time session status updates
- **Diff Viewer**: View code changes with syntax highlighting
- **Thinking Indicator**: Real-time AI thinking state visualization
- **Error Handling**: Comprehensive error messages and toasts
- **Loading States**: Clear feedback during operations

## Development

### Prerequisites

- Rust (edition 2024)
- Node.js 18+ and npm
- npx (for running Pika CLI)

### Frontend Development

The frontend is in `frontend-web/` using Vite + React + TypeScript.

```bash
cd frontend-web
npm install
npm run dev      # Start dev server on http://localhost:5173
npm run build    # Production build to dist/
npm run lint     # Run ESLint
```

### Backend Development

The Rust backend serves both API endpoints and static frontend files.

```bash
cargo run        # Start dev server with hot reload
cargo build      # Build for development
cargo build --release    # Build for production
```

### Testing

```bash
# Backend integration/unit tests
cargo test

# Frontend unit tests
cd frontend-web && npm test

# Frontend lint
cd frontend-web && npm run lint

# E2E tests (requires backend on :7847)
make dev-backend
cd frontend-web && npm run test:e2e
```

### Environment Variables

For frontend development, create `frontend-web/.env`:

```
VITE_API_URL=http://localhost:7847
VITE_WS_URL=ws://localhost:7847/ws
```

For backend auth/security, configure environment variables (especially in production):

```bash
AUTH_USERNAME=admin
AUTH_PASSWORD=change-me
AUTH_SESSION_SECRET=32+bytes-random-secret
BIND_ADDRESS=127.0.0.1
# Optional overrides:
# CORS_ALLOWED_ORIGINS=https://your-domain.example
# TRUSTED_PROXY_CIDRS=127.0.0.1/32
# ALLOW_INSECURE_REMOTE=false
# ALLOWED_PROJECT_ROOTS=/srv/projects:/opt/work
# PIKA_NPX_PATH=/home/your-user/.nvm/versions/node/v22.18.0/bin/npx
```

Notes:
- Credentials are environment-only (not read from `config.toml`).
- Protected API/WS routes require a valid signed session cookie (no Basic Auth fallback).
- Session cookies default to `Secure=true` (set `session_cookie_secure=false` only for local HTTP dev).
- `AUTH_SESSION_SECRET` should be at least 32 bytes.
- Default bind is localhost (`127.0.0.1`).
- Remote bind without auth is blocked unless explicitly overridden.
- HSTS is configured at Cloudflare edge (see `docs/DEPLOYMENT.md`).

## Deployment

### Quick Deploy to Production

The application is deployed at **https://your-domain.example** using Cloudflare Tunnel.

```bash
make deploy        # Build and deploy everything (requires sudo)
make deploy-user   # Build and deploy with user systemd services (no sudo)
```

This will:
1. Build frontend and backend
2. Stage runtime files under `/opt/pika` and `/etc/pika`
3. Install systemd services
4. Start/restart tunnel and backend services

### Building for Production

Use the Makefile to build both frontend and backend:

```bash
make build      # Build frontend then backend
make clean      # Clean all build artifacts
make run        # Build and run production server
```

The production build:
1. Builds the frontend to `frontend-web/dist/`
2. Builds the Rust backend to `target/release/pika`
3. Backend serves static files from `frontend-web/dist/`

### Makefile Targets

```bash
make frontend       # Build frontend only
make backend        # Build backend only
make dev-frontend   # Start frontend dev server
make dev-backend    # Start backend with hot reload
make deploy         # Deploy to production (requires sudo)
make deploy-user    # Deploy using user systemd services (no sudo)
make stage-runtime  # Stage runtime files under /opt/pika + /etc/pika
make install-service # Install systemd services
make install-service-user # Install user systemd services (no sudo)
make restart-service # Restart services
make restart-service-user # Restart user services (no sudo)
make status         # Check service status
make status-user    # Check user service status
make help           # Show all available targets
```

### Running Locally

After building, run the production server:

```bash
./target/release/pika
```

Or use the Makefile:

```bash
make run
```

The server will start on port 7847 (configurable via `config.toml`) and serve the web UI.

### Service Management

```bash
# Check service status
make status
make status-user

# View logs
sudo journalctl -u pika -f
sudo journalctl -u cloudflared-pi -f
journalctl --user -u pika -f
journalctl --user -u cloudflared-pi -f

# Restart services
make restart-service
make restart-service-user
```

## Project Structure

```
pika/
├── src/                    # Rust backend source
│   ├── main.rs            # Server entry point
│   ├── static_files.rs    # Static file serving
│   └── ...
├── frontend-web/          # React frontend
│   ├── src/
│   │   ├── components/    # React components
│   │   │   ├── AppHeader.tsx       # Header with settings & status
│   │   │   ├── AuthPrompt.tsx      # Login prompt
│   │   │   ├── ChatInput.tsx       # Chat input component
│   │   │   ├── DiffViewer.tsx      # Code diff viewer
│   │   │   ├── NewSessionDialog.tsx # Create session wizard
│   │   │   ├── ProjectManager.tsx  # Project folder management
│   │   │   ├── SessionHistory.tsx  # Conversation history
│   │   │   ├── SessionList.tsx     # Main session list
│   │   │   ├── SettingsDialog.tsx  # Settings dialog
│   │   │   └── ThinkingIndicator.tsx # AI thinking state indicator
│   │   ├── hooks/         # Custom React hooks (13 hooks)
│   │   ├── lib/           # Utilities (API client, toasts)
│   │   ├── store/         # Zustand state stores
│   │   └── types/         # TypeScript types
│   ├── dist/              # Production build output
│   └── package.json
├── docs/                  # Documentation
│   ├── DEPLOYMENT.md      # Deployment guide
│   └── MOBILE_TEST_REPORT.md  # Mobile usability test results
├── deploy/                # Deployment scripts
├── config/                # Configuration templates
├── templates/             # Template files
├── Cargo.toml
├── Makefile
├── config.toml            # Backend configuration
├── QUICK_START.md         # Quick deployment guide
├── TUNNEL.md              # Cloudflare tunnel setup
└── README.md
```

## Technology Stack

**Backend:**
- Rust 2024 edition
- Axum web framework (with WebSocket support)
- Tokio async runtime
- Tower HTTP (CORS, static file serving)

**Frontend:**
- React 19
- TypeScript 5
- Vite 7
- Tailwind CSS v4
- shadcn/ui components
- React Query for API state management
- Zustand for global state
- Sonner for toast notifications
- React Router for navigation

**Deployment:**
- Cloudflare Tunnel (your-domain.example)
- systemd services for tunnel and backend
- Production builds served from Rust backend

## Known Issues

### ✅ Mobile Overflow - FIXED
- **Previous Issue**: Horizontal scroll overflow on devices with viewport <390px
- **Affected**: ~60% of mobile users (iPhone SE, iPhone 12/13, Android phones)
- **Fix Applied**: Changed header spacing to `gap-1.5 md:gap-4` in AppHeader.tsx
- **Status**: ✅ **RESOLVED** - Mobile layout now works correctly on all screen sizes
- **Reference**: See `docs/MOBILE_TEST_REPORT.md` for detailed analysis

**Current Implementation**: The AppHeader component uses responsive spacing that adjusts based on screen size, preventing horizontal overflow on mobile devices.

## Documentation

- `QUICK_START.md` - One-command deployment guide
- `STATUS.md` - Current project status and metrics
- `docs/DEPLOYMENT.md` - Detailed deployment instructions
- `docs/MOBILE_TEST_REPORT.md` - Mobile usability test results and fix verification

## License

MIT
