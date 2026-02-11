.PHONY: all build frontend-backend frontend backend clean dev-frontend dev-backend run test test-mobile test-install deploy deploy-user stage-runtime install-service install-service-user restart-service restart-service-user status status-user help

PIKA_USER ?= pika
PIKA_GROUP ?= pika
PIKA_OPT_DIR ?= /opt/pika
PIKA_ETC_DIR ?= /etc/pika

# Default target
all: build

# Build everything: frontend then backend
build: frontend-backend

# Build frontend (production) and then backend
frontend-backend:
	@echo "Building frontend..."
	cd frontend-web && npm run build
	@echo "Frontend built successfully"
	@echo "Building backend..."
	cargo build --release
	@echo "Backend built successfully"

# Build frontend only (for development)
frontend:
	@echo "Building frontend..."
	cd frontend-web && npm run build

# Build backend only
backend:
	@echo "Building backend..."
	cargo build --release

# Development: run frontend dev server
dev-frontend:
	@echo "Starting frontend dev server..."
	cd frontend-web && npm run dev

# Development: run backend with hot reload
dev-backend:
	@echo "Starting backend dev server..."
	cargo run

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cd frontend-web && rm -rf dist node_modules/.vite
	cargo clean
	@echo "Clean complete"

# Run the production build
run: build
	@echo "Starting production server..."
	./target/release/pika

# Install test dependencies
test-install:
	@echo "Installing test dependencies..."
	cd /tmp/pika-test && npm install
	cd /tmp/pika-test && npx playwright install chromium
	@echo "Test dependencies installed"

# Run all E2E tests (requires server to be running)
test:
	@echo "Running E2E tests..."
	@echo "Make sure the server is running on port 7847"
	cd /tmp/pika-test && npx playwright test

# Run mobile E2E tests with visible browser (requires server to be running)
test-mobile:
	@echo "Running mobile E2E tests with visible browser..."
	@echo "Make sure the server is running on port 7847"
	cd /tmp/pika-test && npx playwright test --project=mobile --headed

# Stage production runtime artifacts under /opt/pika and /etc/pika
stage-runtime:
	@echo "📦 Preparing runtime layout..."
	@if [ ! -f target/release/pika ]; then \
		echo "❌ Missing backend binary target/release/pika. Run 'make build' first."; \
		exit 1; \
	fi
	@if [ ! -d frontend-web/dist ]; then \
		echo "❌ Missing frontend build frontend-web/dist. Run 'make build' first."; \
		exit 1; \
	fi
	@if ! id -u $(PIKA_USER) >/dev/null 2>&1; then \
		echo "Creating service user $(PIKA_USER)..."; \
		sudo useradd --system --home /var/lib/pika --create-home --shell /usr/sbin/nologin $(PIKA_USER); \
	fi
	sudo install -d -m 0755 -o $(PIKA_USER) -g $(PIKA_GROUP) /var/lib/pika $(PIKA_OPT_DIR) $(PIKA_OPT_DIR)/target $(PIKA_OPT_DIR)/target/release $(PIKA_OPT_DIR)/frontend-web
	sudo install -d -m 0750 -o root -g $(PIKA_GROUP) $(PIKA_ETC_DIR)
	sudo install -m 0755 -o $(PIKA_USER) -g $(PIKA_GROUP) target/release/pika $(PIKA_OPT_DIR)/target/release/pika
	sudo rm -rf $(PIKA_OPT_DIR)/frontend-web/dist
	sudo cp -r frontend-web/dist $(PIKA_OPT_DIR)/frontend-web/
	sudo chown -R $(PIKA_USER):$(PIKA_GROUP) $(PIKA_OPT_DIR)/frontend-web/dist
	@if [ ! -f $(PIKA_ETC_DIR)/config.toml ]; then \
		if [ -f config.toml ]; then \
			sudo install -m 0640 -o root -g $(PIKA_GROUP) config.toml $(PIKA_ETC_DIR)/config.toml; \
		else \
			sudo install -m 0640 -o root -g $(PIKA_GROUP) config.toml.example $(PIKA_ETC_DIR)/config.toml; \
		fi; \
	fi
	@if [ ! -f $(PIKA_ETC_DIR)/pika.env ]; then \
		sudo install -m 0640 -o root -g $(PIKA_GROUP) /dev/null $(PIKA_ETC_DIR)/pika.env; \
	fi
	@echo "✅ Runtime artifacts staged"

# Install systemd services (requires artifacts from stage-runtime)
install-service: stage-runtime
	@echo "Installing systemd services (requires sudo)..."
	sudo cp cloudflared-pi.service /etc/systemd/system/
	sudo cp pika.service /etc/systemd/system/
	sudo systemctl daemon-reload
	sudo systemctl enable cloudflared-pi.service
	sudo systemctl enable pika.service
	@echo "✅ Systemd services installed"

# Restart system services
restart-service:
	@echo "Restarting systemd services..."
	sudo systemctl restart cloudflared-pi.service
	sudo systemctl restart pika.service
	@echo "✅ Services restarted"

# Deploy: Build everything, stage runtime artifacts, install and start services
deploy: build install-service
	@echo "🚀 Deploying your-domain.example..."
	@echo "Stopping any existing unmanaged pika process..."
	-pkill -f "target/release/pika|/opt/pika/target/release/pika" || true
	@sleep 1
	sudo systemctl start cloudflared-pi.service
	sudo systemctl restart pika.service
	@echo "✅ Deployment complete"

# Deploy without sudo using user-level systemd services
deploy-user: build
	@echo "🚀 Deploying with user systemd services (no sudo)..."
	@echo "Installing user systemd services..."
	@mkdir -p $(HOME)/.config/systemd/user
	cp cloudflared-pi.user.service $(HOME)/.config/systemd/user/cloudflared-pi.service
	cp pika.user.service $(HOME)/.config/systemd/user/pika.service
	systemctl --user daemon-reload
	@echo "Stopping any existing pika process on port 7847..."
	-pkill -f pika || true
	@echo "Waiting for port to be released..."
	@sleep 1
	@echo "Enabling and starting user services..."
	systemctl --user enable cloudflared-pi.service
	systemctl --user start cloudflared-pi.service
	systemctl --user enable pika.service
	systemctl --user start pika.service
	systemctl --user restart pika.service
	@echo "✅ User services restarted"

# Install user systemd services without starting
install-service-user:
	@echo "Installing user systemd services (no sudo)..."
	@mkdir -p $(HOME)/.config/systemd/user
	cp cloudflared-pi.user.service $(HOME)/.config/systemd/user/cloudflared-pi.service
	cp pika.user.service $(HOME)/.config/systemd/user/pika.service
	systemctl --user daemon-reload
	@echo "✅ User services installed"

# Restart user services
restart-service-user:
	@echo "Restarting user systemd services (no sudo)..."
	systemctl --user restart cloudflared-pi.service
	systemctl --user restart pika.service
	@echo "✅ User services restarted"

# Check service status
status:
	@echo "=== Cloudflare Tunnel ==="
	sudo systemctl status cloudflared-pi.service --no-pager -l
	@echo ""
	@echo "=== Pika ==="
	sudo systemctl status pika.service --no-pager -l

# Check user service status
status-user:
	@echo "=== Cloudflare Tunnel (user) ==="
	systemctl --user status cloudflared-pi.service --no-pager -l
	@echo ""
	@echo "=== Pika (user) ==="
	systemctl --user status pika.service --no-pager -l

# Help target
help:
	@echo "Available targets:"
	@echo "  make all             - Build frontend and backend (default)"
	@echo "  make build           - Same as 'make all'"
	@echo "  make frontend        - Build frontend only"
	@echo "  make backend         - Build backend only"
	@echo "  make dev-frontend    - Start frontend dev server (Vite)"
	@echo "  make dev-backend     - Start backend with hot reload (port 7847)"
	@echo "  make clean           - Remove build artifacts"
	@echo "  make run             - Build and run production server"
	@echo "  make test-install    - Install E2E test dependencies"
	@echo "  make test            - Run all E2E tests (server must be running)"
	@echo "  make test-mobile     - Run mobile E2E tests with visible browser"
	@echo "  make deploy          - Build, stage runtime layout, and deploy to production"
	@echo "  make deploy-user     - Build and deploy using user systemd services (no sudo)"
	@echo "  make stage-runtime   - Stage binary/frontend/config under /opt/pika + /etc/pika"
	@echo "  make install-service - Install systemd services (expects built artifacts)"
	@echo "  make install-service-user - Install user systemd services only"
	@echo "  make restart-service - Restart systemd services"
	@echo "  make restart-service-user - Restart user systemd services"
	@echo "  make status          - Check service status"
	@echo "  make status-user     - Check user service status"
	@echo "  make help            - Show this help message"
