#!/bin/bash

# Release Validation Script
# This script validates the project before release

set -e

echo "🔍 Log Analyzer Release Validation"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "success")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "warning")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "error")
            echo -e "${RED}❌ $message${NC}"
            ;;
    esac
}

# Check if we're in the right directory
if [[ ! -f "log-analyzer/package.json" ]]; then
    print_status "error" "This script must be run from the project root directory"
    exit 1
fi

# 1. Version Consistency Check
print_status "success" "Step 1: Checking version consistency..."

RELEASE_PLAN_OUTPUT=$(node scripts/prepare-release.mjs plan)
PACKAGE_VERSION=$(printf '%s\n' "$RELEASE_PLAN_OUTPUT" | awk -F= '/^currentVersion=/{print $2}')
TARGET_VERSION=$(printf '%s\n' "$RELEASE_PLAN_OUTPUT" | awk -F= '/^targetVersion=/{print $2}')
LATEST_TAG=$(printf '%s\n' "$RELEASE_PLAN_OUTPUT" | awk -F= '/^latestTag=/{print $2}')
STRATEGY=$(printf '%s\n' "$RELEASE_PLAN_OUTPUT" | awk -F= '/^strategy=/{print $2}')

print_status "success" "Workspace version: $PACKAGE_VERSION"
print_status "success" "Latest tag: $LATEST_TAG"
print_status "success" "Planned release version: $TARGET_VERSION"
print_status "success" "Planning strategy: $STRATEGY"

# 2. Build Validation
print_status "success" "Step 2: Validating build process..."

cd log-analyzer

# Check Node.js version
NODE_VERSION=$(node --version)
REQUIRED_NODE="v22"
if [[ "$NODE_VERSION" != $REQUIRED_NODE* ]]; then
    print_status "warning" "Node.js version is $NODE_VERSION, recommended is $REQUIRED_NODE.x"
fi

# Install dependencies
print_status "success" "Installing dependencies..."
npm ci

# Type checking
print_status "success" "Running type check..."
npm run type-check

# Linting
print_status "success" "Running linter..."
npm run lint

# Build test
print_status "success" "Testing build..."
npm run build

# 3. Rust Validation
print_status "success" "Step 3: Validating Rust backend..."

cd src-tauri

# Format check
cargo fmt -- --check

# Clippy check
cargo clippy --all-features --all-targets -- -D warnings

# Test run
cargo test --all-features

cd ../..

# 4. Security Check
print_status "success" "Step 4: Security validation..."

cd log-analyzer/src-tauri

# Check for security advisories
cargo audit || print_status "warning" "Security audit completed with warnings"

# Check for outdated dependencies
cargo outdated || print_status "warning" "Some dependencies may be outdated"

cd ../..

# 5. Release Notes Check
print_status "success" "Step 5: Checking release notes..."

if [[ -f "CHANGELOG.md" ]]; then
    print_status "success" "CHANGELOG.md exists"
    
    # Check for recent changes
    LATEST_TAG=$(git tag -l "v*" --sort=-v:refname | head -n 1)
    if [[ -n "$LATEST_TAG" ]]; then
        CHANGES=$(git log --oneline "$LATEST_TAG"..HEAD --grep="^feat\|^fix\|^perf\|^refactor" | wc -l)
        if [[ $CHANGES -gt 0 ]]; then
            print_status "warning" "Found $CHANGES potentially unreleased changes"
        fi
    fi
else
    print_status "warning" "No CHANGELOG.md found"
fi

# 6. Git Status Check
print_status "success" "Step 6: Checking git status..."

if [[ -n $(git status --porcelain) ]]; then
    print_status "warning" "Working directory is not clean"
    git status --short
else
    print_status "success" "Working directory is clean"
fi

# 7. Final Summary
print_status "success" "=================================="
print_status "success" "Release validation completed!"
print_status "success" "Current version: $PACKAGE_VERSION"
print_status "success" "Ready for release process"

echo ""
echo "Next steps:"
echo "1. Review and update CHANGELOG.md if needed"
echo "2. Commit any pending changes"
echo "3. Push to main branch to trigger release workflow"
echo "4. Or use manual trigger: gh workflow run release.yml"
