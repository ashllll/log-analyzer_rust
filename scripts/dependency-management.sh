#!/bin/bash
# =============================================================================
# Rust 依赖管理工具脚本
# 用法: ./dependency-management.sh [command]
# =============================================================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目目录
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAURI_DIR="$PROJECT_DIR/log-analyzer/src-tauri"

echo_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

echo_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

echo_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查工具是否安装
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo_error "$1 未安装"
        return 1
    fi
    return 0
}

# 安装必要工具
install_tools() {
    echo_info "安装依赖管理工具..."
    
    tools=(
        "cargo-deny"
        "cargo-outdated"
        "cargo-audit"
        "cargo-machete"
        "cargo-tree"
        "cargo-bloat"
    )
    
    for tool in "${tools[@]}"; do
        if ! check_tool "$tool" 2>/dev/null; then
            echo_info "安装 $tool..."
            cargo install "$tool"
        else
            echo_success "$tool 已安装"
        fi
    done
}

# 运行完整审计
run_audit() {
    echo_info "运行依赖审计..."
    cd "$TAURI_DIR"
    
    echo_info "1. 运行 cargo-deny..."
    cargo deny check
    
    echo_info "2. 运行 cargo-audit..."
    cargo audit
    
    echo_success "审计完成！"
}

# 检查过期依赖
check_outdated() {
    echo_info "检查过期依赖..."
    cd "$TAURI_DIR"
    
    cargo outdated -R
}

# 检查重复依赖
check_duplicates() {
    echo_info "检查重复依赖..."
    cd "$TAURI_DIR"
    
    echo_info "依赖重复情况:"
    cargo tree --duplicates
}

# 查找未使用的依赖
find_unused() {
    echo_info "查找未使用的依赖..."
    cd "$TAURI_DIR"
    
    cargo machete
}

# 分析二进制大小
analyze_size() {
    echo_info "分析二进制大小..."
    cd "$TAURI_DIR"
    
    echo_info "构建发布版本..."
    cargo build --release
    
    echo_info "最大的 20 个依赖:"
    cargo bloat --release -n 20
    
    echo_info "按 crate 分析:"
    cargo bloat --release --crates
}

# 更新所有依赖
update_all() {
    echo_info "更新所有依赖..."
    cd "$TAURI_DIR"
    
    echo_info "更新 Cargo.lock..."
    cargo update
    
    echo_success "依赖已更新"
    echo_warning "请运行测试确保一切正常: cargo test --all-features"
}

# 更新特定依赖
update_single() {
    local crate="$1"
    if [ -z "$crate" ]; then
        echo_error "请指定 crate 名称"
        echo "用法: $0 update-single <crate-name>"
        exit 1
    fi
    
    echo_info "更新 $crate..."
    cd "$TAURI_DIR"
    
    cargo update -p "$crate"
    echo_success "$crate 已更新"
}

# 显示依赖树
show_tree() {
    local crate="$1"
    cd "$TAURI_DIR"
    
    if [ -z "$crate" ]; then
        echo_info "显示完整依赖树..."
        cargo tree
    else
        echo_info "显示 $crate 的依赖树..."
        cargo tree -i "$crate"
    fi
}

# 显示帮助
show_help() {
    cat << EOF
Rust 依赖管理工具

用法: $0 [command] [options]

命令:
    install-tools       安装所有依赖管理工具
    audit               运行完整依赖审计 (cargo-deny + cargo-audit)
    outdated            检查过期依赖
    duplicates          检查重复依赖
    unused              查找未使用的依赖
    size                分析二进制大小
    update              更新所有依赖
    update-single <c>   更新特定依赖
    tree [crate]        显示依赖树
    help                显示此帮助

示例:
    $0 install-tools           # 安装工具
    $0 audit                   # 运行审计
    $0 tree sqlx               # 查看 sqlx 依赖树
    $0 update-single serde     # 更新 serde

EOF
}

# 主入口
main() {
    case "${1:-help}" in
        install-tools)
            install_tools
            ;;
        audit)
            run_audit
            ;;
        outdated)
            check_outdated
            ;;
        duplicates)
            check_duplicates
            ;;
        unused)
            find_unused
            ;;
        size)
            analyze_size
            ;;
        update)
            update_all
            ;;
        update-single)
            update_single "$2"
            ;;
        tree)
            show_tree "$2"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo_error "未知命令: $1"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
