.PHONY: all build frontend-backend frontend backend clean dev-frontend dev-backend run test test-mobile test-install help

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

# Help target
help:
	@echo "Available targets:"
	@echo "  make all          - Build frontend and backend (default)"
	@echo "  make build        - Same as 'make all'"
	@echo "  make frontend     - Build frontend only"
	@echo "  make backend      - Build backend only"
	@echo "  make dev-frontend - Start frontend dev server (Vite)"
	@echo "  make dev-backend  - Start backend with hot reload (port 7847)"
	@echo "  make clean        - Remove build artifacts"
	@echo "  make run          - Build and run production server"
	@echo "  make test-install - Install E2E test dependencies"
	@echo "  make test         - Run all E2E tests (server must be running)"
	@echo "  make test-mobile  - Run mobile E2E tests with visible browser"
	@echo "  make help         - Show this help message"
