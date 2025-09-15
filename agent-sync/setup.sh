#!/bin/bash
# Bash script to setup the high-performance build environment
# Run this script to install all required tools

set -e

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

echo -e "${CYAN}==================================${NC}"
echo -e "${CYAN}agent-sync Build System Setup${NC}"
echo -e "${CYAN}==================================${NC}"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install Rust if not present
if ! command_exists cargo; then
    echo -e "\n${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo -e "\n${YELLOW}Installing build tools...${NC}"

# Install cargo-make
if ! command_exists cargo-make; then
    echo -e "${GREEN}Installing cargo-make...${NC}"
    cargo install cargo-make
fi

# Install just
if ! command_exists just; then
    echo -e "${GREEN}Installing just...${NC}"
    cargo install just
fi

# Install cargo-nextest
if ! command_exists cargo-nextest; then
    echo -e "${GREEN}Installing cargo-nextest...${NC}"
    cargo install cargo-nextest
fi

# Install sccache
if ! command_exists sccache; then
    echo -e "${GREEN}Installing sccache...${NC}"
    cargo install sccache
fi

# Install hyperfine for benchmarking
if ! command_exists hyperfine; then
    echo -e "${GREEN}Installing hyperfine...${NC}"
    cargo install hyperfine
fi

# Install additional tools
echo -e "\n${YELLOW}Installing additional development tools...${NC}"
cargo install cargo-watch --quiet 2>/dev/null || true
cargo install cargo-edit --quiet 2>/dev/null || true
cargo install cargo-outdated --quiet 2>/dev/null || true
cargo install cargo-audit --quiet 2>/dev/null || true
cargo install cargo-deny --quiet 2>/dev/null || true

# Setup sccache and mold
echo -e "\n${YELLOW}Configuring sccache and mold linker...${NC}"
./scripts/setup-sccache.sh

# Verify installation
echo -e "\n${GREEN}==================================${NC}"
echo -e "${GREEN}Installation Complete!${NC}"
echo -e "${GREEN}==================================${NC}"

echo -e "\n${CYAN}Installed tools:${NC}"
echo -e "${WHITE}  ✓ cargo-make - Task automation${NC}"
echo -e "${WHITE}  ✓ just - Command runner${NC}"
echo -e "${WHITE}  ✓ cargo-nextest - Fast test runner${NC}"
echo -e "${WHITE}  ✓ sccache - Build caching${NC}"
echo -e "${WHITE}  ✓ mold - Fast linker (if on Linux)${NC}"
echo -e "${WHITE}  ✓ hyperfine - Benchmarking tool${NC}"

echo -e "\n${CYAN}Quick start commands:${NC}"
echo -e "${WHITE}  cargo make dev     - Development build cycle${NC}"
echo -e "${WHITE}  cargo make test    - Run tests${NC}"
echo -e "${WHITE}  cargo make ci      - Run CI pipeline${NC}"
echo -e "${WHITE}  just build        - Build project${NC}"
echo -e "${WHITE}  just test         - Run tests with nextest${NC}"

echo -e "\n${YELLOW}For more commands, run:${NC}"
echo -e "${WHITE}  cargo make --list-all-steps${NC}"
echo -e "${WHITE}  just --list${NC}"

echo -e "\n${GREEN}Refer to BUILD.md for detailed documentation.${NC}"

# Reminder to reload shell
echo -e "\n${YELLOW}NOTE: Run 'source ~/.bashrc' or restart your terminal for all changes to take effect.${NC}"
