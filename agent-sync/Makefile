# Makefile for cross-platform agent-sync builds
# Compatible with GNU Make and Windows nmake

# Default settings
TARGET ?= x86_64-unknown-linux-gnu
BUILD_MODE ?= debug
DOCKER_TAG ?= agent-sync:latest

# Platform detection
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
    SHELL_EXT := .ps1
    CARGO_TARGET_DEFAULT := x86_64-pc-windows-msvc
    BUILD_SCRIPT := build-windows.ps1
else
    DETECTED_OS := $(shell uname -s)
    SHELL_EXT := .sh
    CARGO_TARGET_DEFAULT := x86_64-unknown-linux-gnu
    BUILD_SCRIPT := build-linux.sh
endif

# Common targets
.PHONY: help build test clean install docker dev ci release benchmark security fmt lint check

help: ## Show this help message
	@echo "Agent-Sync Cross-Platform Build System"
	@echo "Detected OS: $(DETECTED_OS)"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the project for current platform
ifeq ($(DETECTED_OS),Windows)
	@powershell -ExecutionPolicy Bypass -File .\build-windows.ps1
else
	@chmod +x build-linux.sh
	@./build-linux.sh
endif

build-release: ## Build in release mode
ifeq ($(DETECTED_OS),Windows)
	@powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 -Release
else
	@chmod +x build-linux.sh
	@./build-linux.sh $(CARGO_TARGET_DEFAULT) release
endif

test: ## Run tests
ifeq ($(DETECTED_OS),Windows)
	@powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 -Test
else
	@chmod +x build-linux.sh
	@./build-linux.sh $(CARGO_TARGET_DEFAULT) debug true
endif

test-release: ## Run tests in release mode
ifeq ($(DETECTED_OS),Windows)
	@powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 -Release -Test
else
	@chmod +x build-linux.sh
	@./build-linux.sh $(CARGO_TARGET_DEFAULT) release true
endif

clean: ## Clean build artifacts
ifeq ($(DETECTED_OS),Windows)
	@powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 -Clean
else
	@chmod +x build-linux.sh
	@./build-linux.sh $(CARGO_TARGET_DEFAULT) debug false true
endif
	@cargo clean

benchmark: ## Run benchmarks
	@cargo bench --all-features

install: ## Install the crate locally
	@cargo install --path . --force

fmt: ## Format code
	@cargo fmt --all

lint: ## Run linting with clippy
	@cargo clippy --all-features -- -D warnings

check: ## Quick compile check
	@cargo check --all-features

security: ## Run security audits
	@cargo audit || (echo "Installing cargo-audit..." && cargo install cargo-audit && cargo audit)
	@cargo deny check || (echo "Installing cargo-deny..." && cargo install cargo-deny && cargo deny check)

# Cross-compilation targets
build-windows: ## Build for Windows
	@cargo build --target x86_64-pc-windows-msvc --release

build-linux: ## Build for Linux GNU
	@cargo build --target x86_64-unknown-linux-gnu --release

build-linux-musl: ## Build for Linux MUSL (static)
	@cargo build --target x86_64-unknown-linux-musl --release

build-arm64: ## Build for ARM64 Linux
	@cargo build --target aarch64-unknown-linux-gnu --release

build-all: ## Build for all targets
	@echo "Building for all supported targets..."
	@rustup target add x86_64-pc-windows-msvc || true
	@rustup target add x86_64-unknown-linux-gnu || true
	@rustup target add x86_64-unknown-linux-musl || true
	@rustup target add aarch64-unknown-linux-gnu || true
	@cargo build --target x86_64-pc-windows-msvc --release || echo "Windows build failed"
	@cargo build --target x86_64-unknown-linux-gnu --release || echo "Linux GNU build failed"
	@cargo build --target x86_64-unknown-linux-musl --release || echo "Linux MUSL build failed"
	@cargo build --target aarch64-unknown-linux-gnu --release || echo "ARM64 build failed"

# Docker targets
docker-build: ## Build Docker image
	@docker build -t $(DOCKER_TAG) .

docker-build-dev: ## Build Docker development image
	@docker build --target builder -t $(DOCKER_TAG)-dev .

docker-build-test: ## Build Docker test image
	@docker build --target test -t $(DOCKER_TAG)-test .

docker-run: ## Run Docker container
	@docker-compose up -d redis
	@docker run --rm -it --network agent-sync_agent-sync-net $(DOCKER_TAG)

docker-dev: ## Start development environment with Docker Compose
	@docker-compose up -d redis agent-sync-dev
	@echo "Development environment started. Connect with:"
	@echo "docker exec -it agent-sync-dev bash"

docker-test: ## Run tests in Docker
	@docker-compose up --build agent-sync-test

docker-clean: ## Clean Docker artifacts
	@docker-compose down -v --remove-orphans
	@docker system prune -f

# CI targets
ci: fmt lint check test security ## Run full CI pipeline locally

ci-release: fmt lint check test-release security benchmark ## Run release CI pipeline

coverage: ## Generate code coverage report
	@cargo install cargo-llvm-cov || true
	@cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
	@echo "Coverage report generated: lcov.info"

# Development targets
dev: ## Start development environment
ifeq ($(DETECTED_OS),Windows)
	@echo "Windows development setup:"
	@echo "1. Start Redis: redis-server redis-agent.conf"
	@echo "2. Run: cargo run"
else
	@echo "Linux development setup:"
	@echo "1. Start Redis: redis-server redis-agent.conf"
	@echo "2. Run: cargo run"
endif

watch: ## Watch for changes and rebuild
	@cargo install cargo-watch || true
	@cargo watch -c -w src -x "check --all-features" -x "test --all-features"

# Release targets
release: clean build-all test-release security ## Prepare for release
	@echo "Release build completed"
	@echo "Artifacts available in target/ directories"

package: release ## Create release packages
ifeq ($(DETECTED_OS),Windows)
	@powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 -Release -Package
else
	@mkdir -p dist
	@tar -czf dist/agent-sync-linux-x86_64.tar.gz -C target/x86_64-unknown-linux-gnu/release .
	@echo "Linux package created: dist/agent-sync-linux-x86_64.tar.gz"
endif

publish: ## Publish to crates.io (requires API token)
	@cargo publish

# Installation helpers
install-deps: ## Install build dependencies
ifeq ($(DETECTED_OS),Windows)
	@echo "Windows dependencies:"
	@echo "1. Install Rust: winget install Rustlang.Rustup"
	@echo "2. Install MSVC: winget install Microsoft.VisualStudio.2022.BuildTools"
else
	@echo "Installing Linux dependencies..."
ifeq ($(shell command -v apt-get 2> /dev/null),)
	@echo "Please install: build-essential pkg-config libssl-dev musl-tools"
else
	@sudo apt-get update
	@sudo apt-get install -y build-essential pkg-config libssl-dev musl-tools musl-dev
endif
endif

install-tools: ## Install additional Rust tools
	@cargo install cargo-audit cargo-deny cargo-watch cargo-llvm-cov cross || true
	@rustup component add rustfmt clippy || true

# Documentation
docs: ## Generate documentation
	@cargo doc --all-features --open

docs-private: ## Generate documentation including private items
	@cargo doc --all-features --document-private-items --open

# Quick start
quick-start: install-deps install-tools build test ## Complete quick start setup
	@echo "Quick start completed successfully!"
	@echo "Run 'make dev' to start development"