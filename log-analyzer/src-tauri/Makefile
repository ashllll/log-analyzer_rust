# =============================================================================
# Rust 项目 Makefile - 常用命令快捷方式
# =============================================================================

.PHONY: all help build test check fmt lint audit clean

# 默认目标
all: check

# 帮助信息
help:
	@echo "Rust 项目常用命令"
	@echo ""
	@echo "构建命令:"
	@echo "  make build          构建开发版本"
	@echo "  make build-release  构建发布版本"
	@echo "  make build-ffi      构建 FFI 库"
	@echo ""
	@echo "测试命令:"
	@echo "  make test           运行所有测试"
	@echo "  make test-lib       运行单元测试"
	@echo "  make test-integration 运行集成测试"
	@echo ""
	@echo "代码质量:"
	@echo "  make check          运行 cargo check"
	@echo "  make fmt            格式化代码"
	@echo "  make fmt-check      检查代码格式"
	@echo "  make lint           运行 clippy"
	@echo "  make lint-fix       自动修复 clippy 警告"
	@echo ""
	@echo "依赖管理:"
	@echo "  make audit          运行 cargo-deny 审计"
	@echo "  make outdated       检查过期依赖"
	@echo "  make duplicates     检查重复依赖"
	@echo "  make unused         查找未使用的依赖"
	@echo "  make update         更新所有依赖"
	@echo ""
	@echo "其他:"
	@echo "  make clean          清理构建产物"
	@echo "  make bench          运行基准测试"
	@echo "  make doc            生成文档"

# 构建命令
build:
	cargo build

build-release:
	cargo build --release

build-ffi:
	cargo build --release --features ffi

build-standalone:
	cargo build --release --features standalone

# 测试命令
test:
	cargo test --all-features

test-lib:
	cargo test --lib --all-features

test-integration:
	cargo test --test '*' --all-features

test-coverage:
	cargo tarpaulin --all-features --out Html

# 代码质量
check:
	cargo check --all-features

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

lint:
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	cargo clippy --all-features --all-targets --fix -- -D warnings

# 依赖管理
audit:
	cargo deny check

outdated:
	cargo outdated -R

duplicates:
	cargo tree --duplicates

unused:
	cargo machete

update:
	cargo update

update-precise:
	@echo "更新特定依赖示例: cargo update -p <crate>"

tree:
	cargo tree

# 其他
clean:
	cargo clean

bench:
	cargo bench

doc:
	cargo doc --all-features --no-deps --open

# 发布准备
release-check:
	@echo "=== 发布前检查 ==="
	cargo fmt -- --check
	cargo clippy --all-features --all-targets -- -D warnings
	cargo test --all-features
	cargo deny check
	cargo audit
	@echo "=== 检查完成 ==="

# CI 完整验证 (本地模拟 CI)
ci-check:
	@echo "=== CI 完整验证 ==="
	@echo "1. 格式化检查..."
	cargo fmt -- --check
	@echo "2. Clippy 检查..."
	cargo clippy --all-features --all-targets -- -D warnings
	@echo "3. 单元测试..."
	cargo test --lib --all-features
	@echo "4. 集成测试..."
	cargo test --test '*' --all-features
	@echo "5. 依赖审计..."
	cargo deny check
	@echo "6. 安全检查..."
	cargo audit
	@echo "=== 所有检查通过 ==="
