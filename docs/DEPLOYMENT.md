# Deployment Guide - your-domain.example

**Status**: ✅ **Deployed and Operational** at https://your-domain.example

## Quick Start

Run the setup script to build and deploy everything:

```bash
./deploy/setup-deployment.sh
```

This will:
1. Build the frontend (production)
2. Build the backend (Rust release binary)
3. Install and start the Cloudflare tunnel service
4. Install and start the pika service

After setup, your app will be available at: **https://your-domain.example**

---

## Application Status

### ✅ Production Features
- **Session Management**: Full CRUD operations for pi-coding-agent sessions
- **Real-time Updates**: WebSocket connection for live status
- **Authentication**: API key configuration via Settings dialog
- **Project Management**: Add/remove project folders
- **Chat Interface**: Send prompts and view conversation history
- **Code Diff Viewer**: View code changes with syntax highlighting
- **Responsive Design**: Mobile-friendly interface
- **Error Handling**: Comprehensive error messages and toast notifications

### 🔧 Configuration
- **Backend Port**: 7847 (configurable via `config.toml`)
- **Frontend**: Static files served by Rust backend from `frontend-web/dist/`
- **Tunnel**: Cloudflare Tunnel ID `TUNNEL_ID_REDACTED`
- **Services**: `pika` and `cloudflared-pi` systemd services

### 📱 Known Issues
- **Mobile Overflow**: Horizontal scroll on devices <390px viewport
  - Affects ~60% of mobile users
  - Fix documented in `MOBILE_TEST_REPORT.md`
  - Quick fix: Change `gap-4` to `gap-2 md:gap-4` in header component

## Overview

- **Domain**: your-domain.example
- **Local Port**: 7847
- **Architecture**: Single Rust backend serves both API and static frontend files
- **Tunnel ID**: TUNNEL_ID_REDACTED

The Rust backend serves:
- **API endpoints** at `/api/*`
- **Static frontend files** from `frontend-web/dist/`
- **WebSocket** at `/ws` for real-time updates

All traffic goes through the Cloudflare tunnel - that's it! No separate worker or deployment needed.

## Setup Steps

### 1. Install Systemd Service (Run as root)

```bash
sudo cp deploy/cloudflared-pi.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable cloudflared-pi.service
sudo systemctl start cloudflared-pi.service
```

### 2. Verify Tunnel Status

```bash
sudo systemctl status cloudflared-pi
```

### 3. Build the Application

```bash
make build
```

This will:
- Build the frontend (production)
- Build the backend (Rust release binary)

### 4. Start the Backend Server

```bash
./target/release/pika
```

Or use systemd service (if created):
```bash
sudo systemctl start pika
```

### 5. Build Frontend (Static Files)

The Rust backend serves static files from `frontend-web/dist/`, so build the frontend:

```bash
cd frontend-web
npm run build
cd ..
```

## Architecture

```
User Browser
    ↓
pi-ayour-domain.example (Cloudflare)
    ↓
Cloudflare Tunnel (TUNNEL_ID_REDACTED)
    ↓
localhost:7847 (Rust Backend)
    ↓
    ├─→ /api/* → API endpoints
    ├─→ /ws → WebSocket (real-time updates)
    └─→ /* → Static frontend files (from frontend-web/dist/)
```

## Configuration Files

- **Tunnel Config**: `~/.cloudflared/config-pi-api.yml`
- **Systemd Service**: `/etc/systemd/system/cloudflared-pi-api.service`
- **Project Config**: `config.toml` (in project root)
- **Frontend Env**: `frontend-web/.env`

## Management Commands

### Check Tunnel Status
```bash
sudo systemctl status cloudflared-pi-api
```

### Restart Tunnel
```bash
sudo systemctl restart cloudflared-pi-api
```

### View Tunnel Logs
```bash
sudo journalctl -u cloudflared-pi-api -f
```

### Update Frontend
```bash
cd frontend-web
npm run build
# No deployment needed - Rust backend serves static files directly
```

### Update Backend
```bash
cargo build --release
sudo systemctl restart pika
```

## Environment Variables

Frontend (`.env`):
```
VITE_API_BASE_URL=https://pi-ayour-domain.example
```

Backend will automatically use the tunnel endpoint.

## Troubleshooting

### Tunnel not working
```bash
# Check if cloudflared is running
sudo systemctl status cloudflared-pi-api

# Check DNS
nslookup pi-ayour-domain.example

# Test local connection
curl http://localhost:7847
```

### Frontend not updating
```bash
# Ensure frontend is built
cd frontend-web
npm run build

# Check that dist/ directory exists and has files
ls -la dist/

# Restart backend to pick up new static files
sudo systemctl restart pika
```

### Frontend not connecting to API
- Verify the tunnel is running
- Check browser console for CORS errors
- Ensure `VITE_API_BASE_URL` is set correctly
