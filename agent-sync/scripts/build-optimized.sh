#!/bin/bash
# High-performance build script for agent-sync
# Uses all available optimizations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect OS
OS="unknown"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="windows"
fi

print_info "Detected OS: $OS"

# Set number of parallel jobs based on CPU cores
if command -v nproc &> /dev/null; then
    JOBS=$(nproc)
elif command -v sysctl &> /dev/null; then
    JOBS=$(sysctl -n hw.ncpu)
else
    JOBS=4
fi

print_info "Using $JOBS parallel jobs"

# Check for sccache and start if available
if command -v sccache &> /dev/null; then
    print_info "Starting sccache..."
    export RUSTC_WRAPPER=sccache
    sccache --start-server || true
    sccache --show-stats
else
    print_warn "sccache not found, builds will be slower"
fi

# Set optimal rust flags
export RUSTFLAGS="-C target-cpu=native -C opt-level=3"

# Add mold linker flags on Linux
if [[ "$OS" == "linux" ]] && command -v mold &> /dev/null; then
    print_info "Using mold linker for faster linking"
    export RUSTFLAGS="$RUSTFLAGS -C link-arg=-fuse-ld=mold"
fi

# Parse command line arguments
PROFILE="release"
FEATURES=""
CLEAN=false
STRIP=false
LTO="fat"

while [[ $# -gt 0 ]]; do
    case $1 in
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --features)
            FEATURES="--features $2"
            shift 2
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --strip)
            STRIP=true
            shift
            ;;
        --lto)
            LTO="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --profile <profile>    Build profile (debug, release, production)"
            echo "  --features <features>  Comma-separated list of features"
            echo "  --clean               Clean before building"
            echo "  --strip               Strip symbols from binary"
            echo "  --lto <type>          LTO type (thin, fat, off)"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Clean if requested
if [ "$CLEAN" = true ]; then
    print_info "Cleaning build artifacts..."
    cargo clean
fi

# Set LTO in environment
export CARGO_PROFILE_RELEASE_LTO="$LTO"

# Build command
BUILD_CMD="cargo build --workspace --profile $PROFILE $FEATURES -j $JOBS"

print_info "Building with command: $BUILD_CMD"
print_info "Build configuration:"
echo "  Profile: $PROFILE"
echo "  Features: ${FEATURES:-default}"
echo "  LTO: $LTO"
echo "  Jobs: $JOBS"
echo "  Rust flags: $RUSTFLAGS"

# Start timer
START_TIME=$(date +%s)

# Execute build
$BUILD_CMD

# Calculate build time
END_TIME=$(date +%s)
BUILD_TIME=$((END_TIME - START_TIME))
print_info "Build completed in ${BUILD_TIME} seconds"

# Strip symbols if requested
if [ "$STRIP" = true ] && [ "$PROFILE" = "release" -o "$PROFILE" = "production" ]; then
    print_info "Stripping symbols from binaries..."
    if [[ "$OS" == "linux" ]] || [[ "$OS" == "macos" ]]; then
        find target/$PROFILE -maxdepth 1 -type f -executable -exec strip {} \;
    fi
fi

# Show binary sizes
print_info "Binary sizes:"
if [[ "$OS" == "windows" ]]; then
    ls -lh target/$PROFILE/*.exe 2>/dev/null || true
else
    find target/$PROFILE -maxdepth 1 -type f -executable -exec ls -lh {} \; 2>/dev/null || true
fi

# Show sccache stats if available
if command -v sccache &> /dev/null; then
    print_info "sccache statistics:"
    sccache --show-stats
fi

print_info "✅ Build complete!"
