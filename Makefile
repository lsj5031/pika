.PHONY: all build frontend-backend frontend backend clean dev-frontend dev-backend run help

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
	./target/release/pi-agent-manager

# Help target
help:
	@echo "Available targets:"
	@echo "  make all          - Build frontend and backend (default)"
	@echo "  make build        - Same as 'make all'"
	@echo "  make frontend     - Build frontend only"
	@echo "  make backend      - Build backend only"
	@echo "  make dev-frontend - Start frontend dev server (Vite)"
	@echo "  make dev-backend  - Start backend with hot reload"
	@echo "  make clean        - Remove build artifacts"
	@echo "  make run          - Build and run production server"
	@echo "  make help         - Show this help message"
