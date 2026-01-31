# Pika

<div align="center">
  <img src="frontend-web/public/logo.png" alt="Pika Logo" width="200" />
</div>

[![Status: Production Ready](https://img.shields.io/badge/status-production%20ready-brightgreen)](https://pi.liu.nz)
[![Deployment](https://img.shields.io/badge/deployment-pi.liu.nz-blue)](https://pi.liu.nz)

Pika is your cute AI Coding Companion. A web application for managing multiple pi-coding-agent sessions across projects. Built with Rust (Axum) backend and React + TypeScript + Vite frontend.

**Status**: ✅ **Production Ready** - Deployed at https://pi.liu.nz

## Features

### Core Functionality
- **Pika Companion**: Your friendly rodent manager for AI sessions
- **Session Management**: View, create, start, and stop pi-coding-agent sessions
- **Project Organization**: Sessions grouped by project folder
- **Real-time Updates**: WebSocket integration for live status updates
- **Chat Interface**: Send prompts and view conversation history
- **Authentication**: Secure API key authentication for pi-coding-agent

### User Interface
- **Session List**: View all sessions with status indicators
- **Session Detail**: Full conversation history with diff viewer
- **Create Sessions**: Easy wizard for creating new sessions
- **Project Management**: Add and manage project folders
- **Settings**: Configure API keys and connection settings
- **Mobile Responsive**: Optimized for mobile devices (with known overflow issue on <390px screens)

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
- npx (for running pi-coding-agent)

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

### Environment Variables

For frontend development, create `frontend-web/.env`:

```
VITE_API_URL=http://localhost:8080/api
VITE_WS_URL=ws://localhost:8080/ws
```

The backend serves API at `http://localhost:8080/api` and WebSocket at `ws://localhost:8080/ws`.

## Deployment

### Quick Deploy to Production

The application is deployed at **https://pi.liu.nz** using Cloudflare Tunnel.

```bash
make deploy        # Build and deploy everything
```

This will:
1. Build frontend and backend
2. Install systemd services
3. Start both tunnel and backend services

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
make deploy         # Deploy to production
make install-service # Install systemd services
make restart-service # Restart services
make status         # Check service status
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

# View logs
sudo journalctl -u pika -f
sudo journalctl -u cloudflared-pi -f

# Restart services
make restart-service
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
│   │   │   ├── AuthPrompt.tsx      # API key input
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
- Cloudflare Tunnel (pi.liu.nz)
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
