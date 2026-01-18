# 本地 CI 验证脚本 (PowerShell 版本)
# 在推送前运行此脚本，确保本地通过所有 CI 检查

$ErrorActionPreference = "Stop"

# 项目根目录
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location "$ProjectRoot\log-analyzer"

Write-Host "=== 本地 CI 验证脚本 ===" -ForegroundColor Green
Write-Host "项目路径: $ProjectRoot"
Write-Host ""

# 检查 Node 版本
$NodeVersion = node -v
Write-Host "Node 版本: $NodeVersion" -ForegroundColor Yellow
if (-not ($NodeVersion -match "v22")) {
  Write-Host "⚠️  警告: CI 使用 Node 22，本地使用 $NodeVersion" -ForegroundColor Red
}
Write-Host ""

# ============================================================================
# 1. 前端 Lint 检查
# ============================================================================
Write-Host "[1/6] 运行 ESLint..." -ForegroundColor Green
npm run lint
Write-Host "✅ ESLint 通过" -ForegroundColor Green
Write-Host ""

# ============================================================================
# 2. TypeScript 类型检查
# ============================================================================
Write-Host "[2/6] 运行 TypeScript 类型检查..." -ForegroundColor Green
npm run type-check
Write-Host "✅ 类型检查通过" -ForegroundColor Green
Write-Host ""

# ============================================================================
# 3. 前端测试
# ============================================================================
Write-Host "[3/6] 运行前端测试..." -ForegroundColor Green
$env:NODE_ENV = "test"
npm test -- --testPathIgnorePatterns=e2e --verbose
Write-Host "✅ 前端测试通过" -ForegroundColor Green
Write-Host ""

# ============================================================================
# 4. 前端构建
# ============================================================================
Write-Host "[4/6] 构建前端..." -ForegroundColor Green
npm run build
Write-Host "✅ 前端构建成功" -ForegroundColor Green
Write-Host ""

# ============================================================================
# 5. Rust 格式检查
# ============================================================================
Write-Host "[5/6] 检查 Rust 代码格式..." -ForegroundColor Green
Set-Location src-tauri
cargo fmt -- --check
Write-Host "✅ 代码格式检查通过" -ForegroundColor Green
Write-Host ""

# ============================================================================
# 6. Rust Clippy 检查
# ============================================================================
Write-Host "[6/6] 运行 Clippy..." -ForegroundColor Green
cargo clippy --all-features --all-targets -- -D warnings
Write-Host "✅ Clippy 检查通过" -ForegroundColor Green
Write-Host ""

# ============================================================================
# Rust 测试 (可选)
# ============================================================================
$RunTests = Read-Host "是否运行 Rust 测试? (y/N)"
if ($RunTests -eq "y" -or $RunTests -eq "Y") {
  Write-Host "[7/7] 运行 Rust 测试..." -ForegroundColor Green
  cargo test --all-features --verbose
  Write-Host "✅ Rust 测试通过" -ForegroundColor Green
}

Write-Host ""
Write-Host "===========================================" -ForegroundColor Green
Write-Host "✅ 本地 CI 验证全部通过！" -ForegroundColor Green
Write-Host "===========================================" -ForegroundColor Green
Write-Host ""
Write-Host "可以安全地推送到远程仓库了！"
