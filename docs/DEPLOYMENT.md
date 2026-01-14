# Deployment Guide - pi.liu.nz

## Quick Start

Run the setup script to build and deploy everything:

```bash
./deploy/setup-deployment.sh
```

This will:
1. Build the frontend (production)
2. Build the backend (Rust release binary)
3. Install and start the Cloudflare tunnel service
4. Install and start the pi-agent-manager service

After setup, your app will be available at: **https://pi.liu.nz**

This guide covers deploying the pi-agent-manager application using Cloudflare Tunnel.

## Overview

- **Domain**: pi.liu.nz
- **Local Port**: 7847
- **Architecture**: Single Rust backend serves both API and static frontend files
- **Tunnel ID**: 3f997687-540b-436a-b2cb-250984b6b0cf

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
./target/release/pi-agent-manager
```

Or use systemd service (if created):
```bash
sudo systemctl start pi-agent-manager
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
pi-api.liu.nz (Cloudflare)
    ↓
Cloudflare Tunnel (3f997687-540b-436a-b2cb-250984b6b0cf)
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
sudo systemctl restart pi-agent-manager
```

## Environment Variables

Frontend (`.env`):
```
VITE_API_BASE_URL=https://pi-api.liu.nz
```

Backend will automatically use the tunnel endpoint.

## Troubleshooting

### Tunnel not working
```bash
# Check if cloudflared is running
sudo systemctl status cloudflared-pi-api

# Check DNS
nslookup pi-api.liu.nz

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
sudo systemctl restart pi-agent-manager
```

### Frontend not connecting to API
- Verify the tunnel is running
- Check browser console for CORS errors
- Ensure `VITE_API_BASE_URL` is set correctly
