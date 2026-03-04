#!/usr/bin/env bash

# Flutter Desktop Build Script
#
# 自动化 Flutter 应用打包流程
# 支持 Windows、macOS、Linux 三个平台

set -e  # 遇到错误时退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/log-analyzer_flutter"

# 输出目录
OUTPUT_DIR="$PROJECT_DIR/build"
RELEASE_DIR="$PROJECT_DIR/release"

# 应用信息
APP_NAME="Log Analyzer"
VERSION=$(grep -oP 'version: (.+)' "$PROJECT_DIR/pubspec.yaml" | cut -d' ' -f2)

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}  Flutter Desktop Build Script${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${YELLOW}应用:${NC} $APP_NAME"
echo -e "${YELLOW}版本:${NC} $VERSION"
echo ""

# 检测构建平台
detect_platform() {
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            echo "windows"
            ;;
        Darwin)
            echo "macos"
            ;;
        *)
            echo "linux"
            ;;
    esac
}

PLATFORM=$(detect_platform)
echo -e "${YELLOW}检测平台:${NC} $PLATFORM"

# 清理函数
clean_build() {
    echo -e "${YELLOW}清理构建目录...${NC}"
    rm -rf "$OUTPUT_DIR"
    rm -rf "$RELEASE_DIR"
    echo -e "${GREEN}✓ 清理完成${NC}"
}

# 运行代码生成
generate_code() {
    echo -e "${YELLOW}运行代码生成...${NC}"
    cd "$PROJECT_DIR"
    dart run build_runner build --delete-conflicting-outputs
    echo -e "${GREEN}✓ 代码生成完成${NC}"
}

# 构建前检查
check_prerequisites() {
    echo -e "${YELLOW}检查构建前提条件...${NC}"

    # 检查 Flutter SDK
    if ! command -v flutter &> /dev/null; then
        echo -e "${RED}✗ Flutter 未安装或不在 PATH 中${NC}"
        echo "请安装 Flutter SDK: https://flutter.dev/docs/get-started/install"
        exit 1
    fi

    # 检查 Dart
    if ! command -v dart &> /dev/null; then
        echo -e "${RED}✗ Dart 未安装${NC}"
        exit 1
    fi

    # 检查项目依赖
    if [ ! -d "$PROJECT_DIR/.dart_tool" ]; then
        echo -e "${YELLOW}运行 flutter pub get...${NC}"
        cd "$PROJECT_DIR"
        flutter pub get
    fi

    echo -e "${GREEN}✓ 前提条件检查完成${NC}"
}

# 构建 Windows
build_windows() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}开始构建 Windows 版本${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    check_prerequisites

    # 检查环境
    if [ "$PLATFORM" != "windows" ] && [ "$PLATFORM" != "linux" ]; then
        echo -e "${YELLOW}警告: 跨平台构建 Windows 需要 Windows 环境${NC}"
    fi

    cd "$PROJECT_DIR"

    # 运行代码生成
    generate_code

    echo -e "${YELLOW}构建 Windows Release...${NC}"
    flutter build windows --release

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Windows 构建成功${NC}"

        # 创建发布目录
        mkdir -p "$RELEASE_DIR"

        # 复制构建产物
        cp -r "$OUTPUT_DIR/windows/runner/Release/"* "$RELEASE_DIR/" 2>/dev/null || true

        echo -e "${YELLOW}Windows 发布包位置:${NC}"
        echo "  $RELEASE_DIR/"
        echo ""
        echo -e "${GREEN}可执行文件:${NC}"
        ls -lh "$RELEASE_DIR/log_analyzer_flutter.exe" 2>/dev/null || echo "  文件未找到"

        # 生成 SHA256 校验和
        if command -v sha256sum &> /dev/null; then
            echo ""
            echo -e "${YELLOW}生成校验和...${NC}"
            sha256sum "$RELEASE_DIR"/* > "$RELEASE_DIR/SHA256SUMS.txt" 2>/dev/null
            echo -e "${GREEN}✓ 校验和生成完成${NC}"
        fi
    else
        echo -e "${RED}✗ Windows 构建失败${NC}"
        exit 1
    fi
}

# 构建 macOS
build_macos() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}开始构建 macOS 版本${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    if [ "$PLATFORM" != "macos" ]; then
        echo -e "${YELLOW}警告: 跨平台构建 macOS 需要 macOS 环境${NC}"
    fi

    check_prerequisites
    cd "$PROJECT_DIR"
    generate_code

    echo -e "${YELLOW}构建 macOS Release...${NC}"
    flutter build macos --release

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ macOS 构建成功${NC}"

        # 创建发布目录
        mkdir -p "$RELEASE_DIR"

        # 复制 .app 文件
        APP_BUNDLE="$OUTPUT_DIR/macos/Build/Products/Release/log_analyzer_flutter.app"
        if [ -d "$APP_BUNDLE" ]; then
            cp -r "$APP_BUNDLE" "$RELEASE_DIR/"
            echo -e "${YELLOW}macOS 应用包位置:${NC}"
            echo "  $RELEASE_DIR/log_analyzer_flutter.app"
            echo ""

            # 创建 DMG 镜像（如果有 hdiutil）
            if command -v hdiutil &> /dev/null; then
                DMG_FILE="$RELEASE_DIR/log-analyzer_flutter-$VERSION.dmg"
                echo -e "${YELLOW}创建 DMG 镜像...${NC}"
                hdiutil create -srcfolder "$RELEASE_DIR/log_analyzer_flutter.app" -volname "Log Analyzer" -ov -format UDRW "$DMG_FILE"
                echo -e "${GREEN}✓ DMG 镜像创建完成${NC}"
                echo "  $DMG_FILE"
            fi
        else
            echo -e "${RED}✗ 应用包未找到${NC}"
        fi
    else
        echo -e "${RED}✗ macOS 构建失败${NC}"
        exit 1
    fi
}

# 构建 Linux
build_linux() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}开始构建 Linux 版本${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    check_prerequisites
    cd "$PROJECT_DIR"
    generate_code

    echo -e "${YELLOW}构建 Linux Release...${NC}"
    flutter build linux --release

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Linux 构建成功${NC}"

        # 创建发布目录
        mkdir -p "$RELEASE_DIR"

        # 复制构建产物
        BUILD_BUNDLE="$OUTPUT_DIR/linux/x64/release/bundle"
        if [ -d "$BUILD_BUNDLE" ]; then
            cp -r "$BUILD_BUNDLE" "$RELEASE_DIR/"
            echo -e "${YELLOW}Linux 发布包位置:${NC}"
            echo "  $RELEASE_DIR/"
            echo ""
            echo -e "${GREEN}可执行文件:${NC}"
            ls -lh "$RELEASE_DIR/log_analyzer_flutter" 2>/dev/null || echo "  文件未找到"

            # 创建 Debian 包（如果有 dpkg-debbuild）
            if command -v dpkg-debbuild &> /dev/null; then
                echo -e "${YELLOW}创建 Debian 包...${NC}"
                cd "$RELEASE_DIR"
                dpkg-debbuild --root owner-root --inputtype gnome 2>/dev/null || true
                echo -e "${GREEN}✓ Debian 包创建完成${NC}"
            fi
        else
            echo -e "${RED}✗ Linux 构建产物未找到${NC}"
        fi
    else
        echo -e "${RED}✗ Linux 构建失败${NC}"
        exit 1
    fi
}

# 安装器构建
build_installer() {
    echo ""
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}构建安装器${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    # 注意：需要 Inno Setup (Windows) 或其他安装器工具
    echo -e "${YELLOW}安装器构建需要额外工具:${NC}"
    echo "  Windows: Inno Setup (https://jrsoftware.org/inno-setup)"
    echo "  macOS: create-dmg 或 Packages (built-in)"
    echo "  Linux: dpkg-debbuild, AppImage, 或 Flatpak"
    echo ""
    echo -e "${YELLOW}此脚本仅构建应用本身${NC}"
    echo "  如需安装器，请手动运行对应工具"
}

# 主菜单
show_menu() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}选择构建目标平台${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""
    echo -e "${YELLOW}1)${NC} Windows"
    echo -e "${YELLOW}2)${NC} macOS"
    echo -e "${YELLOW}3)${NC} Linux"
    echo -e "${YELLOW}4)${NC} 全平台（当前平台 + 其他）"
    echo -e "${YELLOW}5)${NC} 仅代码生成"
    echo -e "${YELLOW}6)${NC} 清理构建目录"
    echo -e "${YELLOW}0)${NC} 退出"
    echo ""
    read -p "请输入选项 [1-6]: " choice

    case $choice in
        1)
            build_windows
            ;;
        2)
            build_macos
            ;;
        3)
            build_linux
            ;;
        4)
            echo -e "${YELLOW}构建全平台...${NC}"
            if [ "$PLATFORM" != "linux" ]; then
                build_windows
            fi
            if [ "$PLATFORM" != "windows" ]; then
                build_macos
            fi
            if [ "$PLATFORM" != "macos" ]; then
                build_linux
            fi
            ;;
        5)
            check_prerequisites
            generate_code
            echo -e "${GREEN}✓ 代码生成完成，跳过构建${NC}"
            ;;
        6)
            clean_build
            echo -e "${GREEN}✓ 构建目录已清理${NC}"
            ;;
        0)
            echo -e "${YELLOW}退出${NC}"
            exit 0
            ;;
        *)
            echo -e "${RED}无效选项: $choice${NC}"
            exit 1
            ;;
    esac
}

# 自动模式（CI/CD）
auto_build() {
    echo -e "${YELLOW}自动构建模式${NC}"

    # CI 环境构建所有平台
    if [ "$CI" = "true" ]; then
        build_windows
        build_macos
        build_linux
    else
        # 本地环境仅构建当前平台
        case "$PLATFORM" in
            windows)
                build_windows
                ;;
            macos)
                build_macos
                ;;
            linux)
                build_linux
                ;;
            *)
                echo -e "${RED}未知平台: $PLATFORM${NC}"
                exit 1
                ;;
        esac
    fi
}

# 解析命令行参数
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --clean|-c)
            clean_build
            exit 0
            ;;
        --code-only)
            check_prerequisites
            generate_code
            exit 0
            ;;
        --windows|-w)
            build_windows
            exit 0
            ;;
        --macos|-m)
            build_macos
            exit 0
            ;;
        --linux|-l)
            build_linux
            exit 0
            ;;
        --all|-a)
            auto_build
            exit 0
            ;;
        --auto)
            CI=true auto_build
            exit 0
            ;;
        -h|--help)
            echo "用法: $0 [选项]"
            echo ""
            echo "选项:"
            echo "  --clean, -c              清理构建目录"
            echo "  --code-only            仅运行代码生成"
            echo "  --windows, -w          构建 Windows 版本"
            echo "  --macos, -m            构建 macOS 版本"
            echo "  --linux, -l             构建 Linux 版本"
            echo "  --all, -a               构建所有平台"
            echo "  --auto                  自动构建模式（CI）"
            echo "  --help, -h              显示此帮助信息"
            echo ""
            echo "无选项时显示交互式菜单"
            exit 0
            ;;
        *)
            echo -e "${RED}未知选项: $1${NC}"
            echo "使用 --help 查看帮助"
            exit 1
            ;;
    esac
    shift
done

# 默认显示菜单
show_menu
