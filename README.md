# PI Agent Manager

A local web application for managing multiple pi-coding-agent sessions across projects. Built with Rust (Axum) backend and React + TypeScript + Vite frontend.

## Features

- **Web-based UI**: Responsive interface accessible from browser or phone via tunnel
- **Session Management**: View, create, start, and stop pi-coding-agent sessions
- **Project Organization**: Sessions grouped by project folder
- **Real-time Updates**: WebSocket integration for live status updates
- **Chat Interface**: Send prompts and view conversation history

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

## Building for Production

Use the Makefile to build both frontend and backend:

```bash
make build      # Build frontend then backend
make clean      # Clean all build artifacts
make run        # Build and run production server
```

The production build:
1. Builds the frontend to `frontend-web/dist/`
2. Builds the Rust backend to `target/release/pi-agent-manager`
3. Backend serves static files from `frontend-web/dist/`

### Other Makefile Targets

```bash
make frontend       # Build frontend only
make backend        # Build backend only
make dev-frontend   # Start frontend dev server
make dev-backend    # Start backend with hot reload
make help           # Show all available targets
```

## Running

After building, run the production server:

```bash
./target/release/pi-agent-manager
```

Or use the Makefile:

```bash
make run
```

The server will start on port 8080 (configurable) and serve the web UI at `http://localhost:8080`.

## Project Structure

```
pi-agent-manager/
├── src/                    # Rust backend source
│   ├── main.rs            # Server entry point
│   ├── static_files.rs    # Static file serving
│   └── ...
├── frontend-web/          # React frontend
│   ├── src/
│   │   ├── components/    # React components
│   │   ├── hooks/         # Custom React hooks
│   │   ├── lib/           # Utilities (API client, toasts)
│   │   ├── store/         # Zustand state stores
│   │   └── types/         # TypeScript types
│   ├── dist/              # Production build output
│   └── package.json
├── Cargo.toml
├── Makefile
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
- React Query for API state
- Zustand for global state
- Sonner for toast notifications

## License

MIT
