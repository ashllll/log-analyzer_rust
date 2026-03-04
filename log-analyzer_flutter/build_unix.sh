#!/usr/bin/env bash

# Flutter Desktop Build Script for macOS/Linux
#
# 自动化 Flutter 应用打包流程
# 支持 macOS (Intel 和 Apple Silicon) 和 Linux (x64_64)

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

# 检测平台
detect_platform() {
    case "$(uname -s)" in
        Darwin)
            # 检测是 Intel 还是 Apple Silicon
            if [ "$(uname -m)" = "arm64" ]; then
                echo "macos-arm64"
            else
                echo "macos-x64_64"
            fi
            ;;
        *)
            echo "linux"
            ;;
    esac
}

PLATFORM=$(detect_platform)
CURRENT_ARCH=$(detect_platform)

echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}Flutter Desktop Build Script${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${YELLOW}应用:${NC} $APP_NAME"
echo -e "${YELLOW}版本:${NC} $VERSION"
echo ""
echo -e "${YELLOW}目标平台:${NC} $PLATFORM"

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

# 构建 macOS
build_macos() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}开始构建 macOS 版本${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    # 检查是否是 macOS
    if [ "$PLATFORM" != "macos-x64_64" ] && [ "$PLATFORM" != "macos-arm64" ]; then
        echo -e "${RED}错误: 此脚本仅支持 macOS 平台${NC}"
        return 1
    fi

    cd "$PROJECT_DIR"

    # 运行代码生成
    generate_code

    echo -e "${YELLOW}构建 macOS Release ($CURRENT_ARCH)...${NC}"
    flutter build macos --release

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ macOS 构建成功${NC}"

        # 创建发布目录
        mkdir -p "$RELEASE_DIR"

        # 复制 .app 文件
        APP_BUNDLE="$OUTPUT_DIR/macos/Build/Products/Release/$APP_NAME.app"
        if [ -d "$APP_BUNDLE" ]; then
            cp -r "$APP_BUNDLE" "$RELEASE_DIR/"
            echo -e "${YELLOW}macOS 应用包位置:${NC}"
            echo "  $RELEASE_DIR/$APP_NAME.app"
            echo ""

            # 计算 SHA256 校验和
            if command -v shasum &> /dev/null; then
                echo ""
                echo -e "${YELLOW}生成 SHA256 校验和...${NC}"
                (cd "$RELEASE_DIR" && shasum -a 256 .) > SHA256SUMS.txt
                echo -e "${GREEN}✓ 校验和生成完成${NC}"
            fi

            # 创建 DMG 镜像（如果有 hdiutil）
            if command -v hdiutil &> /dev/null; then
                DMG_FILE="$RELEASE_DIR/$APP_NAME-$VERSION.dmg"
                echo ""
                echo -e "${YELLOW}创建 DMG 镜像...${NC}"
                hdiutil create -srcfolder "$RELEASE_DIR/$APP_NAME.app" \
                    -volname "$APP_NAME" \
                    -ov -format UDRW \
                    "$DMG_FILE" 2>/dev/null || true

                if [ -f "$DMG_FILE" ]; then
                    echo -e "${GREEN}✓ DMG 镜像创建完成${NC}"
                    echo "  $DMG_FILE"
                fi
            fi
        else
            echo -e "${RED}✗ 应用包未找到${NC}"
        return 1
    else
        echo -e "${RED}✗ macOS 构建失败${NC}"
        return 1
    fi
}

# 构建 Linux
build_linux() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}开始构建 Linux 版本${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    # 检查是否是 Linux
    if [ "$PLATFORM" != "linux" ]; then
        echo -e "${RED}错误: 此脚本仅支持 Linux 平台${NC}"
        return 1
    fi

    cd "$PROJECT_DIR"

    # 运行代码生成
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
                echo ""
                echo -e "${YELLOW}创建 Debian 包...${NC}"
                cd "$RELEASE_DIR"
                dpkg-debbuild --root owner-root --inputtype gnome 2>/dev/null || true
                if [ -f "$RELEASE_DIR/$APP_NAME_${VERSION}_amd64.deb" ]; then
                    echo -e "${GREEN}✓ Debian 包创建完成${NC}"
                fi
                cd - > /dev/null
            fi
        else
            echo -e "${RED}✗ 构建产物未找到${NC}"
        fi
    else
        echo -e "${RED}✗ Linux 构建失败${NC}"
        return 1
    fi
}

# 创建 AppImage (Flatpak 格式)
create_appimage() {
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}创建 AppImage${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    if [ "$PLATFORM" != "linux" ]; then
        echo -e "${YELLOW}警告: AppImage 仅支持 Linux${NC}"
    fi

    cd "$PROJECT_DIR"

    # 检查是否安装了 flatpak
    if ! command -v flatpak &> /dev/null; then
        echo -e "${YELLOW}flatpak 未安装，跳过 AppImage 创建${NC}"
        return 1
    fi

    # AppImage 元数据
    APPIMAGE_FILE="$RELEASE_DIR/$APP_NAME-AppImage"

    echo -e "${YELLOW}创建 AppImage 清单...${NC}"
    cat > "$APPIMAGE_FILE.yml" << EOF
app-id: com.joeash.log-analyzer
runtime: org.freedesktop.Platform.Sdk
sdk: stable/22.08

base: org.electronjs.Electron20.BaseApp
base-version: v20.08

command: log_analyzer_flutter

separate-locales: false
rename-icon: log-analyzer
rename-desktop-file: Log Analyzer

finish-args: --命令=run-in-terminal

modules:
  - name: libappindicator-gtk3
    build-args: --libdir=/app/liblog_analyzer_flutter

  - name: libdecor-0
    build-args: --libdir=/app/liblog_analyzer_flutter

  - name: xdg-desktop-portal-gtk3
    build-args: --libdir=/app/liblog_analyzer_flutter

  - name: xdg-desktop-gtk3
    build-args: --libdir=/app/liblog_analyzer_flutter

build-args: --socket=x11
build-args: --share=network

metadata:
  summary: High-performance desktop log analysis tool
  description: |
    A powerful desktop log analysis tool built with Flutter and Rust.
    Features include multi-keyword search, real-time log monitoring,
    keyword highlighting, and workspace management.
  categories:
    - Development
    - Utility
  developer-name: Joe Ash
  developer-url: https://github.com/joeash
  release-notes: Version $VERSION

icon:
  resized:
    width: 128
    height: 128
  files:
    - assets/icons/icon-128.png
    - assets/icons/icon-512.png

modules:
  - name: log_analyzer_flutter
    buildsystem: simple
    build-args: --libdir=/app/liblog_analyzer_flutter
    sources:
      - type: script
        only-locations: true
        commands:
          - mkdir -p /app/bin
          - sh -c "ln -s /app/lib/log_analyzer_flutter/liblog_analyzer_flutter.so /app/bin/log_analyzer_flutter"
EOF

    echo -e "${GREEN}✓ AppImage 清单创建完成${NC}"
    echo "  $APPIMAGE_FILE.yml"

    # 创建图标
    ICON_SRC="$PROJECT_DIR/assets/icons/icon-512.png"
    if [ -f "$ICON_SRC" ]; then
        ICON_DST="$RELEASE_DIR/icons"
        mkdir -p "$ICON_DST"
        cp "$ICON_SRC" "$ICON_DST/icon-512.png"
        cp "$ICON_SRC" "$ICON_DST/icon-128.png"
        echo -e "${GREEN}✓ 图标复制完成${NC}"
    fi
}

# 显示构建摘要
show_build_summary() {
    echo ""
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}构建摘要${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""

    echo -e "${YELLOW}发布文件位置:${NC}"
    echo "  $RELEASE_DIR"
    echo ""

    # 列出文件
    if [ -d "$RELEASE_DIR" ]; then
        echo -e "${YELLOW}发布文件:${NC}"
        ls -lh "$RELEASE_DIR" 2>/dev/null | head -20
    fi
}

# 主菜单
show_menu() {
    echo ""
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}选择构建目标平台${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""
    echo -e "${YELLOW}1)${NC} macOS ($CURRENT_ARCH)"
    echo -e "${YELLOW}2)${NC} Linux"
    echo -e "${YELLOW}3)${NC} 仅代码生成"
    echo -e "${YELLOW}4)${NC} 清理构建目录"
    echo -e "${YELLOW}5)${NC} 创建 AppImage"
    echo -e "${YELLOW}6)${NC} 显示构建摘要"
    echo -e "${YELLOW}7)${NC} 构建所有（当前平台）"
    echo -e "${YELLOW}8)${NC} 构建所有（所有平台）"
    echo -e "${YELLOW}0)${NC} 退出"
    echo ""
    read -p "请输入选项 [0-8]: " choice

    case $choice in
        1)
            build_macos
            ;;
        2)
            build_linux
            ;;
        3)
            generate_code
            echo -e "${GREEN}✓ 代码生成完成${NC}"
            ;;
        4)
            clean_build
            ;;
        5)
            create_appimage
            ;;
        6)
            show_build_summary
            ;;
        7)
            if [ "$PLATFORM" = "macos-x64_64" ] || [ "$PLATFORM" = "macos-arm64" ]; then
                build_macos
            elif [ "$PLATFORM" = "linux" ]; then
                build_linux
            else
                echo -e "${RED}不支持的构建平台: $PLATFORM${NC}"
            fi
            ;;
        8)
            echo -e "${YELLOW}构建所有平台...${NC}"
            # macOS Intel
            PLATFORM_SAVED="$PLATFORM"
            PLATFORM="macos-x64_64"
            build_macos
            # macOS Apple Silicon
            PLATFORM="macos-arm64"
            build_macos
            # Linux
            PLATFORM="linux"
            build_linux
            PLATFORM="$PLATFORM_SAVED"
            show_build_summary
            ;;
        0)
            echo -e "${YELLOW}退出构建脚本${NC}"
            exit 0
            ;;
        *)
            echo -e "${RED}无效选项: $choice${NC}"
            exit 1
            ;;
    esac
}

# 解析命令行参数
auto_build() {
    echo -e "${YELLOW}自动构建模式${NC}"

    # 代码生成
    generate_code

    # 根据平台构建
    case "$PLATFORM" in
        macos-x64_64|macos-arm64)
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
}

# 处理命令行参数
parse_args() {
    while [[ "$#" -gt 0 ]]; do
        case $1 in
            --clean|-c)
                clean_build
                exit 0
                ;;
            --code-only)
                generate_code
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
                generate_code
                if [ "$PLATFORM" = "macos-x64_64" ] || [ "$PLATFORM" = "macos-arm64" ]; then
                    build_macos
                elif [ "$PLATFORM" = "linux" ]; then
                    build_linux
                fi
                exit 0
                ;;
            --appimage)
                create_appimage
                exit 0
                ;;
            -h|--help)
                echo "用法: $0 [选项]"
                echo ""
                echo "选项:"
                echo "  --clean, -c              清理构建目录"
                echo "  --code-only            仅运行代码生成"
                echo "  --macos, -m             构建 macOS 版本"
                echo "  --linux, -l              构建 Linux 版本"
                echo "  --all, -a                构建所有平台"
                echo "  --appimage              创建 AppImage（仅 Linux）"
                echo "  -h, --help               显示此帮助信息"
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
}

# 检查是否在 CI 环境中
if [ -n "$CI" ]; then
    # CI 模式：自动构建
    auto_build
else
    # 本地模式：显示菜单
    show_menu
fi
