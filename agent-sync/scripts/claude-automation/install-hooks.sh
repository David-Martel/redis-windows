#!/bin/bash
# Install and configure pre-commit hooks for agent-sync project
# Integrates with Claude framework automation tools

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "🔧 Installing Claude Framework Pre-commit Hooks for agent-sync"
echo "Project root: $PROJECT_ROOT"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}❌ Not in a git repository${NC}"
    exit 1
fi

# Check if we're in a Rust project
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ No Cargo.toml found. Not a Rust project?${NC}"
    exit 1
fi

echo -e "${BLUE}📋 Phase 1: Installing pre-commit${NC}"

# Check if pre-commit is installed
if command -v pre-commit &> /dev/null; then
    echo -e "${GREEN}✅ pre-commit is already installed: $(pre-commit --version)${NC}"
else
    echo -e "${YELLOW}⚠️ pre-commit not found, installing...${NC}"

    # Try different installation methods
    if command -v pip &> /dev/null; then
        pip install pre-commit
    elif command -v pip3 &> /dev/null; then
        pip3 install pre-commit
    elif command -v brew &> /dev/null; then
        brew install pre-commit
    elif command -v apt-get &> /dev/null; then
        sudo apt-get update && sudo apt-get install -y pre-commit
    elif command -v yum &> /dev/null; then
        sudo yum install -y pre-commit
    else
        echo -e "${RED}❌ Cannot install pre-commit automatically. Please install manually:${NC}"
        echo "  pip install pre-commit"
        echo "  or visit: https://pre-commit.com/#installation"
        exit 1
    fi
fi

echo -e "${BLUE}📋 Phase 2: Installing Rust tools${NC}"

# Install required Rust tools
rust_tools=(
    "cargo-audit"
    "cargo-deny"
    "cargo-sort"
)

for tool in "${rust_tools[@]}"; do
    if command -v "$tool" &> /dev/null; then
        echo -e "${GREEN}✅ $tool is already installed${NC}"
    else
        echo -e "${YELLOW}⚠️ Installing $tool...${NC}"
        cargo install "$tool" || echo -e "${YELLOW}⚠️ Failed to install $tool (non-critical)${NC}"
    fi
done

echo -e "${BLUE}📋 Phase 3: Installing AST-grep${NC}"

# Install AST-grep if not present
if command -v sg &> /dev/null; then
    echo -e "${GREEN}✅ ast-grep is already installed: $(sg --version)${NC}"
else
    echo -e "${YELLOW}⚠️ Installing ast-grep...${NC}"
    cargo install ast-grep --locked
fi

echo -e "${BLUE}📋 Phase 4: Validating configuration${NC}"

# Check if .pre-commit-config.yaml exists
if [ ! -f ".pre-commit-config.yaml" ]; then
    echo -e "${RED}❌ .pre-commit-config.yaml not found${NC}"
    echo "This should have been created by the integration script."
    exit 1
else
    echo -e "${GREEN}✅ .pre-commit-config.yaml found${NC}"
fi

# Validate the configuration
echo -e "${BLUE}Validating pre-commit configuration...${NC}"
if pre-commit validate-config; then
    echo -e "${GREEN}✅ Pre-commit configuration is valid${NC}"
else
    echo -e "${RED}❌ Pre-commit configuration is invalid${NC}"
    exit 1
fi

echo -e "${BLUE}📋 Phase 5: Installing pre-commit hooks${NC}"

# Install the pre-commit hook
pre-commit install

echo -e "${BLUE}📋 Phase 6: Initial hook run${NC}"

# Run pre-commit on all files to set up the environment
echo -e "${BLUE}Running pre-commit on all files (this may take a while)...${NC}"
if pre-commit run --all-files; then
    echo -e "${GREEN}✅ All pre-commit hooks passed${NC}"
else
    echo -e "${YELLOW}⚠️ Some pre-commit hooks failed or made changes${NC}"
    echo -e "${BLUE}This is normal for the first run. Hooks may have automatically fixed issues.${NC}"

    # Show what changed
    if ! git diff --quiet; then
        echo -e "${BLUE}Files that were automatically modified:${NC}"
        git diff --name-only | while read -r file; do
            echo -e "  ${YELLOW}Modified: $file${NC}"
        done

        echo -e "${BLUE}To commit these changes:${NC}"
        echo "  git add -A"
        echo "  git commit -m '🔧 Apply pre-commit auto-fixes'"
    fi
fi

echo -e "${BLUE}📋 Phase 7: Creating convenience scripts${NC}"

# Create a convenience script for running pre-commit
cat > "scripts/claude-automation/run-pre-commit.sh" << 'EOF'
#!/bin/bash
# Convenience script to run pre-commit hooks

set -euo pipefail

echo "🔧 Running pre-commit hooks..."

cd "$(git rev-parse --show-toplevel)"

# Run pre-commit on staged files
if [ "$#" -eq 0 ]; then
    echo "Running on staged files..."
    pre-commit run
else
    case "$1" in
        all)
            echo "Running on all files..."
            pre-commit run --all-files
            ;;
        *)
            echo "Running on specific files: $*"
            pre-commit run --files "$@"
            ;;
    esac
fi

echo "✅ Pre-commit hooks completed"
EOF

chmod +x "scripts/claude-automation/run-pre-commit.sh"

# Create a script to update hooks
cat > "scripts/claude-automation/update-hooks.sh" << 'EOF'
#!/bin/bash
# Update pre-commit hooks to latest versions

set -euo pipefail

echo "🔄 Updating pre-commit hooks..."

cd "$(git rev-parse --show-toplevel)"

# Update hook repositories
pre-commit autoupdate

# Clean and reinstall
pre-commit clean
pre-commit install

echo "✅ Pre-commit hooks updated"
EOF

chmod +x "scripts/claude-automation/update-hooks.sh"

echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}🎉 INSTALLATION COMPLETE${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"

echo -e "${BLUE}Pre-commit hooks are now installed and configured!${NC}"
echo ""
echo -e "${BLUE}Usage:${NC}"
echo "  • Hooks run automatically on every commit"
echo "  • Run manually: pre-commit run"
echo "  • Run on all files: pre-commit run --all-files"
echo "  • Convenience script: ./scripts/claude-automation/run-pre-commit.sh"
echo ""
echo -e "${BLUE}Installed hooks include:${NC}"
echo "  • AST-grep Rust auto-fixes and validation"
echo "  • Cargo format, fix, and clippy"
echo "  • Rust compilation and testing"
echo "  • Redis and async pattern checks"
echo "  • Security auditing"
echo "  • Documentation validation"
echo "  • File format validation"
echo ""
echo -e "${BLUE}To update hooks in the future:${NC}"
echo "  ./scripts/claude-automation/update-hooks.sh"

echo -e "${GREEN}Happy coding with automated quality checks! 🦀${NC}"