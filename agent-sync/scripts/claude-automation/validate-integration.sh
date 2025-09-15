#!/bin/bash
# Validation script for Claude automation integration in agent-sync project

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "🔍 Claude Automation Integration Validator"
echo "Project root: $PROJECT_ROOT"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

check_file() {
    local file="$1"
    local description="$2"

    if [ -f "$file" ]; then
        echo -e "${GREEN}✅${NC} $description: $file"
        return 0
    else
        echo -e "${RED}❌${NC} $description: $file (missing)"
        return 1
    fi
}

check_dir() {
    local dir="$1"
    local description="$2"

    if [ -d "$dir" ]; then
        echo -e "${GREEN}✅${NC} $description: $dir"
        return 0
    else
        echo -e "${RED}❌${NC} $description: $dir (missing)"
        return 1
    fi
}

check_executable() {
    local file="$1"
    local description="$2"

    if [ -x "$file" ]; then
        echo -e "${GREEN}✅${NC} $description: $file (executable)"
        return 0
    else
        echo -e "${YELLOW}⚠️${NC} $description: $file (not executable)"
        return 1
    fi
}

echo -e "\n${BLUE}📋 Phase 1: Core Configuration Files${NC}"
total_checks=0
passed_checks=0

# AST-grep configuration
((total_checks++))
if check_file "sgconfig.yml" "AST-grep configuration"; then ((passed_checks++)); fi

((total_checks++))
if check_file ".ast-greprc" "AST-grep user preferences"; then ((passed_checks++)); fi

echo -e "\n${BLUE}📋 Phase 2: AST-grep Rules Directory${NC}"

((total_checks++))
if check_dir ".ast-grep" "AST-grep directory"; then ((passed_checks++)); fi

((total_checks++))
if check_dir ".ast-grep/rules" "AST-grep rules directory"; then ((passed_checks++)); fi

# Check individual rule files
rules=(
    "rust-memory-safety.yml"
    "rust-performance.yml"
    "rust-error-handling.yml"
    "rust-async-patterns.yml"
    "rust-redis-patterns.yml"
    "rust-style-fixes.yml"
)

for rule in "${rules[@]}"; do
    ((total_checks++))
    if check_file ".ast-grep/rules/$rule" "Rule: $rule"; then ((passed_checks++)); fi
done

echo -e "\n${BLUE}📋 Phase 3: GitHub Workflows${NC}"

((total_checks++))
if check_dir ".github" "GitHub directory"; then ((passed_checks++)); fi

((total_checks++))
if check_dir ".github/workflows" "GitHub workflows directory"; then ((passed_checks++)); fi

((total_checks++))
if check_file ".github/workflows/ast-grep-rust.yml" "AST-grep Rust workflow"; then ((passed_checks++)); fi

((total_checks++))
if check_file ".github/workflows/rust-quality-gate.yml" "Rust quality gate workflow"; then ((passed_checks++)); fi

echo -e "\n${BLUE}📋 Phase 4: Semantic Indexing${NC}"

((total_checks++))
if check_dir ".semantic_index" "Semantic index directory"; then ((passed_checks++)); fi

((total_checks++))
if check_file ".semantic_index/index.json" "Semantic index configuration"; then ((passed_checks++)); fi

((total_checks++))
if check_file ".semantic_index/manifest.json" "Semantic index manifest"; then ((passed_checks++)); fi

((total_checks++))
if check_file ".semantic_index/symbols.json" "Semantic symbols index"; then ((passed_checks++)); fi

echo -e "\n${BLUE}📋 Phase 5: Claude Automation Scripts${NC}"

((total_checks++))
if check_dir "scripts" "Scripts directory"; then ((passed_checks++)); fi

((total_checks++))
if check_dir "scripts/claude-automation" "Claude automation scripts directory"; then ((passed_checks++)); fi

((total_checks++))
if check_file "scripts/claude-automation/rust-auto-fix.sh" "Rust auto-fix script (bash)"; then
    ((passed_checks++))
    if check_executable "scripts/claude-automation/rust-auto-fix.sh" "Rust auto-fix script executable"; then
        echo -e "${GREEN}  Script is executable${NC}"
    else
        echo -e "${YELLOW}  Run: chmod +x scripts/claude-automation/rust-auto-fix.sh${NC}"
    fi
fi

echo -e "\n${BLUE}📋 Phase 6: Rust Project Validation${NC}"

# Check if it's a valid Rust project
((total_checks++))
if check_file "Cargo.toml" "Cargo.toml"; then ((passed_checks++)); fi

# Check Rust compilation
echo -e "${BLUE}Checking Rust compilation...${NC}"
if cargo check --all-targets --all-features > /dev/null 2>&1; then
    echo -e "${GREEN}✅${NC} Rust project compiles successfully"
    ((total_checks++))
    ((passed_checks++))
else
    echo -e "${RED}❌${NC} Rust project has compilation errors"
    ((total_checks++))
fi

echo -e "\n${BLUE}📋 Phase 7: Tool Availability${NC}"

# Check for required tools
tools=("cargo" "rustc" "git")
for tool in "${tools[@]}"; do
    ((total_checks++))
    if command -v "$tool" &> /dev/null; then
        echo -e "${GREEN}✅${NC} $tool is available: $(command -v "$tool")"
        ((passed_checks++))
    else
        echo -e "${RED}❌${NC} $tool is not available"
    fi
done

# Check for AST-grep
((total_checks++))
if command -v sg &> /dev/null; then
    echo -e "${GREEN}✅${NC} ast-grep (sg) is available: $(sg --version)"
    ((passed_checks++))
else
    echo -e "${YELLOW}⚠️${NC} ast-grep (sg) is not available - install with: cargo install ast-grep"
fi

echo -e "\n${BLUE}📋 Phase 8: Integration Test${NC}"

# Test AST-grep configuration
((total_checks++))
if [ -f "sgconfig.yml" ] && command -v sg &> /dev/null; then
    if sg scan --config sgconfig.yml --lang rust --dry-run > /dev/null 2>&1; then
        echo -e "${GREEN}✅${NC} AST-grep configuration is valid"
        ((passed_checks++))
    else
        echo -e "${RED}❌${NC} AST-grep configuration has errors"
    fi
else
    echo -e "${YELLOW}⚠️${NC} Cannot test AST-grep configuration (missing sgconfig.yml or ast-grep)"
fi

# Summary
echo -e "\n${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}📊 VALIDATION SUMMARY${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"

if [ $passed_checks -eq $total_checks ]; then
    echo -e "${GREEN}🎉 ALL CHECKS PASSED: $passed_checks/$total_checks${NC}"
    echo -e "${GREEN}Claude automation integration is complete and ready!${NC}"
    exit 0
elif [ $passed_checks -gt $((total_checks * 80 / 100)) ]; then
    echo -e "${YELLOW}⚠️  MOSTLY COMPLETE: $passed_checks/$total_checks passed ($(((passed_checks * 100) / total_checks))%)${NC}"
    echo -e "${YELLOW}Some optional components are missing but core functionality is available${NC}"
    exit 1
else
    echo -e "${RED}❌ INTEGRATION INCOMPLETE: $passed_checks/$total_checks passed ($(((passed_checks * 100) / total_checks))%)${NC}"
    echo -e "${RED}Critical components are missing. Please complete the setup.${NC}"
    exit 2
fi