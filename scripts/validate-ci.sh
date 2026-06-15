#!/bin/bash
# Non-interactive pre-push validation matching the required CI checks.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${GREEN}=== Local CI validation ===${NC}"
echo "Project: $PROJECT_ROOT"
echo ""

echo -e "${GREEN}[1/12] Validate CI workflow configuration...${NC}"
cd "$PROJECT_ROOT"
node scripts/check_ci_workflows.mjs --verify-remote
echo -e "${GREEN}CI workflow configuration passed${NC}"
echo ""

echo -e "${GREEN}[2/12] Validate Node.js version and npm lockfile...${NC}"
cd "$PROJECT_ROOT/log-analyzer"
NODE_VERSION=$(node -v)
echo -e "${YELLOW}Node.js: $NODE_VERSION${NC}"
NODE_MAJOR=$(node -p 'Number(process.versions.node.split(".")[0])')
NODE_MINOR=$(node -p 'Number(process.versions.node.split(".")[1])')
if (( NODE_MAJOR < 22 || (NODE_MAJOR == 22 && NODE_MINOR < 12) )); then
  echo -e "${RED}Node.js 22.12.0 or newer is required${NC}"
  exit 1
fi
npm ci --dry-run --ignore-scripts
echo -e "${GREEN}Node.js and npm lockfile passed${NC}"
echo ""

echo -e "${GREEN}[3/12] Check workflow formatting...${NC}"
npx prettier --check \
  "../.github/workflows/*.{yml,yaml}" \
  "../.github/actions/**/*.{yml,yaml}"
echo -e "${GREEN}Workflow formatting passed${NC}"
echo ""

echo -e "${GREEN}[4/12] Run ESLint...${NC}"
npm run lint
echo -e "${GREEN}ESLint passed${NC}"
echo ""

echo -e "${GREEN}[5/12] Run TypeScript type checking...${NC}"
npm run type-check
echo -e "${GREEN}TypeScript passed${NC}"
echo ""

echo -e "${GREEN}[6/12] Run frontend tests and coverage...${NC}"
NODE_ENV=test CI=true npm run test:ci
NODE_ENV=test CI=true npm run test:coverage
echo -e "${GREEN}Frontend tests passed${NC}"
echo ""

echo -e "${GREEN}[7/12] Build frontend...${NC}"
npm run build
echo -e "${GREEN}Frontend build passed${NC}"
echo ""

echo -e "${GREEN}[8/12] Check IPC consistency...${NC}"
cd "$PROJECT_ROOT"
bash scripts/check_ipc_consistency.sh
echo -e "${GREEN}IPC consistency passed${NC}"
echo ""

echo -e "${GREEN}[9/12] Check Rust formatting...${NC}"
cd "$PROJECT_ROOT/log-analyzer/src-tauri"
cargo fmt -- --check
echo -e "${GREEN}Rust formatting passed${NC}"
echo ""

echo -e "${GREEN}[10/12] Run Clippy...${NC}"
cargo clippy --all-features --all-targets -- -D warnings
echo -e "${GREEN}Clippy passed${NC}"
echo ""

echo -e "${GREEN}[11/12] Run Rust workspace tests...${NC}"
RUST_BACKTRACE=1 cargo test --workspace --all-features -- --test-threads=2
echo -e "${GREEN}Rust workspace tests passed${NC}"
echo ""

echo -e "${GREEN}[12/12] Run Tauri debug smoke build...${NC}"
cd "$PROJECT_ROOT/log-analyzer"
npm run tauri build -- --debug --no-bundle
echo -e "${GREEN}Tauri debug smoke build passed${NC}"
echo ""

echo -e "${GREEN}All local CI checks passed.${NC}"
