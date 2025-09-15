#!/bin/bash
# Bash script to setup sccache and mold for Linux/WSL
# This script installs and configures sccache and mold linker for faster Rust compilation

set -e

echo -e "\033[32mSetting up high-performance build tools for Linux/WSL...\033[0m"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install sccache
if ! command_exists sccache; then
    echo -e "\033[33mInstalling sccache...\033[0m"
    cargo install sccache
else
    echo -e "\033[32msccache is already installed\033[0m"
fi

# Install mold linker (much faster than default linker)
if ! command_exists mold; then
    echo -e "\033[33mInstalling mold linker for faster linking...\033[0m"
    
    # Check if running on x86_64
    if [ "$(uname -m)" = "x86_64" ]; then
        # Download and install mold
        MOLD_VERSION=$(curl -s https://api.github.com/repos/rui314/mold/releases/latest | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
        curl -L "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/mold-${MOLD_VERSION}-x86_64-linux.tar.gz" | tar xzf -
        
        # Install to /usr/local
        if [ -w /usr/local/bin ]; then
            cp "mold-${MOLD_VERSION}-x86_64-linux/bin/mold" /usr/local/bin/
            cp "mold-${MOLD_VERSION}-x86_64-linux/lib/mold/mold-wrapper.so" /usr/local/lib/
        else
            sudo cp "mold-${MOLD_VERSION}-x86_64-linux/bin/mold" /usr/local/bin/
            sudo cp "mold-${MOLD_VERSION}-x86_64-linux/lib/mold/mold-wrapper.so" /usr/local/lib/
        fi
        
        # Cleanup
        rm -rf "mold-${MOLD_VERSION}-x86_64-linux"
        echo -e "\033[32mmold linker installed successfully\033[0m"
    else
        echo -e "\033[33mSkipping mold installation (not x86_64 architecture)\033[0m"
    fi
else
    echo -e "\033[32mmold linker is already installed\033[0m"
fi

# Configure sccache
echo -e "\033[33mConfiguring sccache...\033[0m"

# Set cache directory
CACHE_DIR="${HOME}/.cache/sccache"
mkdir -p "$CACHE_DIR"

# Export environment variables
export SCCACHE_DIR="$CACHE_DIR"
export RUSTC_WRAPPER="sccache"
export SCCACHE_CACHE_SIZE="10G"
export SCCACHE_IDLE_TIMEOUT="0"

# Add to shell configuration
SHELL_RC=""
if [ -f "$HOME/.bashrc" ]; then
    SHELL_RC="$HOME/.bashrc"
elif [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
fi

if [ -n "$SHELL_RC" ]; then
    # Check if already configured
    if ! grep -q "RUSTC_WRAPPER=sccache" "$SHELL_RC"; then
        echo "" >> "$SHELL_RC"
        echo "# Rust build optimization" >> "$SHELL_RC"
        echo "export SCCACHE_DIR=\"$CACHE_DIR\"" >> "$SHELL_RC"
        echo "export RUSTC_WRAPPER=\"sccache\"" >> "$SHELL_RC"
        echo "export SCCACHE_CACHE_SIZE=\"10G\"" >> "$SHELL_RC"
        echo "export SCCACHE_IDLE_TIMEOUT=\"0\"" >> "$SHELL_RC"
        echo "export CARGO_BUILD_JOBS=\"$(nproc)\"" >> "$SHELL_RC"
        echo "" >> "$SHELL_RC"
        echo "# Use mold linker for faster linking" >> "$SHELL_RC"
        echo "export RUSTFLAGS=\"-C link-arg=-fuse-ld=mold -C target-cpu=native\"" >> "$SHELL_RC"
        
        echo -e "\033[32mEnvironment variables added to $SHELL_RC\033[0m"
    else
        echo -e "\033[32mEnvironment variables already configured in $SHELL_RC\033[0m"
    fi
fi

# Configure cargo to use mold
CARGO_CONFIG="$HOME/.cargo/config.toml"
if [ ! -f "$CARGO_CONFIG" ]; then
    mkdir -p "$HOME/.cargo"
    cat > "$CARGO_CONFIG" << 'CONFIG'
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-unknown-linux-musl]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold", "-C", "link-arg=-static"]
CONFIG
    echo -e "\033[32mCargo configured to use mold linker\033[0m"
fi

# Start sccache server
echo -e "\033[33mStarting sccache server...\033[0m"
sccache --start-server || true

# Show sccache statistics
echo -e "\033[33m\nsccache statistics:\033[0m"
sccache --show-stats

# Install additional tools
echo -e "\033[33m\nInstalling additional build tools...\033[0m"

# Install cargo-nextest if not present
if ! command_exists cargo-nextest; then
    echo "Installing cargo-nextest..."
    cargo install cargo-nextest
fi

# Summary
echo -e "\033[32m\n✅ High-performance build setup complete!\033[0m"
echo -e "\033[32mInstalled:\033[0m"
echo "  - sccache (build caching)"
echo "  - mold (fast linker)"
echo "  - cargo-nextest (fast test runner)"
echo ""
echo -e "\033[33mEnvironment variables set:\033[0m"
echo "  SCCACHE_DIR: $CACHE_DIR"
echo "  RUSTC_WRAPPER: sccache"
echo "  SCCACHE_CACHE_SIZE: 10G"
echo "  CARGO_BUILD_JOBS: $(nproc)"
echo ""
echo -e "\033[33mNote: Restart your terminal or run 'source $SHELL_RC' for changes to take effect.\033[0m"
