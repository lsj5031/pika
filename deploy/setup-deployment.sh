#!/bin/bash
set -e

echo "🚀 Setting up pi.liu.nz deployment"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}Please don't run this script as root${NC}"
    echo "The script will use sudo where needed"
    exit 1
fi

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo -e "${YELLOW}Step 1: Building frontend...${NC}"
cd "$PROJECT_ROOT/frontend-web"
npm run build
echo -e "${GREEN}✓ Frontend built${NC}"

echo -e "${YELLOW}Step 2: Building backend...${NC}"
cd "$PROJECT_ROOT"
cargo build --release
echo -e "${GREEN}✓ Backend built${NC}"

echo -e "${YELLOW}Step 3: Installing cloudflared systemd service...${NC}"
sudo cp "$SCRIPT_DIR/cloudflared-pi.service" /etc/systemd/system/
sudo systemctl daemon-reload
echo -e "${GREEN}✓ Cloudflared service installed${NC}"

echo -e "${YELLOW}Step 4: Installing Pika systemd service...${NC}"
sudo cp "$SCRIPT_DIR/pika.service" /etc/systemd/system/
sudo systemctl daemon-reload
echo -e "${GREEN}✓ Pika service installed${NC}"

echo -e "${YELLOW}Step 5: Starting services...${NC}"
sudo systemctl enable cloudflared-pi.service
sudo systemctl start cloudflared-pi.service
sudo systemctl enable pika.service
sudo systemctl start pika.service
echo -e "${GREEN}✓ Services started${NC}"

echo ""
echo -e "${GREEN}🎉 Deployment complete!${NC}"
echo ""
echo "Services status:"
sudo systemctl status cloudflared-pi.service --no-pager -l
echo ""
sudo systemctl status pika.service --no-pager -l
echo ""
echo "Your app should now be available at: https://pi.liu.nz"
echo ""
echo "Useful commands:"
echo "  - View logs: sudo journalctl -u pika -f"
echo "  - Restart: sudo systemctl restart pika"
echo "  - Update frontend: cd frontend-web && npm run build && sudo systemctl restart pika"
echo "  - Update backend: cargo build --release && sudo systemctl restart pika"
