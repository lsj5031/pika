# Cloudflare Tunnel - your-domain.example

## Tunnel Information

- **Name**: pi-api
- **ID**: `TUNNEL_ID_REDACTED`
- **Domain**: your-domain.example
- **Config**: `~/.cloudflared/config-pi.yml`
- **Credentials**: `~/.cloudflared/TUNNEL_ID_REDACTED.json`

## Quick Commands

### Check Tunnel Status
```bash
sudo systemctl status cloudflared-pi
```

### View Tunnel Logs
```bash
sudo journalctl -u cloudflared-pi -f
```

### Restart Tunnel
```bash
sudo systemctl restart cloudflared-pi
```

### Test Connection
```bash
# Test local backend
curl http://localhost:7847/health

# Test through tunnel
curl https://your-domain.example/health
```

## Tunnel Configuration

The tunnel is configured to route:
- `your-domain.example` → `http://localhost:7847`

This means all traffic (API, WebSocket, static files) goes through the tunnel.

## Systemd Service

Service file: `/etc/systemd/system/cloudflared-pi.service`

```ini
[Unit]
Description=Cloudflare Tunnel - pi
After=network.target

[Service]
Type=simple
User=youruser
ExecStart=/home/youruser/.local/bin/cloudflared tunnel --config /home/youruser/.cloudflared/config-pi.yml run
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Troubleshooting

### Tunnel not connecting
```bash
# Check if tunnel is running
sudo systemctl status cloudflared-pi

# Check logs for errors
sudo journalctl -u cloudflared-pi -n 50

# Verify config file
cat ~/.cloudflared/config-pi.yml

# Test tunnel manually
cloudflared tunnel --config ~/.cloudflared/config-pi.yml run
```

### DNS not resolving
```bash
# Check DNS
nslookup your-domain.example
dig your-domain.example

# Verify CNAME in Cloudflare dashboard
# Should point to: TUNNEL_ID_REDACTED.cfargotunnel.com
```

### Connection timeout
- Check if backend is running: `sudo systemctl status pika`
- Verify backend is listening on port 7847: `sudo lsof -i :7847`
- Check firewall rules
