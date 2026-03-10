# Deployment Guide

## Quick Start

Use either command:

```bash
make deploy
# or
./deploy/setup-deployment.sh
```

Both workflows:
1. Build frontend + backend
2. Create/prepare runtime layout (`/opt/pika`, `/etc/pika`, `/var/lib/pika`)
3. Stage binary and frontend assets under `/opt/pika`
4. Install/enable/start a Cloudflare tunnel service and `pika` systemd service

---

## Application Status

### ✅ Production Features
- **Session Management**: Full CRUD operations for Pika sessions
- **Real-time Updates**: WebSocket connection for live status
- **Authentication**: environment credentials + signed session cookies (cookie-only protected routes)
- **Project Management**: Add/remove project folders
- **Chat Interface**: Send prompts and view conversation history
- **Code Diff Viewer**: View code changes with syntax highlighting
- **Responsive Design**: Mobile-friendly interface
- **Error Handling**: Comprehensive error messages and toast notifications

### 🔧 Runtime Layout
- **Backend Port**: 7847 (configurable via `config.toml` or `PORT` env var)
- **Runtime Root**: `/opt/pika`
- **Backend Binary**: `/opt/pika/target/release/pika`
- **Frontend Assets**: `/opt/pika/frontend-web/dist/`
- **Service Config**: `/etc/pika/config.toml`
- **Service Env File**: `/etc/pika/pika.env`
- **Services**: `pika` and `cloudflared-pi` (or your chosen tunnel service)

## Overview

- **Local Port**: 7847
- **Architecture**: Single Rust backend serves API + static frontend files
- **Tunnel**: Cloudflare Tunnel (or any reverse proxy of your choice)

All traffic goes through Cloudflare Tunnel (or your reverse proxy).

## Edge Security Headers (HSTS at Cloudflare)

HSTS is enforced at the Cloudflare edge, not by the tunnel ingress config.

### Configure HSTS in Cloudflare

1. Open Cloudflare Dashboard → your zone
2. Go to **SSL/TLS → Edge Certificates**
3. Enable **Always Use HTTPS**
4. Enable **HTTP Strict Transport Security (HSTS)** with:
   - `max-age=31536000`
   - `includeSubDomains` (only if all subdomains are HTTPS-ready)
   - `preload` (only when ready for long-lived preload behavior)

Recommended header value:

```text
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

### Verify

```bash
curl -I https://your-domain.example | grep -i strict-transport-security
```

## Setup Steps (Manual)

### 1) Build Artifacts

```bash
make build
```

### 2) Create Service User + Directories

```bash
sudo useradd --system --home /var/lib/pika --create-home --shell /usr/sbin/nologin pika
sudo install -d -m 0755 -o pika -g pika /var/lib/pika /opt/pika /opt/pika/target /opt/pika/target/release /opt/pika/frontend-web
sudo install -d -m 0750 -o root -g pika /etc/pika
```

### 3) Stage Runtime Artifacts

```bash
sudo install -m 0755 -o pika -g pika target/release/pika /opt/pika/target/release/pika
sudo rm -rf /opt/pika/frontend-web/dist
sudo cp -r frontend-web/dist /opt/pika/frontend-web/
sudo chown -R pika:pika /opt/pika/frontend-web/dist
```

Initial config/env bootstrap (only if missing):

```bash
if [ ! -f /etc/pika/config.toml ]; then
  sudo install -m 0640 -o root -g pika config.toml.example /etc/pika/config.toml
fi
[ -f /etc/pika/pika.env ] || sudo install -m 0640 -o root -g pika /dev/null /etc/pika/pika.env
```

### 4) Install Services

```bash
sudo cp deploy/pika.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable pika.service
```

Optionally install a Cloudflare tunnel service:

```bash
sudo cp deploy/cloudflared-pi.service /etc/systemd/system/
sudo systemctl enable cloudflared-pi.service
```

### 5) Start Services

```bash
sudo systemctl start cloudflared-pi.service   # if using Cloudflare Tunnel
sudo systemctl restart pika.service
```

### 6) Configure Environment

Edit `/etc/pika/pika.env` with at least:

```bash
AUTH_USERNAME=your-user
AUTH_PASSWORD=your-password
AUTH_SESSION_SECRET=32+bytes-random-secret
BIND_ADDRESS=127.0.0.1
CORS_ALLOWED_ORIGINS=https://your-domain.example
TRUSTED_PROXY_CIDRS=127.0.0.1/32
# Optional: force npx path when Node is installed via nvm
# PIKA_NPX_PATH=/home/youruser/.nvm/versions/node/v22.x.x/bin/npx
```

`TRUSTED_PROXY_CIDRS=127.0.0.1/32` is recommended for Cloudflare Tunnel on localhost. Without it, login/WS rate limiting can treat all users as the proxy peer IP.

Then restart:

```bash
sudo systemctl restart pika
```

## Architecture

```
User Browser
    ↓
your-domain.example (Cloudflare or reverse proxy)
    ↓
Cloudflare Tunnel (or other reverse proxy)
    ↓
localhost:7847 (Rust backend)
    ├─→ /api/* → API endpoints
    ├─→ /ws → WebSocket
    └─→ /* → Static frontend files (from /opt/pika/frontend-web/dist/)
```

## Configuration Files

- **Tunnel Config**: `~/.cloudflared/config.yml` (if using Cloudflare Tunnel)
- **Pika Service**: `/etc/systemd/system/pika.service`
- **Pika Config**: `/etc/pika/config.toml`
- **Pika Env**: `/etc/pika/pika.env`

## `ProtectHome=true` Notes

`pika.service` uses `ProtectHome=true` and a tight filesystem policy. If project roots are outside default readable paths, add a scoped drop-in override.

Example:

```bash
sudo systemctl edit pika
```

```ini
[Service]
BindReadOnlyPaths=/srv/projects
ReadWritePaths=/var/lib/pika /run/pika /srv/projects/repo-a
```

Apply changes:

```bash
sudo systemctl daemon-reload
sudo systemctl restart pika
```

## Management Commands

```bash
# status
sudo systemctl status cloudflared-pi --no-pager -l
sudo systemctl status pika --no-pager -l

# logs
sudo journalctl -u cloudflared-pi -f
sudo journalctl -u pika -f

# restart
sudo systemctl restart cloudflared-pi
sudo systemctl restart pika
```

## Troubleshooting

### Tunnel not working

```bash
sudo systemctl status cloudflared-pi
curl http://localhost:7847/health
```

### Frontend not updating

```bash
make frontend
make stage-runtime
sudo systemctl restart pika
```

### Frontend not connecting to API

- Verify tunnel is running
- Check browser console for CORS errors
- Verify `CORS_ALLOWED_ORIGINS=https://your-domain.example`
