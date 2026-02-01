# Quick Start - Deploy to pi.liu.nz

**Current Status**: ✅ Production deployed at https://pi.liu.nz

## One-Command Deployment

```bash
make deploy
```

This will:
1. ✅ Build frontend (`npm run build`)
2. ✅ Build backend (`cargo build --release`)
3. ✅ Install systemd services (tunnel + backend)
4. ✅ Start both services
5. ✅ Show service status

**Then access at: https://pi.liu.nz** 🎉

---

## Application Features

### ✅ Implemented
- Session management (create, view, start, stop)
- Real-time status updates via WebSocket
- Chat interface with conversation history
- Project folder management
- API key authentication
- Settings dialog
- Diff viewer for code changes
- Mobile responsive design
- Error handling and toast notifications

### 🔧 Configuration
- API Key: Configure in Settings dialog (stored in localStorage)
- Projects: Add project folders in Settings
- Sessions: Create via floating action button (+)

### 📱 Mobile Access
- Open https://pi.liu.nz on any device
- Responsive design adapts to screen size

---

## Other Useful Commands

### Build only (no deployment)
```bash
make build
# or
make frontend    # Frontend only
make backend     # Backend only
```

### Install services only (if already built)
```bash
make install-service
make restart-service
```

### Check service status
```bash
make status
```

### View logs
```bash
# Tunnel logs
sudo journalctl -u cloudflared-pi -f

# Backend logs
sudo journalctl -u pika -f
```

### Update after code changes
```bash
make deploy
```

---

## What's Deployed

```
pi.liu.nz (Cloudflare)
    ↓
Cloudflare Tunnel
    ↓
localhost:7847 (Rust Backend)
    ├─ /api/*      → API endpoints
    ├─ /ws         → WebSocket
    └─ /*          → Static files (frontend-web/dist/)
```

**Architecture**: Simple single-server setup. The Rust backend serves everything, and the Cloudflare tunnel exposes it securely to the internet.

---

## Configuration Files

- `~/.cloudflared/config-pi.yml` - Tunnel configuration
- `frontend-web/.env` - Frontend environment (points to https://pi.liu.nz)
- `config.toml` - Backend configuration
- `/etc/systemd/system/cloudflared-pi.service` - Tunnel service
- `/etc/systemd/system/pika.service` - Backend service

---

## Troubleshooting

### Not working?
```bash
# Check service status
make status

# Check logs
sudo journalctl -u pika -n 50
sudo journalctl -u cloudflared-pi -n 50

# Restart services
make restart-service
```

### Frontend not updating?
```bash
make frontend
make restart-service
```

### Backend not working?
```bash
make backend
make restart-service
```

---

## Development vs Production

**Development** (local testing):
```bash
make dev-frontend  # Terminal 1: Vite dev server on 5173
make dev-backend   # Terminal 2: Backend with hot reload on 7847
```

**Production** (deployed):
```bash
make deploy        # Build and deploy to pi.liu.nz
```

---

## Documentation

- `docs/DEPLOYMENT.md` - Detailed deployment guide
- `PERFORMANCE_FIXES.md` - Performance optimizations
- `README.md` - General project documentation
