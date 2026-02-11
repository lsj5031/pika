# Quick Start - Deploy to your-domain.example

**Current Status**: ✅ Production deployed at https://your-domain.example

## One-Command Deployment

```bash
make deploy
```

This now:
1. ✅ Builds frontend (`npm run build`)
2. ✅ Builds backend (`cargo build --release`)
3. ✅ Stages runtime files under `/opt/pika` and `/etc/pika`
4. ✅ Installs/enables systemd services (`cloudflared-pi`, `pika`)
5. ✅ Starts/restarts services

**Then access at: https://your-domain.example** 🎉

---

## Required Backend Environment

Set these in `/etc/pika/pika.env`:

```bash
AUTH_USERNAME=your-user
AUTH_PASSWORD=your-password
AUTH_SESSION_SECRET=32+bytes-random-secret
BIND_ADDRESS=127.0.0.1
CORS_ALLOWED_ORIGINS=https://your-domain.example
TRUSTED_PROXY_CIDRS=127.0.0.1/32
```

Notes:
- Protected routes are session-cookie only after login (no Basic Auth fallback).
- `TRUSTED_PROXY_CIDRS=127.0.0.1/32` is recommended for Cloudflare Tunnel on localhost.
- Enable HSTS at Cloudflare edge and verify with `curl -I https://your-domain.example`.

---

## Useful Commands

```bash
make build              # build frontend + backend
make stage-runtime      # stage /opt/pika + /etc/pika from built artifacts
make install-service    # install systemd units (expects built artifacts)
make restart-service    # restart cloudflared + pika
make status             # check system service status
```

View logs:

```bash
sudo journalctl -u cloudflared-pi -f
sudo journalctl -u pika -f
```

---

## Deployed Architecture

```
your-domain.example (Cloudflare)
    ↓
Cloudflare Tunnel
    ↓
localhost:7847 (Rust backend)
    ├─ /api/*      → API endpoints
    ├─ /ws         → WebSocket
    └─ /*          → Static files (/opt/pika/frontend-web/dist)
```

For full details, see `docs/DEPLOYMENT.md`.
