#!/bin/bash
set -euo pipefail

echo "🚀 Setting up Pika deployment"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PIKA_USER="${PIKA_USER:-pika}"
PIKA_GROUP="${PIKA_GROUP:-pika}"
PIKA_OPT_DIR="${PIKA_OPT_DIR:-/opt/pika}"
PIKA_ETC_DIR="${PIKA_ETC_DIR:-/etc/pika}"

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

echo -e "${YELLOW}Step 3: Preparing runtime layout (${PIKA_OPT_DIR}, ${PIKA_ETC_DIR})...${NC}"
if ! id -u "$PIKA_USER" >/dev/null 2>&1; then
    sudo useradd --system --home /var/lib/pika --create-home --shell /usr/sbin/nologin "$PIKA_USER"
fi

sudo install -d -m 0755 -o "$PIKA_USER" -g "$PIKA_GROUP" \
    /var/lib/pika "$PIKA_OPT_DIR" "$PIKA_OPT_DIR/target" "$PIKA_OPT_DIR/target/release" "$PIKA_OPT_DIR/frontend-web"
sudo install -d -m 0750 -o root -g "$PIKA_GROUP" "$PIKA_ETC_DIR"

sudo install -m 0755 -o "$PIKA_USER" -g "$PIKA_GROUP" \
    "$PROJECT_ROOT/target/release/pika" "$PIKA_OPT_DIR/target/release/pika"

sudo rm -rf "$PIKA_OPT_DIR/frontend-web/dist"
sudo cp -r "$PROJECT_ROOT/frontend-web/dist" "$PIKA_OPT_DIR/frontend-web/"
sudo chown -R "$PIKA_USER:$PIKA_GROUP" "$PIKA_OPT_DIR/frontend-web/dist"

if [ ! -f "$PIKA_ETC_DIR/config.toml" ]; then
    if [ -f "$PROJECT_ROOT/config.toml" ]; then
        sudo install -m 0640 -o root -g "$PIKA_GROUP" \
            "$PROJECT_ROOT/config.toml" "$PIKA_ETC_DIR/config.toml"
    else
        sudo install -m 0640 -o root -g "$PIKA_GROUP" \
            "$PROJECT_ROOT/config.toml.example" "$PIKA_ETC_DIR/config.toml"
    fi
fi

if [ ! -f "$PIKA_ETC_DIR/pika.env" ]; then
    sudo install -m 0640 -o root -g "$PIKA_GROUP" /dev/null "$PIKA_ETC_DIR/pika.env"
fi

echo -e "${GREEN}✓ Runtime artifacts staged${NC}"

echo -e "${YELLOW}Step 4: Installing systemd services...${NC}"
sudo cp "$SCRIPT_DIR/pika-tunnel.service" /etc/systemd/system/
sudo cp "$SCRIPT_DIR/pika.service" /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable pika-tunnel.service
sudo systemctl enable pika.service
echo -e "${GREEN}✓ Services installed${NC}"

echo -e "${YELLOW}Step 5: Starting services...${NC}"
sudo systemctl start pika-tunnel.service
sudo systemctl restart pika.service
echo -e "${GREEN}✓ Services started${NC}"

echo ""
echo -e "${GREEN}🎉 Deployment complete!${NC}"
echo ""
echo "Services status:"
sudo systemctl status pika-tunnel.service --no-pager -l
echo ""
sudo systemctl status pika.service --no-pager -l
echo ""
echo "Your app should now be available at your configured domain."
echo ""
echo "Post-deploy checklist:"
echo "  1) Edit $PIKA_ETC_DIR/pika.env with AUTH_USERNAME, AUTH_PASSWORD, AUTH_SESSION_SECRET"
echo "  2) Review $PIKA_ETC_DIR/config.toml project paths and CORS settings"
echo "  3) Restart service after config changes: sudo systemctl restart pika"
