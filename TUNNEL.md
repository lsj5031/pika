# Cloudflare Tunnel - pi.liu.nz

## Tunnel Information

- **Name**: pi-api
- **ID**: `3f997687-540b-436a-b2cb-250984b6b0cf`
- **Domain**: pi.liu.nz
- **Config**: `~/.cloudflared/config-pi.yml`
- **Credentials**: `~/.cloudflared/3f997687-540b-436a-b2cb-250984b6b0cf.json`

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
curl https://pi.liu.nz/health
```

## Tunnel Configuration

The tunnel is configured to route:
- `pi.liu.nz` → `http://localhost:7847`

This means all traffic (API, WebSocket, static files) goes through the tunnel.

## Systemd Service

Service file: `/etc/systemd/system/cloudflared-pi.service`

```ini
[Unit]
Description=Cloudflare Tunnel - pi
After=network.target

[Service]
Type=simple
User=leo
ExecStart=/home/leo/.local/bin/cloudflared tunnel --config /home/leo/.cloudflared/config-pi.yml run
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
nslookup pi.liu.nz
dig pi.liu.nz

# Verify CNAME in Cloudflare dashboard
# Should point to: 3f997687-540b-436a-b2cb-250984b6b0cf.cfargotunnel.com
```

### Connection timeout
- Check if backend is running: `sudo systemctl status pi-agent-manager`
- Verify backend is listening on port 7847: `sudo lsof -i :7847`
- Check firewall rules
