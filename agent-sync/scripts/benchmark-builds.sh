#!/bin/bash
# Benchmark script to measure build performance improvements
# Requires: hyperfine (cargo install hyperfine)

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}Build Performance Benchmark${NC}"
echo "================================"

# Check if hyperfine is installed
if ! command -v hyperfine &> /dev/null; then
    echo -e "${YELLOW}Installing hyperfine...${NC}"
    cargo install hyperfine
fi

# Clean before benchmarking
cargo clean

echo -e "\n${GREEN}1. Benchmarking standard build vs sccache build${NC}"
hyperfine --warmup 1 --runs 3 \
    --prepare 'cargo clean' \
    'cargo build --workspace' \
    'RUSTC_WRAPPER=sccache cargo build --workspace' \
    --export-markdown target/build-benchmark.md

echo -e "\n${GREEN}2. Benchmarking different optimization levels${NC}"
hyperfine --warmup 1 --runs 3 \
    --prepare 'cargo clean' \
    'cargo build --profile dev-fast --workspace' \
    'cargo build --profile dev --workspace' \
    'cargo build --profile dev-opt --workspace' \
    --export-json target/opt-benchmark.json

echo -e "\n${GREEN}3. Benchmarking incremental vs full builds${NC}"
# First build
cargo build --workspace
# Modify a file
touch src/lib.rs
# Benchmark incremental
hyperfine --warmup 1 --runs 5 \
    'cargo build --workspace' \
    --export-csv target/incremental-benchmark.csv

echo -e "\n${GREEN}4. Benchmarking test execution: cargo test vs nextest${NC}"
hyperfine --warmup 1 --runs 3 \
    'cargo test --workspace' \
    'cargo nextest run --workspace' \
    --export-markdown target/test-benchmark.md

echo -e "\n${GREEN}Results saved to:${NC}"
echo "  - target/build-benchmark.md"
echo "  - target/opt-benchmark.json"
echo "  - target/incremental-benchmark.csv"
echo "  - target/test-benchmark.md"

# Generate summary
echo -e "\n${GREEN}Build Performance Summary${NC}"
echo "========================="
if [ -f target/build-benchmark.md ]; then
    cat target/build-benchmark.md
fi
