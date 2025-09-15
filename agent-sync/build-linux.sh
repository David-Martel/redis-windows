#!/bin/bash
# Cross-platform build script for agent-sync Rust project on Linux/WSL

set -euo pipefail

# Default values
TARGET="${1:-x86_64-unknown-linux-gnu}"
BUILD_MODE="${2:-debug}"
RUN_TESTS="${3:-false}"
CLEAN_BUILD="${4:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}==> $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Verify environment
verify_environment() {
    log_info "Verifying build environment"

    # Check Rust installation
    if ! command -v rustc &> /dev/null; then
        log_error "Rust is not installed"
        exit 1
    fi

    local rust_version=$(rustc --version)
    log_success "Rust: $rust_version"

    # Check Cargo installation
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo is not installed"
        exit 1
    fi

    local cargo_version=$(cargo --version)
    log_success "Cargo: $cargo_version"

    # Check if target is installed
    if ! rustup target list --installed | grep -q "$TARGET"; then
        log_warning "Target $TARGET not installed. Installing..."
        rustup target add "$TARGET"
        log_success "Target $TARGET installed"
    else
        log_success "Target $TARGET is available"
    fi

    # Check for required system dependencies
    if [[ "$TARGET" == *"musl"* ]]; then
        if ! command -v musl-gcc &> /dev/null; then
            log_warning "musl-gcc not found. Installing musl-tools..."
            if command -v apt-get &> /dev/null; then
                sudo apt-get update && sudo apt-get install -y musl-tools musl-dev
            elif command -v yum &> /dev/null; then
                sudo yum install -y musl-gcc musl-devel
            else
                log_error "Cannot install musl tools automatically. Please install musl-tools manually."
                exit 1
            fi
        fi
    fi
}

# Clean build artifacts
clean_build() {
    log_info "Cleaning build artifacts"

    if [ -d "target" ]; then
        rm -rf target
        log_success "Removed target directory"
    fi

    if [ -f "Cargo.lock" ]; then
        rm -f Cargo.lock
        log_success "Removed Cargo.lock"
    fi

    cargo clean
    log_success "Cleaned Cargo cache"
}

# Build the project
build_project() {
    log_info "Building agent-sync for target: $TARGET"

    local build_args=("build" "--target" "$TARGET")

    if [ "$BUILD_MODE" = "release" ]; then
        build_args+=("--release")
        log_warning "Building in RELEASE mode"
    else
        log_warning "Building in DEBUG mode"
    fi

    # Set environment variables for cross-compilation
    export PKG_CONFIG_ALLOW_CROSS=1

    if [[ "$TARGET" == *"musl"* ]]; then
        export CC_x86_64_unknown_linux_musl=musl-gcc
        export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
    fi

    log_info "Executing: cargo ${build_args[*]}"

    local build_start=$(date +%s)
    cargo "${build_args[@]}"

    local build_end=$(date +%s)
    local build_time=$((build_end - build_start))
    log_success "Build completed in ${build_time} seconds"

    # Show binary information
    local profile="debug"
    if [ "$BUILD_MODE" = "release" ]; then
        profile="release"
    fi

    local binary_path="target/$TARGET/$profile"
    if [ -d "$binary_path" ]; then
        log_info "Build artifacts:"
        for file in "$binary_path"/*; do
            if [ -f "$file" ]; then
                local size=$(du -h "$file" | cut -f1)
                log_success "  📦 $(basename "$file") ($size)"
            fi
        done
    fi
}

# Run tests
run_tests() {
    log_info "Running tests"

    local test_args=("test" "--target" "$TARGET")

    if [ "$BUILD_MODE" = "release" ]; then
        test_args+=("--release")
    fi

    log_info "Executing: cargo ${test_args[*]}"

    local test_start=$(date +%s)
    cargo "${test_args[@]}"

    local test_end=$(date +%s)
    local test_time=$((test_end - test_start))
    log_success "All tests passed in ${test_time} seconds"
}

# Main function
main() {
    echo "🚀 Agent-Sync Linux/WSL Build Script"
    echo "Target: $TARGET"
    echo "Build Mode: $BUILD_MODE"
    echo "Run Tests: $RUN_TESTS"
    echo "Clean Build: $CLEAN_BUILD"
    echo ""

    local script_start=$(date +%s)

    verify_environment

    if [ "$CLEAN_BUILD" = "true" ]; then
        clean_build
    fi

    build_project

    if [ "$RUN_TESTS" = "true" ]; then
        run_tests
    fi

    local script_end=$(date +%s)
    local total_time=$((script_end - script_start))

    echo ""
    log_success "🎉 Build completed successfully in ${total_time} seconds"

    # Build summary
    log_info "Build Summary"
    log_success "  Target: $TARGET"
    log_success "  Mode: $BUILD_MODE"
    log_success "  Tests: $([ "$RUN_TESTS" = "true" ] && echo "Passed" || echo "Skipped")"
}

# Show usage if help is requested
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    echo "Usage: $0 [TARGET] [BUILD_MODE] [RUN_TESTS] [CLEAN_BUILD]"
    echo ""
    echo "Arguments:"
    echo "  TARGET      Target architecture (default: x86_64-unknown-linux-gnu)"
    echo "  BUILD_MODE  Build mode: debug or release (default: debug)"
    echo "  RUN_TESTS   Run tests: true or false (default: false)"
    echo "  CLEAN_BUILD Clean before build: true or false (default: false)"
    echo ""
    echo "Examples:"
    echo "  $0                                          # Build with defaults"
    echo "  $0 x86_64-unknown-linux-musl release       # Release build for musl"
    echo "  $0 x86_64-unknown-linux-gnu debug true     # Debug build with tests"
    echo "  $0 aarch64-unknown-linux-gnu release true true # Clean release build with tests"
    exit 0
fi

# Execute main function
main "$@"
