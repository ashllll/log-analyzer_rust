#!/usr/bin/env bash
# IPC 一致性检查脚本（CI 入口）
# 调用 Node.js 脚本执行实际检查

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "========================================"
echo "IPC Consistency Check"
echo "========================================"
echo ""

# 检查 Node.js 是否可用
if ! command -v node >/dev/null 2>&1; then
    echo "ERROR: Node.js is required but not installed"
    exit 1
fi

# 运行 Node.js 检查脚本
node "${SCRIPT_DIR}/check_ipc_consistency.cjs"
