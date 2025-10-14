#!/bin/bash
# Setup git hooks for rCore-Coding project

set -e

echo "🔧 Setting up git hooks for rCore-Coding..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="$SCRIPT_DIR/.git/hooks"

# Check if we're in a git repository
if [ ! -d "$SCRIPT_DIR/.git" ]; then
    echo "❌ Error: Not in a git repository"
    exit 1
fi

# Create the pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook for rCore-Coding
# Runs formatting and clippy checks before allowing commit

set -e

echo "🔍 Running pre-commit checks..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track if any checks fail
FAILED=0

# Function to run a check
run_check() {
    local name=$1
    local cmd=$2
    local dir=$3

    echo -e "${YELLOW}Running $name...${NC}"
    if [ -n "$dir" ]; then
        cd "$dir"
    fi

    if eval "$cmd"; then
        echo -e "${GREEN}✓ $name passed${NC}"
    else
        echo -e "${RED}✗ $name failed${NC}"
        FAILED=1
    fi

    if [ -n "$dir" ]; then
        cd - > /dev/null
    fi
}

# Get the repo root
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

echo ""
echo "📝 Checking code formatting..."
run_check "OS formatting" "cargo fmt --all --check" "os"
run_check "User formatting" "cargo fmt --all --check" "user"

echo ""
echo "🔧 Running Clippy lints..."
run_check "OS clippy" "cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings" "os"
run_check "User clippy" "cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings" "user"

echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed! Proceeding with commit.${NC}"
    exit 0
else
    echo -e "${RED}✗ Some checks failed. Please fix the issues before committing.${NC}"
    echo ""
    echo "To fix formatting issues, run:"
    echo "  cd os && cargo fmt --all"
    echo "  cd user && cargo fmt --all"
    echo ""
    echo "To skip these checks (not recommended), use:"
    echo "  git commit --no-verify"
    exit 1
fi
EOF

# Make the hook executable
chmod +x "$HOOKS_DIR/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The pre-commit hook will now run automatically before each commit."
echo "It will check:"
echo "  - Code formatting (rustfmt)"
echo "  - Clippy lints (with -D warnings)"
echo ""
echo "To skip the hook (not recommended), use: git commit --no-verify"
