#!/bin/bash

# Log Analyzer 快速发布脚本
# 用法: ./release.sh <version>
# 示例: ./release.sh 1.0.0

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "❌ 错误: 请提供版本号"
    echo "用法: ./release.sh <version>"
    echo "示例: ./release.sh 1.0.0"
    exit 1
fi

# 移除 'v' 前缀（如果有）
VERSION=${VERSION#v}

echo "🚀 开始发布流程 - 版本 v${VERSION}"
echo ""

# 1. 检查工作目录是否干净
echo "📋 检查 Git 状态..."
if [ -n "$(git status --porcelain)" ]; then
    echo "❌ 错误: 工作目录不干净，请先提交或暂存更改"
    git status --short
    exit 1
fi
echo "✅ 工作目录干净"
echo ""

# 2. 更新 package.json 版本
echo "📝 更新 package.json 版本..."
cd log-analyzer
npm version $VERSION --no-git-tag-version
cd ..
echo "✅ package.json 已更新"
echo ""

# 3. 更新 Cargo.toml 版本
echo "📝 更新 Cargo.toml 版本..."
CARGO_FILE="log-analyzer/src-tauri/Cargo.toml"
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" $CARGO_FILE
else
    # Linux
    sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" $CARGO_FILE
fi
echo "✅ Cargo.toml 已更新"
echo ""

# 4. 运行测试
echo "🧪 运行测试..."
cd log-analyzer/src-tauri
cargo test --all-features
cd ../..
echo "✅ 测试通过"
echo ""

# 5. 提交版本更改
echo "💾 提交版本更改..."
git add log-analyzer/package.json log-analyzer/package-lock.json log-analyzer/src-tauri/Cargo.toml log-analyzer/src-tauri/Cargo.lock
git commit -m "chore: bump version to v${VERSION}"
echo "✅ 版本更改已提交"
echo ""

# 6. 创建标签
echo "🏷️  创建 Git 标签..."
git tag -a "v${VERSION}" -m "Release v${VERSION}"
echo "✅ 标签 v${VERSION} 已创建"
echo ""

# 7. 推送到远程
echo "📤 推送到 GitHub..."
git push origin main
git push origin "v${VERSION}"
echo "✅ 已推送到 GitHub"
echo ""

echo "🎉 发布流程完成！"
echo ""
echo "📋 下一步操作："
echo "  1. 访问 https://github.com/ashllll/log-analyzer_rust/actions"
echo "  2. 查看 Release 工作流的构建进度"
echo "  3. 等待构建完成（通常需要 10-20 分钟）"
echo "  4. 访问 https://github.com/ashllll/log-analyzer_rust/releases 查看发布"
echo ""
